#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/tools/scripts/devenv_process_lib.sh"

cd "$ROOT_DIR/site"
"$ROOT_DIR/tools/scripts/cleanup_managed_processes.sh" \
  "site tailwind watcher" \
  "$ROOT_DIR/tools/scripts/watch_site_tailwind.sh" \
  "./tools/scripts/watch_site_tailwind.sh" \
  "watchexec -r -w content -w layouts -w assets/js -w assets/map -w assets/css/style.css -w scripts -w tailwind.input.css" \
  "-- bun run tailwind:build"

devenv_notify_status "building initial Tailwind output"
bun run tailwind:build
devenv_notify_ready "Tailwind CSS built; watching for changes"
devenv_run_forever watchexec -r --postpone \
  -w content \
  -w layouts \
  -w assets/js \
  -w assets/map \
  -w assets/css/style.css \
  -w scripts \
  -w tailwind.input.css \
  --ignore assets/css/site.css \
  --ignore assets/js/datastar.js \
  --ignore .tailwind \
  --exts smd,md,shtml,html,css,js,mjs,ts \
  -- bun run tailwind:build
