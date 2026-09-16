#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
if ! command -v bun >/dev/null; then
  printf 'bun is required to run the Bun N-API smoke test\n' >&2
  exit 1
fi

target_dir="${CARGO_TARGET_DIR:-$PWD/target}"
CARGO_TARGET_DIR="$target_dir" ./scripts/build-bun-ffi.sh
DQ_BUN_PACKAGE="$target_dir/bun" bun tests/bun_ffi.cjs
