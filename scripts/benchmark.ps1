[CmdletBinding()]
param(
    [string]$Baseline
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$RepositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
Push-Location $RepositoryRoot
try {
    $Arguments = @("bench", "--workspace", "--locked")
    if (-not [string]::IsNullOrWhiteSpace($Baseline)) {
        $Arguments += @("--", "--save-baseline", $Baseline)
    }
    & cargo @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Benchmarks failed."
    }
}
finally {
    Pop-Location
}
