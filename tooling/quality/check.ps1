[CmdletBinding()]
param(
    [switch]$MacOS,
    [string]$MacOSTarget = "aarch64-apple-darwin"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$RepositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "../.."))
Push-Location $RepositoryRoot
try {
    foreach ($RemovedRoot in @("crates", "extensions", "scripts", "packaging", "src-tauri", "web", "rust", "dist")) {
        if (Test-Path -LiteralPath $RemovedRoot) {
            throw "Removed top-level path exists: $RemovedRoot"
        }
    }
    $EngineTauri = Get-ChildItem -Path "engine" -Filter "Cargo.toml" -Recurse | Select-String -Pattern '^\s*tauri(?:\s|=|-)'
    if ($EngineTauri) { throw "Engine crates must not depend on Tauri." }
    $FrontendTauri = Get-ChildItem -Path "apps/desktop/frontend/src" -Include "*.ts", "*.svelte" -File -Recurse |
        Where-Object { $_.DirectoryName -notlike "*frontend*src*bridge*" } |
        Select-String -Pattern '@tauri-apps/'
    if ($FrontendTauri) { throw "Frontend source may import Tauri only through the typed bridge." }
    & cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw "Formatting check failed." }
    & cargo clippy --workspace --all-targets --locked -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "Clippy failed." }
    & cargo test --workspace --all-targets --locked
    if ($LASTEXITCODE -ne 0) { throw "Tests failed." }
    & cargo doc --workspace --no-deps --locked
    if ($LASTEXITCODE -ne 0) { throw "Documentation check failed." }
    Push-Location "apps/desktop"
    try {
        & pnpm format:check
        if ($LASTEXITCODE -ne 0) { throw "Frontend formatting check failed." }
        & pnpm lint
        if ($LASTEXITCODE -ne 0) { throw "Frontend lint failed." }
        & pnpm frontend:check
        if ($LASTEXITCODE -ne 0) { throw "Frontend type check failed." }
        & pnpm test
        if ($LASTEXITCODE -ne 0) { throw "Frontend tests failed." }
        & pnpm frontend:build
        if ($LASTEXITCODE -ne 0) { throw "Frontend build failed." }
    }
    finally {
        Pop-Location
    }
    if ($MacOS) {
        & cargo check --workspace --all-targets --locked --target $MacOSTarget
        if ($LASTEXITCODE -ne 0) { throw "macOS cross-target check failed." }
    }
}
finally {
    Pop-Location
}
