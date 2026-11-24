#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
native_only=false
if [[ ${1:-} == --native && $# -eq 1 ]]; then
  native_only=true
elif [[ $# -gt 0 ]]; then
  echo "usage: ./check.sh [--native]" >&2
  exit 2
fi
mkdir -p target
scratch=$(mktemp -d "$(pwd -P)/target/check.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
export TMPDIR="$scratch"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$scratch/cargo}"
cargo fmt --check --manifest-path crates/judge-routes/Cargo.toml
product=crates/judge-routes/Cargo.toml
cargo build --locked --message-format=json-render-diagnostics --manifest-path "$product" > "$scratch/build.jsonl"
cargo fmt --check --manifest-path crates/judge-routes-tests/Cargo.toml
CHRONO_TEST_ROUTES="$(cd "$CARGO_TARGET_DIR/debug" && pwd -P)/chrono-judge-routes" PATH="$CARGO_TARGET_DIR/debug:$PATH" cargo test --locked --manifest-path crates/judge-routes-tests/Cargo.toml

coverage_failure() {
  echo "E_VERUS_COVERAGE: $*" >&2
  exit 1
}

verifier=$(command -v "${VERUS:-verus}") || coverage_failure "verifier not found"

prove() {
  chrono-host-bootstrap run-compiler-proof "$scratch" "$1" "$verifier" "$2" ||
    coverage_failure "$1: verifier execution failed"
}

admit_proof() {
  local label=$1
  shift
  chrono-host-bootstrap collect-compiler-proof "$scratch" "$label" "$verifier" \
    0.2026.04.24.f8e1704 f8e17045c4ac17a02e566b8997070f2eecc255be "$@" ||
    coverage_failure "$label: required proof result was not admitted"
}

# Read the exact library selected by Cargo in the build of the tested executable.
metadata=$(jq -ers --arg manifest "$(pwd -P)/$product" '
  [.[] | select(.reason == "compiler-artifact" and .manifest_path == $manifest
                 and .target.kind == ["lib"]) | .filenames[] | select(endswith(".rmeta"))]
  | if length == 1 then .[0] else error("expected the product library") end
' "$scratch/build.jsonl") || coverage_failure "native library was not identified"
filename=${metadata##*/}
stem=${filename#lib}
stem=${metadata%/*}/${stem%.rmeta}
chrono-host-bootstrap check-compiler-sources native "$stem" src/verified_bounds.rs src/verified_admission.rs src/declarations.rs src/selection.rs src/lib.rs ||
  coverage_failure "native library differs from required sources"

if "$native_only"; then exit 0; fi

bounds=crates/judge-routes/src/verified_bounds.rs
expectations=crates/judge-routes-tests/tests/verified_bounds.rs

admission=crates/judge-routes/src/verified_admission.rs
admission_tests=crates/judge-routes-tests/tests/verified_admission.rs

prove route-admission "$admission"
prove route-admission-tests "$admission_tests"
prove operation-bounds "$bounds"
prove operation-bounds-tests "$expectations"

admit_proof operation-bounds "$bounds"
admit_proof operation-bounds-tests "$expectations" crates/judge-routes-tests/tests/../../judge-routes/src/verified_bounds.rs

admit_proof route-admission "$admission"
admit_proof route-admission-tests "$admission_tests" crates/judge-routes-tests/tests/../../judge-routes/src/verified_admission.rs
