set minimum-version := "1.56.0"

node-version-file := "apps/desktop/.node-version"

[unix]
set shell := ["sh", "-cu"]

[windows]
set shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default:
    @just --list

# Start the complete desktop development application with the package-manager pin
# resolved from apps/desktop/package.json. Refuse to activate another build.
[unix]
dev:
    cd apps/desktop && NANIKA_DEV_REQUIRE_PRIMARY=1 fnm exec --using .node-version corepack pnpm dev

[windows]
dev:
    Push-Location apps/desktop; try { $env:NANIKA_DEV_REQUIRE_PRIMARY = '1'; fnm exec --using .node-version corepack.cmd pnpm dev } finally { Remove-Item Env:NANIKA_DEV_REQUIRE_PRIMARY -ErrorAction SilentlyContinue; Pop-Location }

# Build and launch a fresh macOS app bundle for Computer Use. This mode has no HMR.
[unix]
dev-computer-use:
    cd apps/desktop && fnm exec --using .node-version corepack pnpm dev:computer-use

# Windows Computer Use can bind the live development window directly.
[windows]
dev-computer-use:
    just dev

# Delete Nanika development state, then start from a clean baseline.
[unix]
dev-fresh:
    fnm exec --using {{node-version-file}} node tooling/development/verify-dev-target.ts
    sh tooling/development/reset-state.sh
    cd apps/desktop && NANIKA_DEV_REQUIRE_PRIMARY=1 fnm exec --using .node-version corepack pnpm dev

[windows]
dev-fresh:
    fnm exec --using {{node-version-file}} node tooling/development/verify-dev-target.ts
    & ./tooling/development/reset-state.ps1
    Push-Location apps/desktop; try { $env:NANIKA_DEV_REQUIRE_PRIMARY = '1'; fnm exec --using .node-version corepack.cmd pnpm dev } finally { Remove-Item Env:NANIKA_DEV_REQUIRE_PRIMARY -ErrorAction SilentlyContinue; Pop-Location }

[unix]
check:
    fnm exec --using {{node-version-file}} sh tooling/quality/check.sh

[windows]
check:
    fnm exec --using {{node-version-file}} powershell.exe -NoLogo -NoProfile -File ./tooling/quality/check.ps1
