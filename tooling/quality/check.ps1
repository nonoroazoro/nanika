[CmdletBinding()]
param(
    [switch]$MacOS,
    [string]$MacOSTarget = "aarch64-apple-darwin"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$RepositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "../.."))
$QualityTarget = Join-Path ([IO.Path]::GetTempPath()) ("nanika-check-" + [Guid]::NewGuid().ToString("N"))
$PreviousCargoTarget = $env:CARGO_TARGET_DIR
$PreviousCargoIncremental = $env:CARGO_INCREMENTAL
$PreviousRustdocFlags = $env:RUSTDOCFLAGS
New-Item -ItemType Directory -Path $QualityTarget | Out-Null
$env:CARGO_TARGET_DIR = $QualityTarget
$env:CARGO_INCREMENTAL = "0"
if ([string]::IsNullOrWhiteSpace($PreviousRustdocFlags)) {
    $env:RUSTDOCFLAGS = "-D warnings"
}
else {
    $env:RUSTDOCFLAGS = "$PreviousRustdocFlags -D warnings"
}
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
    Push-Location "apps/desktop"
    try {
        & corepack.cmd pnpm extensions:build
        if ($LASTEXITCODE -ne 0) { throw "Extension build failed." }
        & corepack.cmd pnpm extensions:prepare
        if ($LASTEXITCODE -ne 0) { throw "Extension preparation failed." }
        & corepack.cmd pnpm format:check
        if ($LASTEXITCODE -ne 0) { throw "Frontend formatting check failed." }
        & corepack.cmd pnpm lint
        if ($LASTEXITCODE -ne 0) { throw "Frontend lint failed." }
        & corepack.cmd pnpm frontend:check
        if ($LASTEXITCODE -ne 0) { throw "Frontend type check failed." }
        & corepack.cmd pnpm frontend:build
        if ($LASTEXITCODE -ne 0) { throw "Frontend build failed." }
        & corepack.cmd pnpm frontend:test
        if ($LASTEXITCODE -ne 0) { throw "Frontend tests failed." }
    }
    finally {
        Pop-Location
    }
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
    $env:CARGO_TARGET_DIR = $PreviousCargoTarget
    $env:CARGO_INCREMENTAL = $PreviousCargoIncremental
    $env:RUSTDOCFLAGS = $PreviousRustdocFlags
    Remove-Item -LiteralPath $QualityTarget -Recurse -Force
}
