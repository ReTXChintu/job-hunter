#!/usr/bin/env bash
# Run every check CI would run.
set -euo pipefail
cd "$(dirname "$0")/.."
echo "== TypeScript typecheck"; pnpm -r typecheck
echo "== ESLint";              pnpm -r lint
echo "== Vitest";              pnpm -r test
echo "== cargo fmt";           cargo fmt --all -- --check
echo "== cargo clippy";        cargo clippy --workspace --all-targets -- -D warnings
echo "== cargo test";          cargo test --workspace
echo "All checks passed."
