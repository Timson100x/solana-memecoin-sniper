#!/usr/bin/env bash
set -euo pipefail

echo "==> cargo fmt --check"
cargo fmt --check

echo "==> cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets -- -D warnings

echo "==> cargo test"
cargo test --all-features

echo "==> cargo run -- --config-check"
cargo run -- --config-check
