#!/usr/bin/env bash
# Run every check CI runs, in CI's order. Stops at the first failure.
set -eu
cd "$(dirname "$0")/.."

run() { echo "==> $*"; "$@"; }

[ -f frontend/dist/index.html ] || run npm --prefix frontend run build
run cargo fmt --check
run cargo clippy --all-targets --all-features -- -D warnings
run cargo test
run npm --prefix frontend run lint
run npx --prefix frontend tsc --noEmit -p frontend
run npm --prefix frontend run test

echo "All checks passed."
