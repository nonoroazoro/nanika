#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$repository_root"

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

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo doc --workspace --no-deps --locked

cd apps/desktop
pnpm format:check
pnpm lint
pnpm frontend:check
pnpm test
pnpm frontend:build
