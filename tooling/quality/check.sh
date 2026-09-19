#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$repository_root"

quality_target=$(mktemp -d "${TMPDIR:-/tmp}/nanika-check.XXXXXX")
cleanup() {
    case "$quality_target" in
        */nanika-check.*) rm -rf -- "$quality_target" ;;
        *) echo "Refusing to remove unexpected quality target: $quality_target" >&2 ;;
    esac
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
export CARGO_TARGET_DIR="$quality_target"
export CARGO_INCREMENTAL=0
if [ "$(uname -s)" = "Darwin" ]; then
    export MACOSX_DEPLOYMENT_TARGET=13.0
fi

for removed_root in crates extensions scripts packaging src-tauri web rust dist; do
    if [ -e "$removed_root" ]; then
        echo "Removed top-level path exists: $removed_root" >&2
        exit 1
    fi
done

if grep -R -E --include='Cargo.toml' '^[[:space:]]*tauri([[:space:]]|=|-)' engine; then
    echo "Engine crates must not depend on Tauri." >&2
    exit 1
fi

if grep -R --include='*.ts' --include='*.svelte' --exclude-dir=bridge \
    '@tauri-apps/' apps/desktop/frontend/src; then
    echo "Frontend source may import Tauri only through the typed bridge." >&2
    exit 1
fi

cd apps/desktop
pnpm extensions:build
pnpm extensions:prepare
pnpm format:check
pnpm lint
pnpm frontend:check
pnpm frontend:build
pnpm frontend:test

cd "$repository_root"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo doc --workspace --no-deps --locked
