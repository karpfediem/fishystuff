#!/usr/bin/env python3

from __future__ import annotations

import argparse
import base64
import json
import math
import re
from collections import OrderedDict
from datetime import datetime, timezone
from pathlib import Path, PurePosixPath
import tomllib


TERRAIN_HEIGHT_TILE_SIZE = 512
TERRAIN_HEIGHT_SOURCE_WIDTH = 32000
TERRAIN_HEIGHT_SOURCE_HEIGHT = 27904
TERRAIN_MANIFEST_URL = "/images/terrain/v1/manifest.json"
TERRAIN_DRAPE_MANIFEST_URL = "/images/terrain_drape/minimap/v1/manifest.json"
TERRAIN_HEIGHT_TILES_URL = "/images/terrain_height/v1"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Compute the exact CDN filenames required by the current deployment."
    )
    parser.add_argument("--cdn-root", required=True)
    parser.add_argument("--api-config", required=True)
    parser.add_argument("--runtime-config", default="")
    parser.add_argument("--expected-map-cache-key", default="")
    parser.add_argument("--legacy-icons-json", required=True)
    parser.add_argument("--consumable-icons-json", required=True)
    parser.add_argument("--enchant-icons-json", required=True)
    parser.add_argument("--lightstone-icons-json", required=True)
    parser.add_argument("--fishing-domain-icons-json", required=True)
    parser.add_argument("--fish-catalog-icons-json", required=True)
    parser.add_argument("--fish-table-icons-json", required=True)
    parser.add_argument("--out", default="-")
    return parser.parse_args()


def normalize_rel_path(value: str) -> str:
    return str(PurePosixPath(value.strip().lstrip("/")))


def parse_runtime_config(path: Path) -> dict[str, str]:
    if not path.is_file():
        return {"mapAssetCacheKey": ""}
    text = path.read_text(encoding="utf-8")
    match = re.search(r"Object\.freeze\((\{.*?\})\);?\s*$", text, re.S)
    if not match:
        return {"mapAssetCacheKey": ""}
    try:
        payload = json.loads(match.group(1))
    except json.JSONDecodeError:
        return {"mapAssetCacheKey": ""}
    return {
        "mapAssetCacheKey": str(payload.get("mapAssetCacheKey") or "").strip(),
    }


def load_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def load_rows(path: Path) -> list[dict]:
    with path.open("r", encoding="utf-8") as handle:
        payload = json.load(handle)
    return payload.get("rows", [])


def parse_asset_stem(raw_path: str | None) -> str:
    if not raw_path:
        return ""
    file_name = str(raw_path).strip().split("?", 1)[0].split("#", 1)[0].split("/")[-1]
    if not file_name:
        return ""
    stem = file_name.rsplit(".", 1)[0]
    return stem.strip()


def pad_icon_id(icon_id: int) -> str:
    return f"{icon_id:08d}"


def occupancy_coords(level: dict) -> list[tuple[int, int]]:
    width = int(level["width"])
    height = int(level["height"])
    min_x = int(level.get("min_x", 0))
    min_y = int(level.get("min_y", 0))
    bits = base64.b64decode(level["occupancy_b64"])
    coords: list[tuple[int, int]] = []
    bit_index = 0
    for dy in range(height):
        for dx in range(width):
            if bits[bit_index >> 3] & (1 << (bit_index & 7)):
                coords.append((min_x + dx, min_y + dy))
            bit_index += 1
    return coords


def materialize_template(template: str, values: dict[str, int]) -> str:
    rendered = template
    for key, value in values.items():
        rendered = rendered.replace(f"{{{key}}}", str(value))
    return rendered


class Report:
    def __init__(self, cdn_root: Path) -> None:
        self.cdn_root = cdn_root
        self.groups: OrderedDict[str, list[str]] = OrderedDict()
        self._seen: dict[str, set[str]] = {}

    def add(self, group: str, rel_path: str) -> None:
        rel_path = normalize_rel_path(rel_path)
        if not rel_path:
            return
        seen = self._seen.setdefault(group, set())
        if rel_path in seen:
            return
        seen.add(rel_path)
        self.groups.setdefault(group, []).append(rel_path)

    def add_many(self, group: str, rel_paths: list[str]) -> None:
        for rel_path in rel_paths:
            self.add(group, rel_path)

    def finalize(self) -> dict:
        ordered_groups = OrderedDict()
        required_paths: list[str] = []
        for group, rel_paths in self.groups.items():
            rel_paths.sort()
            ordered_groups[group] = rel_paths
            required_paths.extend(rel_paths)
        required_paths = sorted(dict.fromkeys(required_paths))
        missing_paths = [
            path for path in required_paths if not (self.cdn_root / path).exists()
        ]
        group_counts = OrderedDict(
            (group, len(paths)) for group, paths in ordered_groups.items()
        )
        return {
            "generated_at_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "cdn_root": str(self.cdn_root),
            "summary": {
                "required_count": len(required_paths),
                "present_local_count": len(required_paths) - len(missing_paths),
                "missing_local_count": len(missing_paths),
                "group_counts": group_counts,
            },
            "required_paths": required_paths,
            "paths_by_group": ordered_groups,
            "missing_local_paths": missing_paths,
        }


def add_runtime_assets(report: Report, cdn_root: Path, cache_key: str) -> None:
    manifest_path = cdn_root / "map/runtime-manifest.json"
    report.add("map_runtime", "map/runtime-manifest.json")
    if cache_key:
        report.add("map_runtime", f"map/runtime-manifest.{cache_key}.json")
    if not manifest_path.is_file():
        return
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    module = str(manifest.get("module") or "").strip()
    wasm = str(manifest.get("wasm") or "").strip()
    if module:
        module_path = f"map/{module}"
        report.add("map_runtime", module_path)
        if (cdn_root / f"{module_path}.map").is_file():
            report.add("map_runtime_source_maps", f"{module_path}.map")
    if wasm:
        wasm_path = f"map/{wasm}"
        report.add("map_runtime", wasm_path)
        if (cdn_root / f"{wasm_path}.map").is_file():
            report.add("map_runtime_source_maps", f"{wasm_path}.map")


def add_manifest_tree(report: Report, group: str, cdn_root: Path, rel_manifest_path: str) -> None:
    manifest_path = cdn_root / rel_manifest_path
    report.add(group, rel_manifest_path)
    if not manifest_path.is_file():
        return
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    root = str(manifest.get("root") or "").strip().lstrip("/")
    if not root:
        root = str(PurePosixPath(rel_manifest_path).parent)
    path_template = (
        str(manifest.get("chunk_path") or "").strip()
        or "{z}/{x}_{y}.png"
    )
    for level in manifest.get("levels", []):
        for x, y in occupancy_coords(level):
            rel_path = PurePosixPath(root) / materialize_template(
                path_template,
                {
                    "level": int(level.get("level", level.get("z", 0))),
                    "z": int(level.get("z", level.get("level", 0))),
                    "x": x,
                    "y": y,
                },
            )
            report.add(group, str(rel_path))


def add_tileset_tree(report: Report, group: str, cdn_root: Path, rel_tileset_path: str) -> None:
    tileset_path = cdn_root / rel_tileset_path
    report.add(group, rel_tileset_path)
    if not tileset_path.is_file():
        return
    manifest = json.loads(tileset_path.read_text(encoding="utf-8"))
    root = str(manifest.get("root") or "").strip().lstrip("/")
    if not root:
        root = str(PurePosixPath(rel_tileset_path).parent)
    for level in manifest.get("levels", []):
        path_template = str(level.get("path") or "{z}/{x}_{y}.png")
        for x, y in occupancy_coords(level):
            rel_path = PurePosixPath(root) / materialize_template(
                path_template,
                {
                    "level": int(level.get("level", level.get("z", 0))),
                    "z": int(level.get("z", level.get("level", 0))),
                    "x": x,
                    "y": y,
                },
            )
            report.add(group, str(rel_path))


def add_terrain_height_tiles(report: Report, root_url: str) -> None:
    rel_root = normalize_rel_path(root_url)
    tiles_x = math.ceil(TERRAIN_HEIGHT_SOURCE_WIDTH / TERRAIN_HEIGHT_TILE_SIZE)
    tiles_y = math.ceil(TERRAIN_HEIGHT_SOURCE_HEIGHT / TERRAIN_HEIGHT_TILE_SIZE)
    for tx in range(tiles_x):
        for ty in range(tiles_y):
            report.add("terrain_height", f"{rel_root}/{tx}_{ty}.png")


def add_icon_filename(report: Report, icon_name: str) -> None:
    icon_name = icon_name.strip()
    if not icon_name:
        return
    report.add("item_icons", f"images/items/{icon_name}.webp")


def add_icon_rows(report: Report, rows: list[dict], prefer_icon_id: bool = False, raw_key: str = "") -> None:
    for row in rows:
        raw_source = row.get(raw_key) if raw_key else None
        icon_name = parse_asset_stem(raw_source)
        if not icon_name and prefer_icon_id:
            raw_icon_id = row.get("icon_id")
            if raw_icon_id not in (None, ""):
                try:
                    icon_name = pad_icon_id(int(raw_icon_id))
                except ValueError:
                    icon_name = ""
        if not icon_name:
            raw_item_id = row.get("item_id")
            if raw_item_id not in (None, ""):
                try:
                    icon_name = pad_icon_id(int(raw_item_id))
                except ValueError:
                    icon_name = ""
        if icon_name:
            add_icon_filename(report, icon_name)


def add_lightstone_icons(report: Report, rows: list[dict]) -> None:
    for row in rows:
        stem = parse_asset_stem(row.get("skill_icon_file"))
        if stem:
            add_icon_filename(report, stem)


def add_fish_table_icons(report: Report, rows: list[dict]) -> None:
    for row in rows:
        fish_stem = parse_asset_stem(row.get("fish_item_icon_file"))
        encyclopedia_stem = parse_asset_stem(row.get("encyclopedia_icon_file"))
        if fish_stem:
            add_icon_filename(report, fish_stem)
        if encyclopedia_stem:
            add_icon_filename(report, encyclopedia_stem)


def main() -> None:
    args = parse_args()
    cdn_root = Path(args.cdn_root).resolve()
    api_config = load_toml(Path(args.api_config))
    runtime_config = parse_runtime_config(Path(args.runtime_config)) if args.runtime_config else {}
    runtime_config_cache_key = runtime_config.get("mapAssetCacheKey", "")
    expected_cache_key = str(args.expected_map_cache_key or "").strip()
    map_version = str(api_config.get("defaults", {}).get("map_version") or "v1").strip() or "v1"

    report = Report(cdn_root)

    cache_key = expected_cache_key or runtime_config_cache_key
    add_runtime_assets(report, cdn_root, cache_key)
    report.add_many("map_host", ["map/map-host.js", "map/ui/fishystuff.css"])

    add_tileset_tree(
        report,
        "minimap_visual",
        cdn_root,
        f"images/tiles/minimap_visual/{map_version}/tileset.json",
    )

    report.add_many(
        "zone_mask_semantics",
        [
            f"fields/zone_mask.{map_version}.bin",
            f"fields/zone_mask.{map_version}.meta.json",
        ],
    )
    report.add_many(
        "region_groups_layer",
        [
            f"fields/region_groups.{map_version}.bin",
            f"fields/region_groups.{map_version}.meta.json",
        ],
    )
    report.add_many(
        "regions_layer",
        [
            f"fields/regions.{map_version}.bin",
            f"fields/regions.{map_version}.meta.json",
        ],
    )
    report.add("region_nodes_layer", f"waypoints/region_nodes.{map_version}.geojson")

    add_manifest_tree(
        report,
        "terrain_manifest",
        cdn_root,
        normalize_rel_path(TERRAIN_MANIFEST_URL),
    )
    add_manifest_tree(
        report,
        "terrain_drape_manifest",
        cdn_root,
        normalize_rel_path(TERRAIN_DRAPE_MANIFEST_URL),
    )
    add_terrain_height_tiles(report, TERRAIN_HEIGHT_TILES_URL)

    add_icon_rows(
        report,
        load_rows(Path(args.legacy_icons_json)),
        prefer_icon_id=True,
        raw_key="item_icon_file",
    )
    add_icon_rows(
        report,
        load_rows(Path(args.consumable_icons_json)),
        raw_key="item_icon_file",
    )
    add_icon_rows(
        report,
        load_rows(Path(args.enchant_icons_json)),
        raw_key="item_icon_file",
    )
    add_lightstone_icons(report, load_rows(Path(args.lightstone_icons_json)))
    add_icon_rows(
        report,
        load_rows(Path(args.fishing_domain_icons_json)),
        raw_key="item_icon_file",
    )
    add_icon_rows(
        report,
        load_rows(Path(args.fish_catalog_icons_json)),
        raw_key="item_icon_file",
    )
    add_fish_table_icons(report, load_rows(Path(args.fish_table_icons_json)))

    payload = report.finalize()
    payload["map_version"] = map_version
    payload["runtime_config_path"] = args.runtime_config or ""
    payload["selected_runtime_map_asset_cache_key"] = cache_key
    payload["expected_runtime_map_asset_cache_key"] = expected_cache_key
    payload["runtime_config_map_asset_cache_key"] = runtime_config_cache_key
    if expected_cache_key and runtime_config_cache_key:
        payload["runtime_config_cache_key_matches_expected"] = (
            runtime_config_cache_key == expected_cache_key
        )
    else:
        payload["runtime_config_cache_key_matches_expected"] = None

    encoded = json.dumps(payload, indent=2, sort_keys=False) + "\n"
    if args.out == "-" or not args.out:
        print(encoded, end="")
        return
    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(encoded, encoding="utf-8")


if __name__ == "__main__":
    main()
