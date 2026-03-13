#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CDN_ROOT="${CDN_ROOT:-$ROOT_DIR/data/cdn/public}"
REMOTE_ROOT="${BUNNY_REMOTE_ROOT:-/}"

: "${BUNNY_FTP_HOST:?set BUNNY_FTP_HOST}"
: "${BUNNY_FTP_PORT:?set BUNNY_FTP_PORT}"
: "${BUNNY_FTP_USER:?set BUNNY_FTP_USER}"
: "${BUNNY_FTP_PASSWORD:?set BUNNY_FTP_PASSWORD}"

if [ ! -d "$CDN_ROOT" ]; then
  echo "CDN staging directory does not exist: $CDN_ROOT" >&2
  echo "Run tools/scripts/stage_cdn_assets.sh first." >&2
  exit 1
fi

lftp -u "$BUNNY_FTP_USER","$BUNNY_FTP_PASSWORD" -p "$BUNNY_FTP_PORT" "$BUNNY_FTP_HOST" <<EOF
set cmd:fail-exit yes
set xfer:clobber yes
set net:max-retries 2
set net:timeout 20
mirror --reverse --delete --verbose "$CDN_ROOT" "$REMOTE_ROOT"
bye
EOF
