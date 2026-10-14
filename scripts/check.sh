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
    rsync -a --exclude target --exclude tests --exclude .git --exclude grammars \
        --exclude extension.wasm "$root/" "$work/src/"
    # The CLI insists every grammar a language uses is declared; the real
    # manifest must not declare `python` (see extension.toml), so add it only
    # to this throwaway copy. Pinned to the tree-sitter-python Zed links (0.25).
    # Packaging after validation also wants a `repository`, which a local dev
    # extension does not need; give the copy a placeholder.
    sed -i.bak 's|^schema_version = 1$|schema_version = 1\nrepository = "https://example.invalid/pytorch-inline-kernels"|' \
        "$work/src/extension.toml"
    cat >>"$work/src/extension.toml" <<'EOF'

[grammars.python]
repository = "https://github.com/tree-sitter/tree-sitter-python"
rev = "d326e4cad262cf681656e130960e49dfc04c03ea"
EOF
    "$ZED_EXTENSION_CLI" --source-dir "$work/src" --output-dir "$work/out" --scratch-dir "$work/scratch"
fi

echo "all checks passed"
