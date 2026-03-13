#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/tools/scripts/devenv_process_lib.sh"

cd "$ROOT_DIR"

devenv_notify_status "building initial map runtime bundle"
./tools/scripts/build_map.sh
devenv_notify_ready "map runtime bundle built; watching for changes"

exec watchexec -r \
  -w map/fishystuff_ui_bevy \
  -w lib/fishystuff_api \
  -w lib/fishystuff_client \
  -w lib/fishystuff_core \
  -w Cargo.toml \
  -w Cargo.lock \
  -w tools/scripts/build_map.sh \
  --exts rs,toml,css \
  -- ./tools/scripts/build_map.sh
