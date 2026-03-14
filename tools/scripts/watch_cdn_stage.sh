#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/tools/scripts/devenv_process_lib.sh"

cd "$ROOT_DIR"
"$ROOT_DIR/tools/scripts/cleanup_managed_processes.sh" \
  "CDN staging watcher" \
  "$ROOT_DIR/tools/scripts/watch_cdn_stage.sh" \
  "./tools/scripts/watch_cdn_stage.sh" \
  "watchexec -r -w site/assets/map -w tools/scripts/stage_cdn_assets.sh -w tools/scripts/run_cdn_stage.sh"

devenv_notify_status "staging initial CDN payload"
./tools/scripts/run_cdn_stage.sh
devenv_notify_ready "CDN payload staged; watching for changes"

exec watchexec -r --postpone \
  -w site/assets/map \
  -w tools/scripts/stage_cdn_assets.sh \
  -w tools/scripts/run_cdn_stage.sh \
  --exts js,css \
  -- ./tools/scripts/run_cdn_stage.sh
