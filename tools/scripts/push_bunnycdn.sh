#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CDN_ROOT="${CDN_ROOT:-$ROOT_DIR/data/cdn/public}"
REMOTE_ROOT="${BUNNY_REMOTE_ROOT:-.}"
PARALLEL_TRANSFERS="${BUNNY_FTP_PARALLEL:-8}"
CONNECTION_LIMIT="${BUNNY_FTP_CONNECTION_LIMIT:-12}"
MIRROR_EXCLUDE_FLAGS="--exclude-glob=.gitkeep --exclude-glob=*/.gitkeep --exclude-glob=.cdn-metadata.json --exclude-glob=*/.cdn-metadata.json --exclude-glob=.DS_Store --exclude-glob=*/.DS_Store --exclude-glob=Thumbs.db --exclude-glob=*/Thumbs.db"

: "${BUNNY_FTP_HOST:?set BUNNY_FTP_HOST}"
: "${BUNNY_FTP_PORT:?set BUNNY_FTP_PORT}"
: "${BUNNY_FTP_USER:?set BUNNY_FTP_USER}"
: "${BUNNY_FTP_PASSWORD:?set BUNNY_FTP_PASSWORD}"

if [ ! -d "$CDN_ROOT" ]; then
  echo "CDN staging directory does not exist: $CDN_ROOT" >&2
  echo "Run tools/scripts/stage_cdn_assets.sh first." >&2
  exit 1
fi

normalize_remote_root() {
  local root="$1"
  case "$root" in
    ""|"/"|".")
      printf '.'
      ;;
    /*)
      printf '%s' "${root#/}"
      ;;
    *)
      printf '%s' "$root"
      ;;
  esac
}

REMOTE_ROOT="$(normalize_remote_root "$REMOTE_ROOT")"

REMOTE_MKDIR_CMD=""
if [ "$REMOTE_ROOT" != "." ]; then
  REMOTE_MKDIR_CMD="mkdir -p $REMOTE_ROOT"
fi

lftp -u "$BUNNY_FTP_USER","$BUNNY_FTP_PASSWORD" -p "$BUNNY_FTP_PORT" "$BUNNY_FTP_HOST" <<EOF
set cmd:fail-exit yes
set xfer:clobber yes
set net:max-retries 2
set net:timeout 20
set net:connection-limit $CONNECTION_LIMIT
set ftp:passive-mode yes
set mirror:parallel-transfer-count $PARALLEL_TRANSFERS
set mirror:parallel-directories yes
set mirror:set-permissions off
$REMOTE_MKDIR_CMD
mirror --reverse --delete --verbose --parallel=$PARALLEL_TRANSFERS $MIRROR_EXCLUDE_FLAGS "$CDN_ROOT" "$REMOTE_ROOT"
bye
EOF
