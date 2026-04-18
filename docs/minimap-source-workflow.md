# Source-Backed Minimap Workflow

Date: 2026-04-18

This document covers the maintained workflow for rebuilding the minimap tile
inputs and display pyramid from original Black Desert Online archive data.

The maintained path is:

- original PAZ archive data
- `fishystuff_tilegen` raw baseline generation for `rader_*.png`
- offline remap into the map-space display pyramid under
  `images/tiles/minimap_visual/v1`

The old ZIP baseline at `data/imagery/minimap_data_pack.zip` is no longer the
maintained source of truth for this workflow.

## When To Use This

Use this workflow when you need to:

- rebuild the raw source-backed `rader_*.png` cache from original archive data
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

## Two Layers

The raw baseline generation is owned by:

- `cargo run -p fishystuff_tilegen --bin minimap_source_tiles -- ...`

The repo convenience wrapper is:

- `tools/scripts/build_minimap_tiles_from_source.mjs`

Default end-to-end rebuild:

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

Raw baseline generation only:

```bash
devenv shell -- cargo run -q -p fishystuff_tilegen --bin minimap_source_tiles -- \
  --source-archive data/scratch/paz \
  --out-dir data/scratch/minimap/source_tiles
```

## What Owns What

`minimap_source_tiles` owns:

1. lists archive matches for
   `ui_texture/new_ui_common_forlua/widget/rader/minimap_data_pack/rader_*.dds`
2. extracts the required source payloads through `pazifista` archive handling
3. converts them to raw `rader_*.png` tiles under
   `data/scratch/minimap/source_tiles/`
4. prunes stale raw PNG tiles that no longer exist in the source archive set
5. writes `data/scratch/minimap/source_tiles/source-manifest.json`

The wrapper script owns:

1. invoking `minimap_source_tiles`
2. deciding whether `minimap_visual/v1` needs rebuilding
3. invoking `tools/fishystuff_tilegen/src/bin/minimap_display_tiles.rs`

## Outputs

Raw source-backed tile state:

- `data/scratch/minimap/source_tiles/rader_*.png`
- `data/scratch/minimap/source_tiles/source-manifest.json`

Display pyramid:

- `data/cdn/public/images/tiles/minimap_visual/v1/tileset.json`
- `data/cdn/public/images/tiles/minimap_visual/v1/{z}/{x}_{y}.png`

## Notes

- The display pyramid is the runtime visual surface. The raw `rader_*.png`
  tiles are intermediate source-backed inputs used to rebuild that surface and
  are not part of the staged CDN payload.
- The raw baseline command is incremental by default. It only rebuilds raw PNG
  tiles that are missing unless you pass `--force`.
- The wrapper only rebuilds the visual pyramid when the raw source manifest or
  visual configuration changed, or when you pass `--force` / `--force-visual`.
