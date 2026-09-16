#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build -p dq-python --release
cargo build -p dq-node --release
if ! rustc --print target-libdir --target wasm32-unknown-unknown >/dev/null; then
  printf '%s\n' \
    'wasm32-unknown-unknown is unavailable; install it with your Rust toolchain, then rerun.' >&2
  exit 1
fi
cargo build -p dq-wasm --release --target wasm32-unknown-unknown
printf '%s\n' \
  'Raw cdylib/WASM artifacts are in the Cargo target directory. Packaging layers are deliberately separate: use maturin for a Python wheel, @napi-rs/cli for .node/npm packaging, wasm-bindgen-cli or wasm-pack for JS bindings, or ./scripts/build-bun-ffi.sh for Bun.' \
  'Run ./scripts/test-node-ffi.sh, ./scripts/test-python-ffi.sh, ./scripts/test-wasm-ffi.sh, and ./scripts/test-bun-ffi.sh to build and load each FFI surface in its host runtime.'
