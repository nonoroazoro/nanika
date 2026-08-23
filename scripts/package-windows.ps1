[CmdletBinding()]
param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [string]$Version,
    [string]$CertificateThumbprint,
    [string]$TimestampUrl = "http://timestamp.digicert.com",
    [string]$SignTool
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$RepositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
if ([string]::IsNullOrWhiteSpace($Version)) {
    $VersionMatch = Select-String -Path (Join-Path $RepositoryRoot "Cargo.toml") -Pattern '^version = "([^"]+)"$' | Select-Object -First 1
    if ($null -eq $VersionMatch) {
        throw "Workspace version was not found."
    }
    $Version = $VersionMatch.Matches[0].Groups[1].Value
}
if ($Version -notmatch '^[0-9A-Za-z.+-]+$') {
    throw "Version contains characters that are unsafe for an artifact name."
}
if ($Target -ne "x86_64-pc-windows-msvc") {
    throw "Milestone 8 supports only the x86_64 Windows release target."
}

$Packages = @(
    "nanika-host",
    "nanika-cli",
    "nanika-extension-application",
    "nanika-extension-command",
    "nanika-extension-script",
    "nanika-extension-calculator",
    "nanika-extension-clipboard"
)
$DistRoot = [IO.Path]::GetFullPath((Join-Path $RepositoryRoot "dist"))
$ArtifactName = "nanika-$Version-windows-x86_64"
$StageRoot = [IO.Path]::GetFullPath((Join-Path $DistRoot $ArtifactName))
$ArchivePath = [IO.Path]::GetFullPath((Join-Path $DistRoot "$ArtifactName.zip"))
if (-not $StageRoot.StartsWith($DistRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Release staging path escaped the dist directory."
}

$BuildArguments = @("build", "--release", "--locked", "--target", $Target)
foreach ($Package in $Packages) {
    $BuildArguments += @("-p", $Package)
}
& cargo @BuildArguments
if ($LASTEXITCODE -ne 0) {
    throw "Cargo release build failed."
}

New-Item -ItemType Directory -Path $DistRoot -Force | Out-Null
if (Test-Path -LiteralPath $StageRoot) {
    Remove-Item -LiteralPath $StageRoot -Recurse -Force
}
if (Test-Path -LiteralPath $ArchivePath) {
    Remove-Item -LiteralPath $ArchivePath -Force
}
New-Item -ItemType Directory -Path $StageRoot | Out-Null

$BinaryRoot = Join-Path $RepositoryRoot "target\$Target\release"
Copy-Item -LiteralPath (Join-Path $BinaryRoot "nanika-host.exe") -Destination (Join-Path $StageRoot "Nanika.exe")
foreach ($Package in $Packages | Select-Object -Skip 1) {
    Copy-Item -LiteralPath (Join-Path $BinaryRoot "$Package.exe") -Destination $StageRoot
}
Copy-Item -LiteralPath (Join-Path $RepositoryRoot "LICENSE") -Destination $StageRoot
Copy-Item -LiteralPath (Join-Path $RepositoryRoot "packaging\README.txt") -Destination $StageRoot

$Executables = Get-ChildItem -LiteralPath $StageRoot -Filter "*.exe"
if (-not [string]::IsNullOrWhiteSpace($CertificateThumbprint)) {
    if ([string]::IsNullOrWhiteSpace($SignTool)) {
        $SignTool = (Get-Command "signtool.exe" -ErrorAction Stop).Source
    }
    foreach ($Executable in $Executables) {
        & $SignTool sign /sha1 $CertificateThumbprint /fd SHA256 /tr $TimestampUrl /td SHA256 $Executable.FullName
        if ($LASTEXITCODE -ne 0) {
            throw "Signing failed for $($Executable.Name)."
        }
        & $SignTool verify /pa /all $Executable.FullName
        if ($LASTEXITCODE -ne 0) {
            throw "Signature verification failed for $($Executable.Name)."
        }
    }
}

Compress-Archive -LiteralPath $StageRoot -DestinationPath $ArchivePath -CompressionLevel Optimal
$Hash = (Get-FileHash -LiteralPath $ArchivePath -Algorithm SHA256).Hash.ToLowerInvariant()
$ChecksumPath = "$ArchivePath.sha256"
Set-Content -LiteralPath $ChecksumPath -Value "$Hash  $([IO.Path]::GetFileName($ArchivePath))" -Encoding ascii

Write-Output $ArchivePath
Write-Output $ChecksumPath
