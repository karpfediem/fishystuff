#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

until [ -f "$ROOT_DIR/data/cdn/public/map/runtime-manifest.json" ]; do
  echo "waiting for map runtime bundle before staging CDN assets..."
  sleep 1
done

exec ./tools/scripts/stage_cdn_assets.sh
