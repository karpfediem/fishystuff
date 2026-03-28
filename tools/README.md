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
- `tools/fishystuff_dolt_import`
  - imports raw fishing workbooks and temporary calculator effect workbooks into Dolt
  - temporary calculator workflow is documented in
    [`docs/calculator-data-path.md`](/home/carp/code/fishystuff/docs/calculator-data-path.md)
  - Dolt schema inspection and schema-history workflow are documented in
    [`docs/dolt-schema-workflow.md`](/home/carp/code/fishystuff/docs/dolt-schema-workflow.md)
- `tools/pazifista`
- `tools/scripts/build_map.sh`
- `tools/scripts/stage_cdn_assets.sh`
  - stages CDN-owned site and map assets under `data/cdn/public/`
  - now rebuilds source-backed calculator item icons into `data/cdn/public/images/items/`
- `tools/scripts/push_bunnycdn.sh`
- `tools/scripts/cleanup_cdn_server.sh`
- `tools/scripts/run_cdn_server.sh`
- `tools/scripts/serve_cdn.py`
- `tools/scripts/rebuild_detailed_regions_layer.sh`
- `tools/scripts/rebuild_detailed_regions_layer_from_pabr.sh`
- `tools/scripts/rebuild_region_groups_vector_layer.sh`
- `tools/scripts/rebuild_region_groups_vector_layer_from_pabr.sh`
- `tools/scripts/rebuild_region_groups_overlay.sh`
- `tools/scripts/rebuild_water_overlay.sh`
- `tools/scripts/extract_fishing_workbooks_from_paz.sh`
- `tools/scripts/build_item_icons_from_source.mjs`
  - resolves the current calculator item icon set from Dolt
  - extracts source `.dds` icon textures from PAZ via `pazifista`
  - converts them to `44x44` WebP under `data/cdn/public/images/items/`
- `tools/scripts/xlsx-*`

For local source-backed item icon generation, use:

```bash
devenv shell -- node tools/scripts/build_item_icons_from_source.mjs
```

To rebuild all current calculator item icons from PAZ source:

```bash
devenv shell -- node tools/scripts/build_item_icons_from_source.mjs --force
```
