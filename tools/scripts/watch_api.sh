#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

exec watchexec -r \
  -w api \
  -w lib/fishystuff_api \
  -w lib/fishystuff_client \
  -w lib/fishystuff_core \
  -w Cargo.toml \
  -w Cargo.lock \
  -w tools/scripts/run_api.sh \
  --exts rs,toml \
  -- ./tools/scripts/run_api.sh
