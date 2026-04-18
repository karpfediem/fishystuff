# Source-Backed Minimap Workflow

Date: 2026-04-18

This document covers the maintained workflow for rebuilding the minimap tile
inputs and display pyramid from original Black Desert Online archive data.

The maintained path is:

- original PAZ archive data
- `pazifista` extraction of `rader_*.dds`
- PNG conversion into raw `rader_*.png` tiles
- offline remap into the map-space display pyramid under
  `images/tiles/minimap_visual/v1`

The old ZIP baseline at `data/imagery/minimap_data_pack.zip` is no longer the
maintained source of truth for this workflow.

## When To Use This

Use this workflow when you need to:

- rebuild `data/cdn/public/images/tiles/minimap/` from original source-backed
  archive data
- refresh `data/cdn/public/images/tiles/minimap_visual/v1/` after a game patch
- verify that the repo can regenerate minimap tile state without depending on
  the legacy ZIP baseline

## Prerequisites

Run commands from the repo root inside the repo `devenv`:

```bash
devenv shell
```

The default source archive location is:

- `data/scratch/paz`

In the usual local setup this is a symlink to the installed game `Paz/`
directory. The workflow also accepts an explicit archive root or `.meta` path
through `--source-archive`.

This workflow depends on:

- `cargo`
- `pazifista`
- ImageMagick `magick`

## One Command

The workflow is driven by:

- `tools/scripts/build_minimap_tiles_from_source.mjs`

Default rebuild:

```bash
devenv shell -- node tools/scripts/build_minimap_tiles_from_source.mjs
```

Full rebuild of both the raw tile set and the display pyramid:

```bash
devenv shell -- node tools/scripts/build_minimap_tiles_from_source.mjs --force
```

Raw `rader_*.png` rebuild only:

```bash
devenv shell -- node tools/scripts/build_minimap_tiles_from_source.mjs --skip-visual
```

Explicit archive root:

```bash
devenv shell -- node tools/scripts/build_minimap_tiles_from_source.mjs \
  --source-archive /path/to/Paz
```

## What The Script Does

The script:

1. lists archive matches for
   `ui_texture/new_ui_common_forlua/widget/rader/minimap_data_pack/rader_*.dds`
2. extracts the required `.dds` files with `pazifista`
3. converts them to raw `rader_*.png` tiles under
   `data/cdn/public/images/tiles/minimap/`
4. prunes stale raw PNG tiles that no longer exist in the source archive set
5. writes `data/cdn/public/images/tiles/minimap/source-manifest.json`
6. rebuilds the map-space display pyramid under
   `data/cdn/public/images/tiles/minimap_visual/v1/`

The raw tile directory remains the source input for
`tools/fishystuff_tilegen/src/bin/minimap_display_tiles.rs`.

## Outputs

Raw source-backed tile state:

- `data/cdn/public/images/tiles/minimap/rader_*.png`
- `data/cdn/public/images/tiles/minimap/source-manifest.json`

Display pyramid:

- `data/cdn/public/images/tiles/minimap_visual/v1/tileset.json`
- `data/cdn/public/images/tiles/minimap_visual/v1/{z}/{x}_{y}.png`

## Notes

- These outputs live under `data/cdn/public/`, which is local CDN payload
  state. Do not commit unrelated generated payloads.
- The display pyramid is the runtime visual surface. The raw `rader_*.png`
  tiles are intermediate source-backed inputs used to rebuild that surface.
- The script is incremental by default. It only rebuilds raw PNG tiles whose
  outputs are missing or older than the script, and it only rebuilds the visual
  pyramid when its inputs or configuration changed.
