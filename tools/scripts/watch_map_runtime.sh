#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

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
