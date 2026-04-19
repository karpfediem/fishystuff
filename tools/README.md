# tools

Offline and administrative tooling.

This component should contain:

- purpose-built Rust tooling crates
- lightweight orchestration scripts under `tools/scripts/`
- documentation for local bake, import, and maintenance workflows

Scripts should stay thin. If a script accumulates business logic, move that logic into a Rust crate and keep the script as a small wrapper.

Current contents:

- `tools/fishystuff_ingest`
- `tools/fishystuff_tilegen`
  - owns the raw minimap baseline generator `minimap_source_tiles`
- `tools/fishystuff_dolt_import`
  - imports raw fishing workbooks and temporary calculator effect workbooks into Dolt
  - community zone fish presence/rate guess workflow is documented in
    [`docs/community-zone-fish-workflow.md`](../docs/community-zone-fish-workflow.md)
  - temporary calculator workflow is documented in
    [`docs/calculator-data-path.md`](../docs/calculator-data-path.md)
  - Dolt schema inspection and schema-history workflow are documented in
    [`docs/dolt-schema-workflow.md`](../docs/dolt-schema-workflow.md)
- `tools/pazifista`
- `tools/scripts/build_map.sh`
  - builds the wasm map runtime
  - rebuilds the maintained non-terrain map runtime assets:
    semantic fields, region-node waypoints, and the minimap display pyramid
- `tools/scripts/stage_cdn_assets.sh`
  - stages CDN-owned site and map assets under `data/cdn/public/`
  - rebuilds source-backed calculator item icons into `data/cdn/public/images/items/`
  - accepts `--map-only` to stage only the map-serving CDN payload without the item icon pass
- `tools/scripts/push_bunnycdn.sh`
- `tools/scripts/run_api.sh`
- `tools/scripts/vector-tap.sh`
  - repo-native entrypoint for live local Vector inspection
  - defaults to bounded JSON samples from the local Vector API
  - exposes stable presets such as `browser-logs`, `process-logs`,
    `raw-traces`, and `to-loki`
- `tools/scripts/rebuild_region_groups_overlay.sh`
- `tools/scripts/rebuild_water_overlay.sh`
- `tools/scripts/extract_fishing_workbooks_from_paz.sh`
- `tools/scripts/build_item_icons_from_source.mjs`
  - resolves the current route-backed item icon set from explicit Dolt/source-table inputs
  - uses the source `IconImageFile` / skill icon filenames as the staged output names when available
  - accepts `--calculator-api-url <url>` when you want an explicit live catalog cross-check
  - extracts source `.dds` icon textures from PAZ via `pazifista`
  - converts them to `44x44` WebP under `data/cdn/public/images/items/`
- `tools/scripts/build_minimap_tiles_from_source.mjs`
  - wraps `minimap_source_tiles` plus `minimap_display_tiles`
  - rebuilds the raw source-backed `rader_*.png` cache under
    `data/scratch/minimap/source_tiles`
  - rebuilds the display pyramid under `data/cdn/public/images/tiles/minimap_visual/v1/`
  - workflow is documented in
    [`docs/minimap-source-workflow.md`](../docs/minimap-source-workflow.md)
- `tools/scripts/xlsx-*`

For local source-backed item icon generation, use:

```bash
devenv shell -- node tools/scripts/build_item_icons_from_source.mjs
```

To rebuild all current calculator item icons from PAZ source:

```bash
devenv shell -- node tools/scripts/build_item_icons_from_source.mjs --force
```

For local source-backed minimap generation, use:

```bash
devenv shell -- node tools/scripts/build_minimap_tiles_from_source.mjs
```

For a map-only local rebuild that also stages the map-serving CDN payload, use:

```bash
just build-map
```

To refresh the broader staged CDN payload, including item icons, use:

```bash
just cdn-stage
```
