#!/usr/bin/env bash
# Local stand-in for .github/workflows/ci.yml. The release builds, wasm check and node parity stay in the workflow.
set -euo pipefail
cd "$(dirname "$0")"
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
echo "ci ok"
