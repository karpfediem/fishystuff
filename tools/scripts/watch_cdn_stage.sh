#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

exec watchexec -r \
  -w site/assets/map \
  -w tools/scripts/stage_cdn_assets.sh \
  -w tools/scripts/run_cdn_stage.sh \
  --exts js,css \
  -- ./tools/scripts/run_cdn_stage.sh
