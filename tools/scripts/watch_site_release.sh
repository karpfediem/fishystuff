#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/tools/scripts/devenv_process_lib.sh"

cd "$ROOT_DIR/site"
devenv_notify_status "building initial site release"
just build-release
devenv_notify_ready "site release built; watching for changes"
exec just watch-release
