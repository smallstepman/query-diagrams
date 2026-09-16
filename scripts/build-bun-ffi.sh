#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
target_dir="${CARGO_TARGET_DIR:-$PWD/target}"

case "$(uname -s)" in
Darwin)
  addon="$target_dir/release/libdq_node.dylib"
  ;;
Linux)
  addon="$target_dir/release/libdq_node.so"
  ;;
MINGW* | MSYS* | CYGWIN*)
  addon="$target_dir/release/dq_node.dll"
  ;;
*)
  printf 'unsupported host for Bun native addon: %s\n' "$(uname -s)" >&2
  exit 1
  ;;
esac

cargo build -p dq-node --release --target-dir "$target_dir"
if [[ ! -f "$addon" ]]; then
  printf 'expected native addon was not produced: %s\n' "$addon" >&2
  exit 1
fi

package_dir="$target_dir/bun"
rm -rf "$package_dir"
mkdir -p "$package_dir"
cp bindings/bun/index.js bindings/bun/index.d.ts bindings/bun/package.json "$package_dir/"
cp "$addon" "$package_dir/dq.node"

printf 'Bun package: %s\n' "$package_dir"
printf 'Try: DQ_BUN_PACKAGE=%q bun tests/bun_ffi.cjs\n' "$package_dir"
