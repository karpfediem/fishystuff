# tools

Offline and administrative tooling.

This component should contain:

- purpose-built Rust tooling crates
- lightweight orchestration scripts under `tools/scripts/`
- documentation for local bake, import, and maintenance workflows

Scripts should stay thin. If a script accumulates business logic, move that logic into a Rust crate and keep the script as a small wrapper.

Current migration contents:

- `tools/fishystuff_ingest`
- `tools/fishystuff_tilegen`
- `tools/fishystuff_dolt_import`
- `tools/pazifista`
- `tools/scripts/build_map.sh`
- `tools/scripts/stage_cdn_assets.sh`
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
- `tools/scripts/xlsx-*`
