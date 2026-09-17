#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
architecture_dir="$root/.architecture"
scip_dir="$architecture_dir/scip"
snapshot_dir="$architecture_dir/snapshots"

require_command() {
  local command="$1"
  local install_hint="$2"

  if ! command -v "$command" >/dev/null 2>&1; then
    printf 'missing %s; %s\n' "$command" "$install_hint" >&2
    return 1
  fi
}

missing=0
require_command "scip-typescript" "install with: npm install --global @sourcegraph/scip-typescript" || missing=1
require_command "rust-analyzer" "install Rust Analyzer with your Rust toolchain or package manager" || missing=1
require_command "scip-python" "install with: npm install --global @sourcegraph/scip-python" || missing=1
require_command "scip" "install a release from https://github.com/scip-code/scip/releases or build ./cmd/scip" || missing=1

if ((missing)); then
  printf 'SCIP indexes were not generated. Install the commands above and rerun %s.\n' "$0" >&2
  exit 1
fi

if ! rust-analyzer scip --help >/dev/null 2>&1; then
  printf 'installed rust-analyzer does not expose the unstable scip subcommand; upgrade rust-analyzer.\n' >&2
  exit 1
fi

rm -rf "$scip_dir" "$snapshot_dir"
mkdir -p "$scip_dir" "$snapshot_dir"

web_root="$root/apps/web"
server_root="$root/services/game-server"
scoreboard_root="$root/pipelines/scoreboard"

if [[ ! -d "$web_root/node_modules" ]]; then
  printf 'warning: %s has no node_modules; run (cd %s && npm ci) for dependency-resolved TypeScript symbols\n' \
    "$web_root" "$web_root" >&2
fi

printf 'Indexing TypeScript with %s\n' "$(scip-typescript --version)"
scip-typescript index --cwd "$web_root" --output "$scip_dir/web.scip" --no-progress-bar

printf 'Indexing Rust with %s\n' "$(rust-analyzer --version)"
rust-analyzer scip "$server_root" --output "$scip_dir/server.scip"

printf 'Indexing Python with %s\n' "$(scip-python --version)"
scip-python index --cwd "$scoreboard_root" --project-name polyglot-snake-scoreboard --output "$scip_dir/scoreboard.scip"

snapshot_index() {
  local name="$1"
  local project_root="$2"
  local comment_syntax="$3"
  local index="$scip_dir/$name.scip"

  if [[ ! -s "$index" ]]; then
    printf 'expected SCIP index was not produced: %s\n' "$index" >&2
    exit 1
  fi

  printf 'Validating %s.scip with %s\n' "$name" "$(scip --version)"
  if ! scip lint "$index" >"$snapshot_dir/$name.lint.log" 2>&1; then
    printf 'warning: scip lint reported indexer metadata gaps for %s.scip; inspect %s\n' \
      "$name" "$snapshot_dir/$name.lint.log" >&2
  fi
  scip stats --from "$index" --project-root "$project_root"
  scip snapshot --from "$index" --to "$snapshot_dir/$name" --project-root "$project_root" --comment-syntax "$comment_syntax" --strict=false
}

snapshot_index web "$web_root" "//"
snapshot_index server "$server_root" "//"
snapshot_index scoreboard "$scoreboard_root" "#"

printf 'Wrote binary indexes to %s and inspection snapshots to %s\n' "$scip_dir" "$snapshot_dir"
