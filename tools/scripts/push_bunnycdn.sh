#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CDN_ROOT="${CDN_ROOT:-$ROOT_DIR/data/cdn/public}"
STATE_FILE="${BUNNY_SYNC_STATE_FILE:-$ROOT_DIR/data/cdn/.last-push-manifest.tsv}"
REMOTE_ROOT="${BUNNY_REMOTE_ROOT:-.}"
PARALLEL_TRANSFERS="${BUNNY_STORAGE_PARALLEL:-${BUNNY_FTP_PARALLEL:-8}}"
EXPLICIT_SYNC_ROOTS_RAW="${BUNNY_SYNC_ROOTS:-}"
BUNNY_STORAGE_CONNECT_TIMEOUT="${BUNNY_STORAGE_CONNECT_TIMEOUT:-10}"
BUNNY_STORAGE_MAX_TIME="${BUNNY_STORAGE_MAX_TIME:-1800}"
BUNNY_STORAGE_SPEED_LIMIT="${BUNNY_STORAGE_SPEED_LIMIT:-1024}"
BUNNY_STORAGE_SPEED_TIME="${BUNNY_STORAGE_SPEED_TIME:-30}"

BUNNY_STORAGE_ENDPOINT="${BUNNY_STORAGE_ENDPOINT:-${BUNNY_FTP_HOST:-storage.bunnycdn.com}}"
BUNNY_STORAGE_ZONE="${BUNNY_STORAGE_ZONE:-${BUNNY_FTP_USER:-fishystuff}}"
BUNNY_STORAGE_ACCESS_KEY="${BUNNY_STORAGE_ACCESS_KEY:-${BUNNY_FTP_PASSWORD:-}}"

: "${BUNNY_STORAGE_ENDPOINT:?set BUNNY_STORAGE_ENDPOINT or BUNNY_FTP_HOST}"
: "${BUNNY_STORAGE_ZONE:?set BUNNY_STORAGE_ZONE or BUNNY_FTP_USER}"
: "${BUNNY_STORAGE_ACCESS_KEY:?set BUNNY_STORAGE_ACCESS_KEY or BUNNY_FTP_PASSWORD}"

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
STORAGE_BASE_URL="https://${BUNNY_STORAGE_ENDPOINT}/${BUNNY_STORAGE_ZONE}"

join_remote_path() {
  local base="$1"
  local suffix="$2"
  if [ -z "$suffix" ]; then
    printf '%s' "$base"
  elif [ "$base" = "." ]; then
    printf '%s' "$suffix"
  else
    printf '%s/%s' "$base" "$suffix"
  fi
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

sync_root_for_path() {
  local path="$1"
  case "$path" in
    map/*) printf '%s\n' "map" ;;
    region_groups/*) printf '%s\n' "region_groups" ;;
    images/FishIcons/*) printf '%s\n' "images/FishIcons" ;;
    images/terrain_fullres/*) printf '%s\n' "$(printf '%s' "$path" | cut -d/ -f1-3)" ;;
    images/terrain_height/*) printf '%s\n' "$(printf '%s' "$path" | cut -d/ -f1-3)" ;;
    images/terrain/*) printf '%s\n' "$(printf '%s' "$path" | cut -d/ -f1-3)" ;;
    images/tiles/mask/*) printf '%s\n' "$(printf '%s' "$path" | cut -d/ -f1-4)" ;;
    images/tiles/minimap/*) printf '%s\n' "$(printf '%s' "$path" | cut -d/ -f1-4)" ;;
    images/tiles/region_groups/*) printf '%s\n' "$(printf '%s' "$path" | cut -d/ -f1-4)" ;;
    images/*) printf '%s\n' "images" ;;
    *) dirname "$path" ;;
  esac
}

is_delete_root() {
  case "$1" in
    map|region_groups) return 0 ;;
    *) return 1 ;;
  esac
}

list_local_files_under_root() {
  local root="$1"
  local out_file="$2"
  local local_dir="$CDN_ROOT/$root"

  : > "$out_file"
  if [ ! -d "$local_dir" ]; then
    return 0
  fi

  find "$local_dir" -type f \
    ! -name '.gitkeep' \
    ! -name '.cdn-metadata.json' \
    ! -name '.DS_Store' \
    ! -name 'Thumbs.db' \
    -printf "$root/%P\n" |
    LC_ALL=C sort > "$out_file"
}

http_list_dir() {
  local remote_dir="$1"
  local out_file="$2"
  local remote_path
  local url
  local status

  remote_path="$(join_remote_path "$REMOTE_ROOT" "$remote_dir")"
  url="${STORAGE_BASE_URL}/${remote_path}/"

  status="$(
    curl -sS -o "$out_file" -w '%{http_code}' \
      -H "AccessKey: ${BUNNY_STORAGE_ACCESS_KEY}" \
      "$url"
  )"

  case "$status" in
    2*)
      ;;
    404)
      printf '[]\n' > "$out_file"
      ;;
    *)
      echo "failed to list Bunny directory ${remote_dir} (HTTP ${status})" >&2
      cat "$out_file" >&2 || true
      exit 1
      ;;
  esac
}

list_remote_files_recursive() {
  local remote_dir="$1"
  local out_file="$2"
  local response_file

  response_file="$(mktemp)"
  http_list_dir "$remote_dir" "$response_file"

  while IFS=$'\t' read -r object_name is_dir; do
    [ -n "$object_name" ] || continue
    if [ "$is_dir" = "true" ]; then
      list_remote_files_recursive "${remote_dir}/${object_name}" "$out_file"
    else
      printf '%s/%s\n' "$remote_dir" "$object_name" >> "$out_file"
    fi
  done < <(jq -r '.[] | [.ObjectName, (.IsDirectory | tostring)] | @tsv' "$response_file")

  rm -f "$response_file"
}

list_remote_files_under_root() {
  local root="$1"
  local out_file="$2"

  : > "$out_file"
  list_remote_files_recursive "$root" "$out_file"
  LC_ALL=C sort -u -o "$out_file" "$out_file"
}

upload_file() {
  local relative_path="$1"
  local local_path="$CDN_ROOT/$relative_path"
  local remote_path
  local url
  local response_file
  local status

  remote_path="$(join_remote_path "$REMOTE_ROOT" "$relative_path")"
  url="${STORAGE_BASE_URL}/${remote_path}"
  response_file="$(mktemp)"

  echo "PUT ${relative_path}" >&2
  status="$(
    curl -sS -o "$response_file" -w '%{http_code}' \
    --connect-timeout "$BUNNY_STORAGE_CONNECT_TIMEOUT" \
    --max-time "$BUNNY_STORAGE_MAX_TIME" \
    --speed-limit "$BUNNY_STORAGE_SPEED_LIMIT" \
    --speed-time "$BUNNY_STORAGE_SPEED_TIME" \
    --retry 3 --retry-all-errors --retry-delay 2 \
    -X PUT \
    -H "AccessKey: ${BUNNY_STORAGE_ACCESS_KEY}" \
    --upload-file "$local_path" \
    "$url"
  )"

  case "$status" in
    2*)
      echo "PUT ok ${relative_path}" >&2
      ;;
    *)
      echo "failed to upload Bunny path ${relative_path} (HTTP ${status})" >&2
      cat "$response_file" >&2 || true
      rm -f "$response_file"
      exit 1
      ;;
  esac

  rm -f "$response_file"
}

delete_remote_file() {
  local relative_path="$1"
  local remote_path
  local url
  local response_file
  local status

  remote_path="$(join_remote_path "$REMOTE_ROOT" "$relative_path")"
  url="${STORAGE_BASE_URL}/${remote_path}"
  response_file="$(mktemp)"

  status="$(
    curl -sS -o "$response_file" -w '%{http_code}' \
      --retry 3 --retry-all-errors --retry-delay 2 \
      -X DELETE \
      -H "AccessKey: ${BUNNY_STORAGE_ACCESS_KEY}" \
      "$url"
  )"

  case "$status" in
    2*|404)
      echo "DELETE ${relative_path}" >&2
      ;;
    *)
      echo "failed to delete Bunny path ${relative_path} (HTTP ${status})" >&2
      cat "$response_file" >&2 || true
      rm -f "$response_file"
      exit 1
      ;;
  esac

  rm -f "$response_file"
}

run_parallel_uploads() {
  local paths_file="$1"
  local active_jobs=0

  if [ ! -s "$paths_file" ]; then
    return 0
  fi

  while IFS= read -r relative_path; do
    [ -n "$relative_path" ] || continue
    upload_file "$relative_path" &
    active_jobs=$((active_jobs + 1))
    if [ "$active_jobs" -ge "$PARALLEL_TRANSFERS" ]; then
      wait -n
      active_jobs=$((active_jobs - 1))
    fi
  done < "$paths_file"

  while [ "$active_jobs" -gt 0 ]; do
    wait -n
    active_jobs=$((active_jobs - 1))
  done
}

current_manifest_file="$(mktemp)"
changed_paths_file="$(mktemp)"
sync_roots_file="$(mktemp)"
upload_paths_file="$(mktemp)"
local_root_files_file="$(mktemp)"
remote_root_files_file="$(mktemp)"

cleanup_tmp_files() {
  rm -f \
    "$current_manifest_file" \
    "$changed_paths_file" \
    "$sync_roots_file" \
    "$upload_paths_file" \
    "$local_root_files_file" \
    "$remote_root_files_file"
}
trap cleanup_tmp_files EXIT

build_manifest > "$current_manifest_file"

if [ -n "$EXPLICIT_SYNC_ROOTS_RAW" ]; then
  printf '%s\n' "$EXPLICIT_SYNC_ROOTS_RAW" |
    tr ',:' '\n\n' |
    sed 's#^/*##; s#/*$##' |
    awk 'NF > 0' |
    LC_ALL=C sort -u > "$sync_roots_file"
else
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

  while IFS= read -r changed_path; do
    sync_root_for_path "$changed_path"
  done < "$changed_paths_file" | LC_ALL=C sort -u > "$sync_roots_file"
fi

if [ ! -s "$sync_roots_file" ]; then
  echo "no CDN roots selected for upload."
  exit 0
fi

: > "$upload_paths_file"

if [ -n "$EXPLICIT_SYNC_ROOTS_RAW" ]; then
  while IFS= read -r sync_root; do
    list_local_files_under_root "$sync_root" "$local_root_files_file"
    cat "$local_root_files_file" >> "$upload_paths_file"
  done < "$sync_roots_file"
else
  while IFS= read -r sync_root; do
    if is_delete_root "$sync_root"; then
      list_local_files_under_root "$sync_root" "$local_root_files_file"
      cat "$local_root_files_file" >> "$upload_paths_file"
    else
      awk -v prefix="${sync_root}/" 'index($0, prefix) == 1 { print }' "$changed_paths_file" >> "$upload_paths_file"
    fi
  done < "$sync_roots_file"
fi

if [ -s "$upload_paths_file" ]; then
  awk 'NF > 0' "$upload_paths_file" |
    while IFS= read -r relative_path; do
      if [ -f "$CDN_ROOT/$relative_path" ]; then
        printf '%s\n' "$relative_path"
      fi
    done |
    LC_ALL=C sort -u > "${upload_paths_file}.filtered"
  mv "${upload_paths_file}.filtered" "$upload_paths_file"
fi

echo "selected CDN roots:" >&2
sed 's/^/  - /' "$sync_roots_file" >&2
echo "uploading $(wc -l < "$upload_paths_file") files via Bunny HTTP API" >&2
run_parallel_uploads "$upload_paths_file"

while IFS= read -r sync_root; do
  if ! is_delete_root "$sync_root"; then
    continue
  fi

  list_local_files_under_root "$sync_root" "$local_root_files_file"
  list_remote_files_under_root "$sync_root" "$remote_root_files_file"

  while IFS= read -r remote_path; do
    [ -n "$remote_path" ] || continue
    if ! grep -Fxq "$remote_path" "$local_root_files_file"; then
      delete_remote_file "$remote_path"
    fi
  done < "$remote_root_files_file"
done < "$sync_roots_file"

if [ -z "$EXPLICIT_SYNC_ROOTS_RAW" ]; then
  cp "$current_manifest_file" "$STATE_FILE"
fi

echo "Bunny sync complete." >&2
