#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CDN_ROOT="${CDN_ROOT:-$ROOT_DIR/data/cdn/public}"
STATE_FILE="${BUNNY_SYNC_STATE_FILE:-$ROOT_DIR/data/cdn/.last-push-manifest.tsv}"
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

mkdir -p "$(dirname "$STATE_FILE")"

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

join_remote_path() {
  local base="$1"
  local suffix="$2"
  if [ "$base" = "." ]; then
    printf '%s' "$suffix"
  else
    printf '%s/%s' "$base" "$suffix"
  fi
}

lftp_quote() {
  printf "'%s'" "$(printf '%s' "$1" | sed "s/'/'\\\\''/g")"
}

build_manifest() {
  find "$CDN_ROOT" -type f \
    ! -name '.gitkeep' \
    ! -name '.cdn-metadata.json' \
    ! -name '.DS_Store' \
    ! -name 'Thumbs.db' \
    -printf '%P\t%s\t%T@\n' |
    LC_ALL=C sort
}

changed_paths_file="$(mktemp)"
current_manifest_file="$(mktemp)"
sync_roots_file="$(mktemp)"
lftp_script_file="$(mktemp)"

cleanup_tmp_files() {
  rm -f \
    "$changed_paths_file" \
    "$current_manifest_file" \
    "$sync_roots_file" \
    "$lftp_script_file"
}
trap cleanup_tmp_files EXIT

build_manifest > "$current_manifest_file"

if [ -f "$STATE_FILE" ]; then
  awk -F '\t' '
    NR == FNR {
      old[$1] = $0
      next
    }
    {
      current[$1] = $0
      if (!($1 in old) || old[$1] != $0) {
        print $1
      }
    }
    END {
      for (path in old) {
        if (!(path in current)) {
          print path
        }
      }
    }
  ' "$STATE_FILE" "$current_manifest_file" |
    LC_ALL=C sort -u > "$changed_paths_file"
else
  cut -f1 "$current_manifest_file" > "$changed_paths_file"
fi

if [ ! -s "$changed_paths_file" ]; then
  echo "CDN payload unchanged; nothing to upload."
  exit 0
fi

sync_root_for_path() {
  local path="$1"
  case "$path" in
    map/*) printf '%s\n' "map" ;;
    region_groups/*) printf '%s\n' "region_groups" ;;
    images/FishIcons/*) printf '%s\n' "images/FishIcons" ;;
    images/terrain_fullres/*) printf '%s\n' "images/terrain_fullres" ;;
    images/terrain_height/*) printf '%s\n' "images/terrain_height" ;;
    images/terrain/*) printf '%s\n' "images/terrain" ;;
    images/tiles/mask/*) printf '%s\n' "images/tiles/mask" ;;
    images/tiles/minimap/*) printf '%s\n' "images/tiles/minimap" ;;
    images/tiles/region_groups/*) printf '%s\n' "images/tiles/region_groups" ;;
    images/*) printf '%s\n' "images" ;;
    *) dirname "$path" ;;
  esac
}

while IFS= read -r changed_path; do
  sync_root_for_path "$changed_path"
done < "$changed_paths_file" | LC_ALL=C sort -u > "$sync_roots_file"

{
  echo "set cmd:fail-exit yes"
  echo "set xfer:clobber yes"
  echo "set net:max-retries 2"
  echo "set net:timeout 20"
  echo "set net:connection-limit $CONNECTION_LIMIT"
  echo "set ftp:passive-mode yes"
  echo "set mirror:parallel-transfer-count $PARALLEL_TRANSFERS"
  echo "set mirror:parallel-directories yes"
  echo "set mirror:set-permissions off"

  if [ "$REMOTE_ROOT" != "." ]; then
    echo "mkdir -p $(lftp_quote "$REMOTE_ROOT")"
  fi

  while IFS= read -r sync_root; do
    [ -n "$sync_root" ] || continue
    local_dir="$CDN_ROOT/$sync_root"
    remote_dir="$(join_remote_path "$REMOTE_ROOT" "$sync_root")"

    if [ ! -d "$local_dir" ]; then
      continue
    fi

    echo "mkdir -p $(lftp_quote "$remote_dir")"

    case "$sync_root" in
      map)
        echo "glob --exist rm -f $(lftp_quote "$(join_remote_path "$REMOTE_ROOT" "map/fishystuff_ui_bevy.js")")"
        echo "glob --exist rm -f $(lftp_quote "$(join_remote_path "$REMOTE_ROOT" "map/fishystuff_ui_bevy_bg.wasm")")"
        echo "mirror --reverse --delete --verbose --parallel=$PARALLEL_TRANSFERS $MIRROR_EXCLUDE_FLAGS $(lftp_quote "$local_dir") $(lftp_quote "$remote_dir")"
        ;;
      region_groups)
        echo "mirror --reverse --delete --verbose --parallel=$PARALLEL_TRANSFERS $MIRROR_EXCLUDE_FLAGS $(lftp_quote "$local_dir") $(lftp_quote "$remote_dir")"
        ;;
      *)
        echo "mirror --reverse --only-newer --verbose --parallel=$PARALLEL_TRANSFERS $MIRROR_EXCLUDE_FLAGS $(lftp_quote "$local_dir") $(lftp_quote "$remote_dir")"
        ;;
    esac
  done < "$sync_roots_file"

  echo "bye"
} > "$lftp_script_file"

lftp -u "$BUNNY_FTP_USER","$BUNNY_FTP_PASSWORD" -p "$BUNNY_FTP_PORT" "$BUNNY_FTP_HOST" -f "$lftp_script_file"

cp "$current_manifest_file" "$STATE_FILE"
