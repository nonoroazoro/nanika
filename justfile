set minimum-version := "1.56.0"

[unix]
set shell := ["sh", "-cu"]

[windows]
set shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default:
    @just --list

# Start the complete Tauri development application.
dev:
    corepack pnpm --dir apps/desktop dev

# Delete Nanika development state, then start from a clean baseline.
[unix]
dev-fresh:
    sh tooling/development/reset-state.sh
    corepack pnpm --dir apps/desktop dev

[windows]
dev-fresh:
    & ./tooling/development/reset-state.ps1
    corepack pnpm --dir apps/desktop dev

[unix]
check:
    sh tooling/quality/check.sh

[windows]
check:
    & ./tooling/quality/check.ps1
