{
  jq,
  lib,
  runCommand,
  currentRoot,
  previousRoots ? [ ],
  runtimeManifestCacheKeys ? [ ],
}:
let
  previousRootArgs = lib.concatMapStringsSep " " (root: lib.escapeShellArg "${root}") previousRoots;
  runtimeManifestCacheKeyArgs =
    lib.concatMapStringsSep " " (key: lib.escapeShellArg key) runtimeManifestCacheKeys;
in
runCommand "cdn-serving-root" { nativeBuildInputs = [ jq ]; } ''
  set -euo pipefail

  current_root=${lib.escapeShellArg "${currentRoot}"}
  previous_roots=(${previousRootArgs})
  runtime_manifest_cache_keys=(${runtimeManifestCacheKeyArgs})

  if [[ ! -d "$current_root" ]]; then
    echo "current CDN root does not exist: $current_root" >&2
    exit 1
  fi

  runtime_manifest="$current_root/map/runtime-manifest.json"
  if [[ -f "$runtime_manifest" ]]; then
    runtime_module="$(jq -er '.module // empty' "$runtime_manifest")"
    runtime_wasm="$(jq -er '.wasm // empty' "$runtime_manifest")"

    if [[ -z "$runtime_module" || -z "$runtime_wasm" ]]; then
      echo "CDN runtime manifest must name module and wasm: $runtime_manifest" >&2
      exit 1
    fi

    for runtime_rel in "map/$runtime_module" "map/$runtime_wasm"; do
      case "$runtime_rel" in
        /*|*..*)
          echo "unsafe CDN runtime manifest path: $runtime_rel" >&2
          exit 1
          ;;
      esac
      if [[ ! -f "$current_root/$runtime_rel" ]]; then
        echo "CDN runtime manifest references missing current-root file: $runtime_rel" >&2
        exit 1
      fi
    done
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
      map/runtime-manifest.*.json.br|\
      map/runtime-manifest.*.json.gz|\
      map/fishystuff_ui_bevy.*.js|\
      map/fishystuff_ui_bevy.*.js.br|\
      map/fishystuff_ui_bevy.*.js.gz|\
      map/fishystuff_ui_bevy.*.js.map|\
      map/fishystuff_ui_bevy.*.js.map.br|\
      map/fishystuff_ui_bevy.*.js.map.gz|\
      map/fishystuff_ui_bevy_bg.*.wasm|\
      map/fishystuff_ui_bevy_bg.*.wasm.br|\
      map/fishystuff_ui_bevy_bg.*.wasm.gz|\
      map/fishystuff_ui_bevy_bg.*.wasm.map|\
      map/fishystuff_ui_bevy_bg.*.wasm.map.br|\
      map/fishystuff_ui_bevy_bg.*.wasm.map.gz|\
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

  should_skip_current_path() {
    local rel="$1"
    case "$rel" in
      map/runtime-manifest.*.json|\
      map/runtime-manifest.*.json.br|\
      map/runtime-manifest.*.json.gz)
        return 0
        ;;
      *)
        return 1
        ;;
    esac
  }

  while IFS= read -r -d "" file; do
    rel="''${file#"$current_root"/}"
    if should_skip_current_path "$rel"; then
      continue
    fi
    link_asset_json "$file" "$rel" "current"
  done < <(find -L "$current_root" -type f -print0 | sort -z)

  for runtime_manifest_cache_key in "''${runtime_manifest_cache_keys[@]}"; do
    if [[ -z "$runtime_manifest_cache_key" ]]; then
      continue
    fi
    if [[ ! "$runtime_manifest_cache_key" =~ ^[A-Za-z0-9._-]+$ || "$runtime_manifest_cache_key" == *..* ]]; then
      echo "unsafe CDN runtime manifest cache key: $runtime_manifest_cache_key" >&2
      exit 1
    fi
    if [[ ! -f "$runtime_manifest" ]]; then
      echo "cannot publish cache-keyed runtime manifest without stable manifest: $runtime_manifest_cache_key" >&2
      exit 1
    fi

    keyed_runtime_manifest_rel="map/runtime-manifest.$runtime_manifest_cache_key.json"
    keyed_runtime_manifest_target="$out/$keyed_runtime_manifest_rel"
    if [[ -e "$keyed_runtime_manifest_target" || -L "$keyed_runtime_manifest_target" ]]; then
      if ! cmp -s "$runtime_manifest" "$keyed_runtime_manifest_target"; then
        echo "cache-keyed CDN runtime manifest does not match stable manifest: /$keyed_runtime_manifest_rel" >&2
        exit 1
      fi
      continue
    fi
    link_asset_json "$runtime_manifest" "$keyed_runtime_manifest_rel" "current"
  done

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
