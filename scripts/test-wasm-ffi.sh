#!/usr/bin/env bash
set -euo pipefail

# cargo install places wasm-bindgen-cli here even when a shell profile omitted it.
if [[ -d "$HOME/.cargo/bin" ]]; then
  PATH="$HOME/.cargo/bin:$PATH"
fi
cd "$(dirname "$0")/.."
if ! command -v node >/dev/null; then
  printf 'node is required to run the wasm-bindgen smoke test\n' >&2
  exit 1
fi
if ! command -v wasm-bindgen >/dev/null; then
  printf 'wasm-bindgen-cli is required; install it with cargo install wasm-bindgen-cli --version 0.2.128 --locked\n' >&2
  exit 1
fi

if ! rustc --print target-libdir --target wasm32-unknown-unknown >/dev/null; then
  printf 'wasm32-unknown-unknown is required; install it with rustup target add wasm32-unknown-unknown\n' >&2
  exit 1
fi

target_dir="${CARGO_TARGET_DIR:-$PWD/target}"
wasm="$target_dir/wasm32-unknown-unknown/release/dq_wasm.wasm"
package_dir="$target_dir/wasm"

cargo build -p dq-wasm --release --target wasm32-unknown-unknown --target-dir "$target_dir"
if [[ ! -f "$wasm" ]]; then
  printf 'expected WASM module was not produced: %s\n' "$wasm" >&2
  exit 1
fi

rm -rf "$package_dir"
mkdir -p "$package_dir"
wasm-bindgen --target nodejs --out-dir "$package_dir" "$wasm"
DQ_WASM_PACKAGE="$package_dir" node tests/wasm_ffi.cjs
