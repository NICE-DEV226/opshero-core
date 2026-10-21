#!/usr/bin/env bash
# Runs the same steps as .github/workflows/ci.yml on this machine, so the project can be
# verified when no CI runner is available (for example when Actions cannot start).
# Needs: cargo (toolchain pinned by rust-toolchain.toml), node, bash, java, python3.
# Usage: scripts/ci-local.sh            all three jobs (the mutation check takes several minutes)
#        scripts/ci-local.sh --quick    skip the mutation check
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_TERM_COLOR=always OPSHERO_REQUIRE_NODE=1 OPSHERO_REQUIRE_BASH=1

echo "== job: test"
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked

echo "== job: formal"
scripts/check-formal.sh

if [ "${1:-}" != "--quick" ]; then
  echo "== job: mutation"
  python3 scripts/mutation-check.py
fi
echo "all selected CI steps passed"
