set minimum-version := "1.56.0"
set shell := ["bun", "exec"]

default:
    @just --list

# Start the complete desktop application and refuse to activate another build.
dev:
    bun run dev

# Build and launch a fresh macOS app bundle for Computer Use. This mode has no HMR.
[unix]
dev-computer-use:
    bun run dev:computer-use

# Windows Computer Use can bind the live development window directly.
[windows]
dev-computer-use:
    just dev

# Delete Nanika development state, then start from a clean baseline.
dev-fresh:
    bun tooling/development/reset-state.ts
    just dev

check:
    bun run check

# Build the current release bundle using the shared Cargo cache.
build:
    bun run build

# Delete the entire target directory. Stop builds and the development app first.
clean:
    bun tooling/build/clean.ts
