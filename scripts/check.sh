#!/usr/bin/env bash
# Runs every local check: query tests (Python), unit tests and lints (Rust),
# and a wasm build. Optionally validates with Zed's own extension CLI:
#   ZED_EXTENSION_CLI=/path/to/zed-extension scripts/check.sh
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

# Zed builds dev extensions with rustup's toolchain; prefer it here as well.
export PATH="$HOME/.cargo/bin:$PATH"

echo "==> query tests"
uv run -q --no-project --with-requirements tests/requirements.txt pytest -q tests

echo "==> rust tests"
cargo test -q

echo "==> clippy (wasm32-wasip2)"
rustup target add wasm32-wasip2 >/dev/null
cargo clippy -q --target wasm32-wasip2 -- -D warnings

if [[ -n "${ZED_EXTENSION_CLI:-}" ]]; then
    echo "==> zed-extension"
    work="$(mktemp -d)"
    trap 'rm -rf "$work"' EXIT
    mkdir -p "$work/out" "$work/scratch"
    # Build from a clean copy so in-place dev build artifacts are not reused.
    rsync -a --exclude target --exclude tests --exclude .git --exclude grammars \
        --exclude extension.wasm "$root/" "$work/src/"
    "$ZED_EXTENSION_CLI" --source-dir "$work/src" --output-dir "$work/out" --scratch-dir "$work/scratch"
fi

echo "all checks passed"
