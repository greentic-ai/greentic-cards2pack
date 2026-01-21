#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo-binstall >/dev/null 2>&1; then
  cargo install cargo-binstall --locked
fi

cargo binstall -y greentic-flow --force
cargo binstall -y greentic-pack --force

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

if command -v greentic-integration-tester >/dev/null 2>&1; then
  greentic-integration-tester run --gtest tests/gtests/smoke --artifacts-dir artifacts/gtests --workdir .
else
  echo "greentic-integration-tester not found; skipping gtests."
fi
