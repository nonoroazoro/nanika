[CmdletBinding()]
param(
    [switch]$MacOS,
    [string]$MacOSTarget = "aarch64-apple-darwin"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$RepositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
Push-Location $RepositoryRoot
try {
    & cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw "Formatting check failed." }
    & cargo clippy --workspace --all-targets --locked -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "Clippy failed." }
    & cargo test --workspace --all-targets --locked
    if ($LASTEXITCODE -ne 0) { throw "Tests failed." }
    & cargo doc --workspace --no-deps --locked
    if ($LASTEXITCODE -ne 0) { throw "Documentation check failed." }
    if ($MacOS) {
        & cargo check --workspace --all-targets --locked --target $MacOSTarget
        if ($LASTEXITCODE -ne 0) { throw "macOS cross-target check failed." }
    }
}
finally {
    Pop-Location
}
