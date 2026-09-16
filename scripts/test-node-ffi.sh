#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
if ! command -v node >/dev/null; then
  printf 'node is required to run the Node N-API smoke test\n' >&2
  exit 1
fi

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
  printf 'unsupported host for Node native addon: %s\n' "$(uname -s)" >&2
  exit 1
  ;;
esac

cargo build -p dq-node --release --target-dir "$target_dir"
if [[ ! -f "$addon" ]]; then
  printf 'expected native addon was not produced: %s\n' "$addon" >&2
  exit 1
fi

package_dir="$target_dir/node"
rm -rf "$package_dir"
mkdir -p "$package_dir"
cp "$addon" "$package_dir/dq.node"

DQ_NODE_ADDON="$package_dir/dq.node" node tests/node_ffi.cjs
