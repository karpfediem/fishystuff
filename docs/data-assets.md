# Data assets and encoding

This pipeline depends on a small number of canonical assets.

## Source vs generated outputs

Authoritative inputs and hand-edited source:

- `data/**` local tooling inputs and scratch working files
- `data/cdn/**` local CDN staging and sync working tree
- runtime-serving static assets under `site/assets/images/**`
- Rust crates under `lib/**`, `api/**`, `map/**`, and `tools/**`
- browser host source files under `site/assets/map/loader.js`, `site/assets/map/map-host.js`, `site/assets/map/map-host.test.mjs`, and `site/assets/map/package.json`
- Bevy-owned UI stylesheet source under `map/fishystuff_ui_bevy/assets/ui/**`

Generated outputs that should be rebuilt rather than edited by hand:

- `site/assets/map/fishystuff_ui_bevy.js`
- `site/assets/map/fishystuff_ui_bevy_bg.wasm`
- copied Bevy UI stylesheet under `site/assets/map/ui/fishystuff.css`
- terrain pyramids, drape pyramids, and regenerated overlay tile trees under `site/assets/images/**` when they are rebuilt by `tools/scripts/*`
- staged CDN publish tree under `data/cdn/public/**` when it is refreshed by `tools/scripts/stage_cdn_assets.sh`

`site/assets/map/` now contains both browser-host source files and generated wasm bundle artifacts, and `site/assets/images/` mixes hand-maintained runtime assets with generated bake outputs, so the distinction above must stay explicit.

The refactor/audit note for crate boundaries and cleanup targets lives in
`docs/refactor-sweep.md`.

## 1) Water mask: `watermap.png`

- Dimensions: same as worldmap and zone masks (e.g., 11560 × 10540).
- Encoding:
  - **Water pixel**: RGB = **(0,0,255)**
  - Terrain/background: RGB = (0,0,0)
  - Red and green channels may contain roads / NPC density **but water is cleanly blue**.

### API contract

Define:
- `is_water(px, py) := (R==0 && G==0 && B==255)`

All computations that require “fishable area” must use `is_water`.

## 2) Community zone mask(s): `zones_mask_<version>.png`

- Dimensions: same as water mask.
- Each pixel’s RGB encodes a community-defined zone key.
- The mask usually only needs updates every 6–24 months.
- Treat masks as versioned assets:
  - `map_version_id` (string: date or semantic version)
  - file path / hash

### Zone key
Use `rgb_key = "R,G,B"` string form as the stable external key.

## 3) Zone metadata: Dolt `zones_merged` view export

Dolt repo: `fishystuff/fishystuff`  
View: `zones_merged` combines index/name and drop metadata.

CSV columns observed (example export):
- `name`
- `bite_time_min`, `bite_time_max`
- `active`, `confirmed`
- `index`
- `R`, `G`, `B`
- `DropID`, `DropIDHarpoon`, `DropIDNet`
- `DropRate1..5`, `DropID1..5`
- `MinWaitTime`, `MaxWaitTime`

### Requirements
- Must be retrievable for arbitrary Dolt commits (historical views).

## 4) Fish names mapping

Primary source is Dolt:
- `fish_names_ko` view (KO)
- `fish_names_en` view (EN with KO fallback)

`fish_names.tsv` is now legacy-only and should be avoided in production.

## 5) Fish icon mapping

Primary source is Dolt:
- `fish_table` table (encyclopedia_key ↔ item_key, name, icon fields)

Icon files live under `site/assets/images/FishIcons/`. The zone evidence fish ids
match `encyclopedia_key`, so UI lookups should resolve via this column and then
join to the desired icon file name.

## 6) Patch table

A curated table defining patch boundaries in **UTC**:

- `patch_id` (string)
- `patch_name` (optional)
- `start_ts_utc` (required)
- `end_ts_utc` (optional)

This can live in Dolt.

## 7) Ranking events (raw)

Semicolon-separated CSV with headers:
`Date;EncyclopediaKey;Length;FamilyName;CharacterName;X;Y;Z`

- Date format: `DD.MM.YYYY HH:MM`
- Timezone: treat as **UTC+0**
- X/Z are used; Y is ignored for planar mapping.

### Coordinate transforms

We use the existing pixel<->world transform:

Given pixel (image space, origin top-left):

```
world_x = (px * SECTOR_PER_PIXEL + LEFT) * SECTOR_SCALE
world_z = (-(py + 1) * SECTOR_PER_PIXEL + TOP) * SECTOR_SCALE
```

Inverse:

```
px = ((world_x / SECTOR_SCALE) - LEFT) / SECTOR_PER_PIXEL
py = ((TOP - (world_z / SECTOR_SCALE)) / SECTOR_PER_PIXEL) - 1
```

All transforms are float; pixel sampling uses floor/clamp.
