set minimum-version := "1.56.0"

node-version-file := "apps/desktop/.node-version"

[unix]
set shell := ["sh", "-cu"]

[windows]
set shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default:
    @just --list

# Start the complete desktop development application with the package-manager pin
# resolved from apps/desktop/package.json.
[unix]
dev:
    cd apps/desktop && fnm exec --using .node-version corepack pnpm dev

[windows]
dev:
    Push-Location apps/desktop; try { fnm exec --using .node-version corepack.cmd pnpm dev } finally { Pop-Location }

# Delete Nanika development state, then start from a clean baseline.
[unix]
dev-fresh:
    sh tooling/development/reset-state.sh
    cd apps/desktop && fnm exec --using .node-version corepack pnpm dev

[windows]
dev-fresh:
    & ./tooling/development/reset-state.ps1
    Push-Location apps/desktop; try { fnm exec --using .node-version corepack.cmd pnpm dev } finally { Pop-Location }

[unix]
check:
    fnm exec --using {{node-version-file}} sh tooling/quality/check.sh

[windows]
check:
    fnm exec --using {{node-version-file}} powershell.exe -NoLogo -NoProfile -File ./tooling/quality/check.ps1
