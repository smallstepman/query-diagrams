#!/usr/bin/env bash
set -euo pipefail

example_root="$(cd "$(dirname "$0")/.." && pwd)"
repo_root="$(cd "$example_root/../.." && pwd)"
architecture_dir="$example_root/.architecture"
temporary_dir="$(mktemp -d "${TMPDIR:-/tmp}/polyglot-snake-verify.XXXXXX")"
python="${PYTHON:-python3}"

cleanup() {
  rm -rf "$temporary_dir"
}
trap cleanup EXIT

if ! command -v "$python" >/dev/null 2>&1; then
  printf 'Python 3.11+ is required to validate TOML and JSON artifacts; set PYTHON or install python3.\n' >&2
  exit 1
fi

"$python" "$example_root/scripts/validate_artifacts.py" "$example_root"

if command -v node >/dev/null 2>&1 && command -v npm >/dev/null 2>&1; then
  printf 'Type-checking the TypeScript example in a temporary directory\n'
  cp -R "$example_root/apps/web" "$temporary_dir/web"
  (
    cd "$temporary_dir/web"
    npm ci --ignore-scripts --no-audit --no-fund
    npm run typecheck
  )
else
  printf 'skipping TypeScript typecheck: install Node.js and npm\n' >&2
fi

if command -v cargo >/dev/null 2>&1; then
  printf 'Checking the standalone Rust example\n'
  cp -R "$example_root/services/game-server" "$temporary_dir/game-server"
  CARGO_TARGET_DIR="$architecture_dir/cargo-target" \
    cargo check --manifest-path "$temporary_dir/game-server/Cargo.toml"
else
  printf 'skipping Rust example check: install cargo\n' >&2
fi

printf 'Compiling and importing the Python example in a temporary virtual environment\n'
cp -R "$example_root/pipelines/scoreboard" "$temporary_dir/scoreboard"
"$python" -m compileall -q "$temporary_dir/scoreboard/src"
"$python" -m venv "$temporary_dir/python-venv"
"$temporary_dir/python-venv/bin/python" -m pip install --disable-pip-version-check --no-input "$temporary_dir/scoreboard"
"$temporary_dir/python-venv/bin/python" -c 'from snake_scoreboard import defs; assert defs is not None; print("Python package import passed")'

if ! command -v cargo >/dev/null 2>&1; then
  printf 'cargo is required to run dq perspectives; install Rust or run the built dq binary manually.\n' >&2
  exit 1
fi

mkdir -p "$architecture_dir"
render_perspective() {
  local perspective="$1"
  local expected="$2"
  local actual
  actual="$temporary_dir/$(basename "$expected")"

  CARGO_TARGET_DIR="$architecture_dir/dq-target" \
    cargo run --quiet --manifest-path "$repo_root/Cargo.toml" -p dq -- \
      "$example_root/expected/architecture.d2" \
      --query "$example_root/perspectives/$perspective" \
      --emit mermaid >"$actual"
  diff -u "$example_root/expected/$expected" "$actual"
}

printf 'Reproducing dq component and code/data perspectives\n'
render_perspective "component-overview.dl" "component-overview.mmd"
render_perspective "code-and-data-flow.dl" "code-and-data-flow.mmd"

validator() {
  local command="$1"
  local override_name="$2"
  local executable="${!override_name:-$command}"

  if "$executable" --version >/dev/null 2>&1; then
    printf '%s' "$executable"
    return 0
  fi

  printf 'skipping external %s validation: install %s or set %s\n' \
    "$command" "$command" "$override_name" >&2
  return 1
}

if d2="$(validator d2 DQ_D2)"; then
  printf 'Compiling assembled D2 with %s\n' "$d2"
  "$d2" "$example_root/expected/architecture.d2" "$temporary_dir/architecture.svg"
fi

if merman="$(validator merman-cli DQ_MERMAN_CLI)"; then
  printf 'Parsing generated Mermaid with %s\n' "$merman"
  "$merman" parse "$example_root/expected/component-overview.mmd"
  "$merman" parse "$example_root/expected/code-and-data-flow.mmd"
fi

printf 'polyglot-snake verification passed\n'
