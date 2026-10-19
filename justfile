set minimum-version := "1.56.0"
set shell := ["bun", "exec"]

default:
    @just --list

# Start the complete desktop application and refuse to activate another build.
dev:
    bun run dev

# Delete Nanika development state, then start from a clean baseline.
dev-fresh:
    bun tooling/development/reset-state.ts
    just dev

# Activate an existing development process, rejecting a different target.
activate pid:
    bun tooling/development/activate.ts {{pid}}

check:
    bun run check

# Run a scenario in the platform's real Tauri WebView.
verify-native scenario="syntax:rendering":
    bun tooling/verification/native.ts {{scenario}}

# Verify real frontend components in an owned headless Chromium session.
verify-ui surface="settings":
    bun tooling/verification/ui.ts {{surface}}

# Drive an isolated real application through WebDriver and native OS input.
verify-e2e scenario="application":
    bun tooling/verification/e2e.ts {{scenario}}

# Measure explicitly; ordinary checks only compile benchmark targets.
bench family *args:
    bun tooling/benchmarks/run.ts {{family}} {{args}}

# Build the current release bundle using the shared Cargo cache.
build:
    bun run build

# Delete the entire target directory. Stop builds and the development app first.
clean:
    bun tooling/build/clean.ts
