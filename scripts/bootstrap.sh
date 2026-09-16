#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

if command -v rustup >/dev/null 2>&1; then
  rustup toolchain install stable --profile minimal
  rustup default stable
  rustup component add rustfmt
elif ! command -v rustfmt >/dev/null 2>&1; then
  printf '%s\n' 'rustfmt is required; install it with your Rust toolchain, then rerun.' >&2
  exit 1
fi


cd "$(dirname "$0")/.."
cargo build -p dq --release
cargo test -p dq-core

printf '\nBuilt dq.\n'
printf 'Try: cargo run -p dq -- examples/architecture.d2 -q examples/all.dl --emit mermaid\n'
