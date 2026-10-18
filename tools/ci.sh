#!/usr/bin/env bash
# Local CI: formatting, lints and tests (Rust + UI), the UI build and the freshness of the generated
# TypeScript bindings. Runs every step, prints a summary, exits non-zero if any step failed.
# Usage: tools/ci.sh   (from anywhere; never modifies tracked files)
set -uo pipefail
cd "$(dirname "$0")/.."

failed=()
step() {
    local name=$1
    shift
    echo "==> $name"
    if ! "$@"; then
        failed+=("$name")
    fi
}

bindings_fresh() {
    local tmp
    tmp=$(mktemp -d) || return 1
    cargo run -q -p rc_ui --example export_bindings -- "$tmp/bevy.ts" >/dev/null &&
        cmp -s "$tmp/bevy.ts" ui/src/bevy.ts ||
        { echo "ui/src/bevy.ts is stale: cargo run -p rc_ui --example export_bindings -- ui/src/bevy.ts"; rm -rf "$tmp"; return 1; }
    rm -rf "$tmp"
}

step "cargo fmt" cargo fmt --all --check
step "cargo clippy" cargo clippy --workspace --all-targets -- -D warnings
step "cargo test" cargo test --workspace
step "bindings up to date" bindings_fresh

if [ ! -d ui/node_modules ]; then
    step "npm ci" npm --prefix ui ci
fi
step "ui typecheck" npm --prefix ui run typecheck
step "ui test" npm --prefix ui test
step "ui build" npm --prefix ui run build

if ((${#failed[@]})); then
    echo "CI FAILED: ${failed[*]}"
    exit 1
fi
echo "CI passed"
