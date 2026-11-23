#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p target
scratch=$(mktemp -d "$(pwd -P)/target/check.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
export TMPDIR="$scratch"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$scratch/cargo}"
cargo fmt --check --manifest-path crates/judge-routes/Cargo.toml
cargo build --locked --manifest-path crates/judge-routes/Cargo.toml
cargo fmt --check --manifest-path crates/judge-routes-tests/Cargo.toml
PATH="$CARGO_TARGET_DIR/debug:$PATH" cargo test --locked --manifest-path crates/judge-routes-tests/Cargo.toml
