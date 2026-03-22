#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TREEISH="${1:-HEAD}"

MAP_RUNTIME_TREE_PATHS=(
  "Cargo.toml"
  "Cargo.lock"
  "map/fishystuff_ui_bevy"
  "lib/fishystuff_api"
  "lib/fishystuff_client"
  "lib/fishystuff_core"
)

sanitize_cache_key() {
  local value="$1"
  value="$(printf '%s' "$value" | tr -cs 'A-Za-z0-9._-' '-' | sed -E 's/^-+//; s/-+$//')"
  if [ -z "$value" ]; then
    value="$(date -u +%Y%m%dT%H%M%SZ)"
  fi
  printf '%s\n' "$value"
}

if ! git -C "$ROOT_DIR" rev-parse "${TREEISH}^{tree}" >/dev/null 2>&1; then
  sanitize_cache_key "$(date -u +%Y%m%dT%H%M%SZ)"
  exit 0
fi

key_material_file="$(mktemp)"
cleanup() {
  rm -f "$key_material_file"
}
trap cleanup EXIT

for path in "${MAP_RUNTIME_TREE_PATHS[@]}"; do
  if object_id="$(git -C "$ROOT_DIR" rev-parse "${TREEISH}:${path}" 2>/dev/null)"; then
    printf '%s\t%s\n' "$path" "$object_id" >> "$key_material_file"
  else
    printf '%s\tmissing\n' "$path" >> "$key_material_file"
  fi
done

cache_key="$(sha256sum "$key_material_file" | cut -c1-16)"
sanitize_cache_key "$cache_key"
