set minimum-version := "1.56.0"

node-version-file := "apps/desktop/.node-version"

[unix]
set shell := ["sh", "-cu"]

[windows]
set shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default:
    @just --list

# Start the complete Tauri development application.
dev:
    fnm exec --using {{node-version-file}} corepack pnpm --dir apps/desktop dev

# Delete Nanika development state, then start from a clean baseline.
[unix]
dev-fresh:
    sh tooling/development/reset-state.sh
    fnm exec --using {{node-version-file}} corepack pnpm --dir apps/desktop dev

[windows]
dev-fresh:
    & ./tooling/development/reset-state.ps1
    fnm exec --using {{node-version-file}} corepack pnpm --dir apps/desktop dev

[unix]
check:
    fnm exec --using {{node-version-file}} sh tooling/quality/check.sh

[windows]
check:
    fnm exec --using {{node-version-file}} powershell.exe -NoLogo -NoProfile -File ./tooling/quality/check.ps1
