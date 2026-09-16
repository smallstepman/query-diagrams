#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
python="${PYTHON:-python3}"
if ! command -v "$python" >/dev/null; then
  printf 'Python interpreter not found: %s\n' "$python" >&2
  exit 1
fi

target_dir="${CARGO_TARGET_DIR:-$PWD/target}"
case "$(uname -s)" in
Darwin)
  extension="$target_dir/release/libdq.dylib"
  module_name="dq.abi3.so"
  ;;
Linux)
  extension="$target_dir/release/libdq.so"
  module_name="dq.abi3.so"
  ;;
MINGW* | MSYS* | CYGWIN*)
  extension="$target_dir/release/dq.dll"
  module_name="dq.pyd"
  ;;
*)
  printf 'unsupported host for Python extension: %s\n' "$(uname -s)" >&2
  exit 1
  ;;
esac

cargo build -p dq-python --release --target-dir "$target_dir"
if [[ ! -f "$extension" ]]; then
  printf 'expected Python extension was not produced: %s\n' "$extension" >&2
  exit 1
fi

package_dir="$target_dir/python"
rm -rf "$package_dir"
mkdir -p "$package_dir"
cp "$extension" "$package_dir/$module_name"

DQ_PYTHON_PACKAGE="$package_dir" "$python" tests/python_ffi.py
