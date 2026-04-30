{
  lib,
  runCommand,
  currentRoot,
  previousRoots ? [ ],
}:
let
  previousRootArgs = lib.concatMapStringsSep " " (root: lib.escapeShellArg "${root}") previousRoots;
in
runCommand "cdn-serving-root" { } ''
  set -euo pipefail

  current_root=${lib.escapeShellArg "${currentRoot}"}
  previous_roots=(${previousRootArgs})

  if [[ ! -d "$current_root" ]]; then
    echo "current CDN root does not exist: $current_root" >&2
    exit 1
  fi

  mkdir -p "$out"
  manifest_assets="$TMPDIR/cdn-serving-assets.ndjson"
  : > "$manifest_assets"

  is_immutable_path() {
    local rel="$1"
    case "$rel" in
      map/runtime-manifest.json|.cdn-metadata.json)
        return 1
        ;;
      map/runtime-manifest.*.json|\
      map/fishystuff_ui_bevy.*.js|\
      map/fishystuff_ui_bevy_bg.*.wasm|\
      images/items/*.webp|\
      images/pets/*.webp|\
      images/tiles/*|\
      images/terrain/*|\
      images/terrain_drape/*|\
      images/terrain_height/*|\
      images/terrain_fullres/*|\
      fields/*|\
      waypoints/*)
        return 0
        ;;
      *)
        return 1
        ;;
    esac
  }

  json_string() {
    local value="$1"
    value="''${value//\\/\\\\}"
    value="''${value//\"/\\\"}"
    value="''${value//$'\n'/\\n}"
    printf '"%s"' "$value"
  }

  link_asset_json() {
    local source="$1"
    local rel="$2"
    local generation="$3"
    local target="$out/$rel"

    case "$rel" in
      /*|*..*)
        echo "unsafe CDN relative path: $rel" >&2
        exit 1
        ;;
    esac

    mkdir -p "$(dirname "$target")"
    ln -s "$source" "$target"
    {
      printf '{"path":'
      json_string "/$rel"
      printf ',"source":'
      json_string "$generation"
      printf ',"store_path":'
      json_string "$source"
      printf '}\n'
    } >> "$manifest_assets"
  }

  while IFS= read -r -d "" file; do
    rel="''${file#"$current_root"/}"
    link_asset_json "$file" "$rel" "current"
  done < <(find -L "$current_root" -type f -print0 | sort -z)

  retained_count=0
  for previous_root in "''${previous_roots[@]}"; do
    if [[ ! -d "$previous_root" ]]; then
      echo "retained CDN root does not exist: $previous_root" >&2
      exit 1
    fi
    retained_count=$((retained_count + 1))
    while IFS= read -r -d "" file; do
      rel="''${file#"$previous_root"/}"
      if ! is_immutable_path "$rel"; then
        continue
      fi
      target="$out/$rel"
      if [[ -e "$target" || -L "$target" ]]; then
        if ! cmp -s "$file" "$target"; then
          echo "immutable CDN path has different bytes across retained roots: /$rel" >&2
          echo "current/earlier: $(readlink -f "$target")" >&2
          echo "retained:        $file" >&2
          exit 1
        fi
        continue
      fi
      link_asset_json "$file" "$rel" "retained"
    done < <(find -L "$previous_root" -type f -print0 | sort -z)
  done

  {
    printf '{\n'
    printf '  "schema_version": 1,\n'
    printf '  "current_root": '
    json_string "$current_root"
    printf ',\n'
    printf '  "retained_roots": ['
    first=1
    for previous_root in "''${previous_roots[@]}"; do
      if [[ "$first" -eq 0 ]]; then
        printf ','
      fi
      first=0
      json_string "$previous_root"
    done
    printf '],\n'
    printf '  "retained_root_count": %s,\n' "$retained_count"
    printf '  "assets": [\n'
    first=1
    while IFS= read -r line; do
      if [[ "$first" -eq 0 ]]; then
        printf ',\n'
      fi
      first=0
      printf '    %s' "$line"
    done < "$manifest_assets"
    printf '\n  ]\n'
    printf '}\n'
  } > "$out/cdn-serving-manifest.json"
''
