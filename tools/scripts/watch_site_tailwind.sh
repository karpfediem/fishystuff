#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/tools/scripts/devenv_process_lib.sh"

cd "$ROOT_DIR/site"
"$ROOT_DIR/tools/scripts/cleanup_managed_processes.sh" \
  "site tailwind watcher" \
  "$ROOT_DIR/tools/scripts/watch_site_tailwind.sh" \
  "./tools/scripts/watch_site_tailwind.sh" \
  "watchexec -r -w content -w layouts -w assets -w scripts -w tailwind.input.css"

devenv_notify_status "building initial Tailwind output"
bun run tailwind:build
devenv_notify_ready "Tailwind CSS built; watching for changes"
exec watchexec -r --postpone \
  -w content \
  -w layouts \
  -w assets \
  -w scripts \
  -w tailwind.input.css \
  --ignore assets/css/site.css \
  --ignore .tailwind \
  --exts smd,md,shtml,html,css,js,mjs,ts \
  -- bun run tailwind:build
