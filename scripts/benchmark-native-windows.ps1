[CmdletBinding()]
param(
    [string]$HostPath,
    [string]$OutputPath,
    [ValidateRange(1, 1000)]
    [int]$Samples = 200,
    [ValidateRange(1, 120)]
    [int]$StartupObservationSeconds = 5,
    [ValidateRange(1, 120)]
    [int]$IdleSeconds = 15,
    [double]$MaximumIdleCorePercent = 2.0,
    [double]$MaximumWarmP95Milliseconds = 50.0
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$RepositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
if ([string]::IsNullOrWhiteSpace($HostPath)) {
    $HostPath = Join-Path $RepositoryRoot "target/release/nanika-host.exe"
}
$HostPath = [IO.Path]::GetFullPath($HostPath)
if (-not (Test-Path -LiteralPath $HostPath -PathType Leaf)) {
    throw "Nanika host not found: $HostPath"
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $Timestamp = [DateTime]::UtcNow.ToString("yyyyMMddTHHmmssZ")
    $OutputPath = Join-Path $RepositoryRoot "target/performance/native-windows-$Timestamp.json"
}
$OutputPath = [IO.Path]::GetFullPath($OutputPath)

if (Get-Process -Name "nanika-host" -ErrorAction SilentlyContinue) {
    throw "Quit the running Nanika host before starting the native benchmark."
}

$ExistingExtensionProcessIds = @(
    Get-Process -Name "nanika-extension-host" -ErrorAction SilentlyContinue |
        ForEach-Object { $_.Id }
)

if ($null -eq ("NanikaNativeBenchmark" -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class NanikaNativeBenchmark
{
    [StructLayout(LayoutKind.Sequential)]
    public struct Rect
    {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    private delegate bool EnumWindowsProc(IntPtr window, IntPtr parameter);

    [DllImport("user32.dll")]
    private static extern bool EnumWindows(EnumWindowsProc callback, IntPtr parameter);

    [DllImport("user32.dll")]
    private static extern bool EnumChildWindows(IntPtr parent, EnumWindowsProc callback, IntPtr parameter);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetWindowText(IntPtr window, StringBuilder text, int maximum);

    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr window);

    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr window, out Rect rectangle);

    [DllImport("user32.dll")]
    private static extern void keybd_event(byte virtualKey, byte scanCode, uint flags, UIntPtr extraInfo);

    [DllImport("user32.dll")]
    private static extern bool PostMessage(IntPtr window, uint message, IntPtr word, IntPtr data);

    private const byte Control = 0x11;
    private const byte Space = 0x20;
    private const byte Escape = 0x1B;
    private const uint KeyUp = 0x0002;
    private const uint KeyDownMessage = 0x0100;
    private const uint KeyUpMessage = 0x0101;

    public static IntPtr FindWindow(uint processId, string title)
    {
        IntPtr result = IntPtr.Zero;
        EnumWindows((window, _) =>
        {
            GetWindowThreadProcessId(window, out uint owner);
            if (owner != processId)
            {
                return true;
            }
            StringBuilder text = new StringBuilder(256);
            GetWindowText(window, text, text.Capacity);
            if (String.Equals(text.ToString(), title, StringComparison.Ordinal))
            {
                result = window;
                return false;
            }
            return true;
        }, IntPtr.Zero);
        return result;
    }

    public static string GetChildText(IntPtr parent)
    {
        StringBuilder result = new StringBuilder();
        EnumChildWindows(parent, (window, _) =>
        {
            StringBuilder text = new StringBuilder(1024);
            GetWindowText(window, text, text.Capacity);
            if (text.Length > 0)
            {
                if (result.Length > 0)
                {
                    result.Append(" | ");
                }
                result.Append(text);
            }
            return true;
        }, IntPtr.Zero);
        return result.ToString();
    }

    public static void SendSummon()
    {
        keybd_event(Control, 0, 0, UIntPtr.Zero);
        keybd_event(Space, 0, 0, UIntPtr.Zero);
        keybd_event(Space, 0, KeyUp, UIntPtr.Zero);
        keybd_event(Control, 0, KeyUp, UIntPtr.Zero);
    }

    public static void SendEscape(IntPtr window)
    {
        PostMessage(window, KeyDownMessage, (IntPtr)Escape, (IntPtr)0x00010001);
        PostMessage(window, KeyUpMessage, (IntPtr)Escape, unchecked((IntPtr)(long)0xC0010001));
    }
}
'@
}

function Wait-NanikaVisibility {
    param(
        [Diagnostics.Process]$Process,
        [bool]$Visible,
        [int]$TimeoutMilliseconds = 2000
    )

    $Timer = [Diagnostics.Stopwatch]::StartNew()
    while ($Timer.ElapsedMilliseconds -lt $TimeoutMilliseconds) {
        $Window = [NanikaNativeBenchmark]::FindWindow([uint32]$Process.Id, "Nanika")
        if ($Window -ne [IntPtr]::Zero -and
            [NanikaNativeBenchmark]::IsWindowVisible($Window) -eq $Visible) {
            return [PSCustomObject]@{
                Window = $Window
                ElapsedMilliseconds = $Timer.Elapsed.TotalMilliseconds
            }
        }
        Start-Sleep -Milliseconds 1
    }
    throw "Nanika window did not reach visible=$Visible within $TimeoutMilliseconds ms."
}

function Get-Percentile {
    param(
        [double[]]$Values,
        [double]$Percentile
    )

    $Sorted = @($Values | Sort-Object)
    $Index = [Math]::Ceiling(($Percentile / 100.0) * $Sorted.Count) - 1
    return [double]$Sorted[[Math]::Max(0, $Index)]
}

function Hide-Nanika {
    param([Diagnostics.Process]$Process)

    $Window = [NanikaNativeBenchmark]::FindWindow([uint32]$Process.Id, "Nanika")
    if ($Window -eq [IntPtr]::Zero) {
        throw "Nanika window was not found before dismissal."
    }
    [NanikaNativeBenchmark]::SendEscape($Window)
    Wait-NanikaVisibility -Process $Process -Visible $false | Out-Null
}

$HostProcess = $null
$BenchmarkFailed = $false
Push-Location $RepositoryRoot
try {
    $StartInfo = [Diagnostics.ProcessStartInfo]::new($HostPath)
    $StartInfo.WorkingDirectory = [IO.Path]::GetDirectoryName($HostPath)
    $StartInfo.UseShellExecute = $false
    $StartInfo.ArgumentList.Add("--background")
    $HostProcess = [Diagnostics.Process]::Start($StartInfo)
    if ($null -eq $HostProcess) {
        throw "Nanika host did not start."
    }

    $StartupTimer = [Diagnostics.Stopwatch]::StartNew()
    $StartupVisibleSamples = 0
    $Window = [IntPtr]::Zero
    while ($StartupTimer.Elapsed.TotalSeconds -lt $StartupObservationSeconds) {
        if ($HostProcess.HasExited) {
            throw "Nanika host exited during startup with code $($HostProcess.ExitCode)."
        }
        $Window = [NanikaNativeBenchmark]::FindWindow([uint32]$HostProcess.Id, "Nanika")
        if ($Window -ne [IntPtr]::Zero -and [NanikaNativeBenchmark]::IsWindowVisible($Window)) {
            $StartupVisibleSamples++
        }
        Start-Sleep -Milliseconds 1
    }
    if ($Window -eq [IntPtr]::Zero) {
        throw "Nanika did not create its hidden native window."
    }
    if ($StartupVisibleSamples -ne 0) {
        $StillVisible = [NanikaNativeBenchmark]::IsWindowVisible($Window)
        $IsForeground = [NanikaNativeBenchmark]::GetForegroundWindow() -eq $Window
        $Rectangle = [NanikaNativeBenchmark+Rect]::new()
        [NanikaNativeBenchmark]::GetWindowRect($Window, [ref]$Rectangle) | Out-Null
        $WindowSize = "$($Rectangle.Right - $Rectangle.Left)x$($Rectangle.Bottom - $Rectangle.Top)"
        $ChildText = [NanikaNativeBenchmark]::GetChildText($Window)
        throw "Nanika exposed its native window during startup ($StartupVisibleSamples visible samples, visible after observation: $StillVisible, foreground: $IsForeground, size: $WindowSize, child text: $ChildText)."
    }

    $HostProcess.Refresh()
    $IdleCpuStart = $HostProcess.TotalProcessorTime.TotalSeconds
    $IdleTimer = [Diagnostics.Stopwatch]::StartNew()
    Start-Sleep -Seconds $IdleSeconds
    $IdleTimer.Stop()
    $HostProcess.Refresh()
    $IdleCpuSeconds = $HostProcess.TotalProcessorTime.TotalSeconds - $IdleCpuStart
    $IdleCorePercent = ($IdleCpuSeconds / $IdleTimer.Elapsed.TotalSeconds) * 100.0

    [NanikaNativeBenchmark]::SendSummon()
    Wait-NanikaVisibility -Process $HostProcess -Visible $true | Out-Null
    Hide-Nanika -Process $HostProcess

    $ActivationSamples = [Collections.Generic.List[double]]::new()
    for ($Index = 0; $Index -lt $Samples; $Index++) {
        $Timer = [Diagnostics.Stopwatch]::StartNew()
        [NanikaNativeBenchmark]::SendSummon()
        Wait-NanikaVisibility -Process $HostProcess -Visible $true | Out-Null
        $Timer.Stop()
        $ActivationSamples.Add($Timer.Elapsed.TotalMilliseconds)
        Hide-Nanika -Process $HostProcess
    }

    $Commit = (& git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "Could not resolve the Git commit."
    }
    $Dirty = [bool](& git status --porcelain)
    $ExecutableHash = (Get-FileHash -LiteralPath $HostPath -Algorithm SHA256).Hash.ToLowerInvariant()
    $LockfileHash = (Get-FileHash -LiteralPath (Join-Path $RepositoryRoot "Cargo.lock") -Algorithm SHA256).Hash.ToLowerInvariant()
    $Computer = Get-CimInstance Win32_ComputerSystem
    $OperatingSystem = Get-CimInstance Win32_OperatingSystem
    $P50 = Get-Percentile -Values $ActivationSamples.ToArray() -Percentile 50
    $P95 = Get-Percentile -Values $ActivationSamples.ToArray() -Percentile 95
    $P99 = Get-Percentile -Values $ActivationSamples.ToArray() -Percentile 99

    $Report = [ordered]@{
        schema_version = 1
        generated_at_utc = [DateTime]::UtcNow.ToString("O")
        commit = $Commit
        dirty_worktree = $Dirty
        executable = $HostPath
        executable_sha256 = $ExecutableHash
        cargo_lock_sha256 = $LockfileHash
        machine = [ordered]@{
            name = $env:COMPUTERNAME
            os = $OperatingSystem.Caption
            os_version = $OperatingSystem.Version
            logical_processors = [Environment]::ProcessorCount
            memory_bytes = [uint64]$Computer.TotalPhysicalMemory
        }
        parameters = [ordered]@{
            startup_observation_seconds = $StartupObservationSeconds
            idle_seconds = $IdleSeconds
            warm_activation_samples = $Samples
        }
        metrics = [ordered]@{
            startup_visible_samples = $StartupVisibleSamples
            hidden_idle_cpu_seconds = $IdleCpuSeconds
            hidden_idle_core_percent = $IdleCorePercent
            working_set_bytes = [uint64]$HostProcess.WorkingSet64
            thread_count = $HostProcess.Threads.Count
            warm_activation_ms = [ordered]@{
                p50 = $P50
                p95 = $P95
                p99 = $P99
                maximum = [double]($ActivationSamples | Measure-Object -Maximum).Maximum
                samples = $ActivationSamples.ToArray()
            }
        }
        thresholds = [ordered]@{
            maximum_idle_core_percent = $MaximumIdleCorePercent
            maximum_warm_p95_ms = $MaximumWarmP95Milliseconds
        }
        passed = $StartupVisibleSamples -eq 0 -and
            $IdleCorePercent -le $MaximumIdleCorePercent -and
            $P95 -le $MaximumWarmP95Milliseconds
    }

    $OutputDirectory = [IO.Path]::GetDirectoryName($OutputPath)
    [IO.Directory]::CreateDirectory($OutputDirectory) | Out-Null
    $Report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $OutputPath -Encoding utf8
    $Report.metrics | Format-List
    Write-Output "Native benchmark report: $OutputPath"

    if (-not $Report.passed) {
        $BenchmarkFailed = $true
        throw "Native performance thresholds were not met."
    }
}
finally {
    if ($null -ne $HostProcess -and -not $HostProcess.HasExited) {
        Stop-Process -Id $HostProcess.Id -Force -ErrorAction SilentlyContinue
        $HostProcess.WaitForExit(5000) | Out-Null
    }
    Start-Sleep -Milliseconds 250
    $NewExtensionProcesses = @(
        Get-Process -Name "nanika-extension-host" -ErrorAction SilentlyContinue |
            Where-Object { $_.Id -notin $ExistingExtensionProcessIds }
    )
    if ($NewExtensionProcesses.Count -gt 0) {
        $NewExtensionProcesses | Stop-Process -Force -ErrorAction SilentlyContinue
        if (-not $BenchmarkFailed) {
            throw "Native benchmark left extension host processes behind."
        }
    }
    Pop-Location
}
