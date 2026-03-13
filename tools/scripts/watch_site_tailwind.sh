#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/tools/scripts/devenv_process_lib.sh"

cd "$ROOT_DIR/site"
devenv_notify_status "building initial Tailwind output"
bunx @tailwindcss/cli -i tailwind.input.css -o assets/css/site.css
devenv_notify_ready "Tailwind CSS built; watching for changes"
exec bunx @tailwindcss/cli -i tailwind.input.css -o assets/css/site.css --watch
