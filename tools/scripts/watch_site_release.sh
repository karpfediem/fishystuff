#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/tools/scripts/devenv_process_lib.sh"

cd "$ROOT_DIR/site"
"$ROOT_DIR/tools/scripts/cleanup_managed_processes.sh" \
  "site release watcher" \
  "$ROOT_DIR/tools/scripts/watch_site_release.sh" \
  "./tools/scripts/watch_site_release.sh" \
  "watchexec -r -w content -w layouts -w assets/CNAME -w assets/config.ziggy -w assets/css -w assets/js -w assets/map -w assets/jsonld -w assets/img/items -w assets/img/embed.png -w assets/img/logo.png -w scripts -w zine.ziggy" \
  "-- just build-release-no-tailwind"

devenv_notify_status "building initial site release"
just build-release-no-tailwind
devenv_notify_ready "site release built; watching for changes"
devenv_run_forever watchexec -r --postpone \
  -w content \
  -w layouts \
  -w assets/CNAME \
  -w assets/config.ziggy \
  -w assets/css \
  -w assets/js \
  -w assets/map \
  -w assets/jsonld \
  -w assets/img/items \
  -w assets/img/embed.png \
  -w assets/img/logo.png \
  -w scripts \
  -w zine.ziggy \
  --ignore assets/js/datastar.js \
  --ignore assets/img/icons.svg \
  --ignore assets/img/guides/*-320.webp \
  --ignore assets/img/guides/*-640.webp \
  --ignore assets/img/favicon-16x16.png \
  --ignore assets/img/favicon-32x32.png \
  --ignore assets/img/logo-32.png \
  --ignore assets/img/logo-64.png \
  --ignore assets/css/fonts/**/*.site.woff2 \
  --exts smd,md,shtml,html,ziggy,css,js,mjs,ts \
  -- just build-release-no-tailwind
