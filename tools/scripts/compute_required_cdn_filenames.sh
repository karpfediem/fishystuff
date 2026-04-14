#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

CDN_ROOT="${CDN_ROOT:-$ROOT_DIR/data/cdn/public}"
OUT_PATH="-"
EXPECTED_MAP_CACHE_KEY=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --out)
      OUT_PATH="${2:?--out requires a value}"
      shift 2
      ;;
    --cdn-root)
      CDN_ROOT="${2:?--cdn-root requires a value}"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

SITE_RUNTIME_CONFIG=""
if [ -f "$ROOT_DIR/site/public/runtime-config.js" ]; then
  SITE_RUNTIME_CONFIG="$ROOT_DIR/site/public/runtime-config.js"
elif [ -f "$ROOT_DIR/site/.out/runtime-config.js" ]; then
  SITE_RUNTIME_CONFIG="$ROOT_DIR/site/.out/runtime-config.js"
fi

if git -C "$ROOT_DIR" rev-parse HEAD >/dev/null 2>&1; then
  EXPECTED_MAP_CACHE_KEY="$(bash "$ROOT_DIR/tools/scripts/resolve_map_runtime_cache_key.sh")"
fi

tmpdir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmpdir"
}
trap cleanup EXIT

run_query() {
  local name="$1"
  local sql="$2"
  dolt sql -r json -q "$sql" > "$tmpdir/$name.json"
}

run_query "legacy-icons" "
SELECT DISTINCT
  CAST(i.icon_id AS SIGNED) AS icon_id,
  CAST(i.id AS SIGNED) AS item_id,
  NULLIF(TRIM(i.name), '') AS display_name,
  NULLIF(TRIM(it.IconImageFile), '') AS item_icon_file
FROM items i
LEFT JOIN item_table it
  ON CAST(it.Index AS SIGNED) = CAST(i.id AS SIGNED)
WHERE i.icon_id IS NOT NULL
ORDER BY CAST(i.icon_id AS SIGNED)
"

run_query "consumable-icons" "
SELECT DISTINCT
  CAST(item_id AS SIGNED) AS item_id,
  NULLIF(TRIM(item_name_ko), '') AS display_name,
  NULLIF(TRIM(item_icon_file), '') AS item_icon_file
FROM calculator_consumable_effect_sources
WHERE item_id IS NOT NULL
ORDER BY CAST(item_id AS SIGNED)
"

run_query "enchant-icons" "
SELECT DISTINCT
  CAST(it.Index AS SIGNED) AS item_id,
  NULLIF(TRIM(it.ItemName), '') AS display_name,
  NULLIF(TRIM(it.IconImageFile), '') AS item_icon_file
FROM calculator_enchant_item_metadata em
JOIN item_table it
  ON CAST(it.Index AS SIGNED) = CAST(em.item_id AS SIGNED)
ORDER BY CAST(it.Index AS SIGNED)
"

run_query "lightstone-icons" "
SELECT DISTINCT
  source_name_en AS display_name,
  set_name_ko,
  skill_icon_file
FROM calculator_lightstone_effect_sources
WHERE NULLIF(TRIM(skill_icon_file), '') IS NOT NULL
"

run_query "fishing-domain-icons" "
SELECT DISTINCT
  CAST(v.item_key AS SIGNED) AS item_id,
  NULLIF(TRIM(it.ItemName), '') AS display_name,
  NULLIF(TRIM(it.IconImageFile), '') AS item_icon_file
FROM item_sub_group_item_variants v
LEFT JOIN item_table it
  ON CAST(it.Index AS SIGNED) = CAST(v.item_key AS SIGNED)
WHERE v.item_key IS NOT NULL
  AND NULLIF(TRIM(it.IconImageFile), '') IS NOT NULL
ORDER BY CAST(v.item_key AS SIGNED)
"

run_query "fish-table-icons" "
SELECT DISTINCT
  CAST(ft.item_key AS SIGNED) AS item_id,
  NULLIF(TRIM(ft.name), '') AS display_name,
  NULLIF(TRIM(ft.icon), '') AS fish_item_icon_file,
  NULLIF(TRIM(ft.encyclopedia_icon), '') AS encyclopedia_icon_file,
  NULLIF(TRIM(it.IconImageFile), '') AS item_icon_file
FROM fish_table ft
LEFT JOIN item_table it
  ON CAST(it.Index AS SIGNED) = CAST(ft.item_key AS SIGNED)
ORDER BY CAST(ft.item_key AS SIGNED)
"

python3 "$ROOT_DIR/tools/scripts/compute_required_cdn_filenames.py" \
  --cdn-root "$CDN_ROOT" \
  --api-config "$ROOT_DIR/api/config.toml" \
  --runtime-config "$SITE_RUNTIME_CONFIG" \
  --expected-map-cache-key "$EXPECTED_MAP_CACHE_KEY" \
  --legacy-icons-json "$tmpdir/legacy-icons.json" \
  --consumable-icons-json "$tmpdir/consumable-icons.json" \
  --enchant-icons-json "$tmpdir/enchant-icons.json" \
  --lightstone-icons-json "$tmpdir/lightstone-icons.json" \
  --fishing-domain-icons-json "$tmpdir/fishing-domain-icons.json" \
  --fish-table-icons-json "$tmpdir/fish-table-icons.json" \
  --out "$OUT_PATH"
