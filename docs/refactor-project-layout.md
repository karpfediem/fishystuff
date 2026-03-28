# Project Layout Refactor Plan

This note defines the staged split of the current `zonegen/` workspace into explicit top-level components. The goal is to stop treating `zonegen/` as the permanent home of unrelated runtime, tooling, data, and generated concerns.

## Target Layout

- `api/`
  Axum/Tower API runtime, deploy configuration, Dolt-backed schema/data integration, and server-only internals.
- `bot/`
  Deployable Serenity/Poise bot runtime.
- `data/`
  Local developer working directory for large inputs, scratch outputs, and non-runtime datasets.
- `lib/`
  Small shared Rust crates that define stable contracts, core math, ids, and reusable helpers.
- `map/`
  Bevy WASM runtime only.
- `site/`
  Zine static site, browser host assets, and deployable static outputs.
- `tools/`
  Lightweight orchestration scripts plus purpose-built Rust tooling crates.
- `zonegen/`
  Temporary migration area only. It should shrink as crates and assets move out.

## Current -> Target Mapping

| Current component | Target destination | Ownership | Notes |
| --- | --- | --- | --- |
| `zonegen/fishystuff_core` | `lib/fishystuff_core` | shared | Domain math, transforms, masks, terrain primitives, ids. |
| `zonegen/fishystuff_api` | `lib/fishystuff_api` | shared | DTOs, ids, error envelope, version constants. |
| `zonegen/fishystuff_client` | `lib/fishystuff_client` | shared | Typed API client for browser/native callers. |
| `zonegen/fishystuff_config` | `lib/fishystuff_config` | shared | Shared config parsing used by API and offline tools. |
| `zonegen/fishystuff_store` | `lib/fishystuff_store` | shared/tooling | Offline SQLite store used by tooling, not server runtime. |
| `zonegen/fishystuff_zones_meta` | `lib/fishystuff_zones_meta` | shared/tooling | Zone metadata loaders reused by tooling and analytics. |
| `zonegen/fishystuff_analytics` | `lib/fishystuff_analytics` | shared/tooling | Offline analytics helpers shared by tooling. |
| `zonegen/fishystuff_server` | `api/fishystuff_server` | runtime | Axum/Tower API server only. |
| `zonegen/sql` | `api/sql` | runtime | Seed/reference files associated with Dolt-backed runtime setup. |
| `zonegen/config.toml` | `api/config.toml` or `api/config.example.toml` | runtime | API-focused local/dev config. |
| `zonegen/fly.toml` | `api/fly.toml` | runtime | API deployment config. |
| `zonegen/fishystuff_ui_bevy` | `map/fishystuff_ui_bevy` | runtime | WASM Bevy runtime, browser bridge, rendering, local interaction. |
| `zonegen/tools/build_map.sh` | `tools/scripts/build_map.sh` | tooling | Thin orchestration around Rust tools and wasm-bindgen. |
| `zonegen/tools/rebuild_region_groups_overlay.sh` | `tools/scripts/rebuild_region_groups_overlay.sh` | tooling | Thin wrapper for raster-generation tooling. |
| `zonegen/tools/rebuild_water_overlay.sh` | `tools/scripts/rebuild_water_overlay.sh` | tooling | Legacy/offline-only wrapper; keep isolated or delete if obsolete. |
| `zonegen/fishystuff_ingest` | `tools/fishystuff_ingest` | tooling | Ranking import, indexing, offline diagnostics. |
| `zonegen/fishystuff_tilegen` | `tools/fishystuff_tilegen` | tooling | Tile and terrain pyramid generation binaries. |
| `zonegen/fishystuff_dolt_import` | `tools/fishystuff_dolt_import` | tooling | XLSX -> Dolt import tooling. |
| `zonegen/data/**` local source inputs | `data/**` | data | Developer-local CSV/XLSX/imagery/terrain sources and scratch outputs. |
| `zonegen/images/**` runtime-serving checked-in assets | `data/cdn/public/**` | generated/runtime | Runtime CDN payload published separately from the site shell. |
| `zonegen/images/**` raw bake inputs or oversized local working data | `data/**` | data | Inputs stay out of runtime components. |
| `zonegen/docs/**` active architecture/build docs | `docs/**` or component README files | shared | Root docs should describe the new layout, not the legacy container. |
| `zonegen/README.md` | `README.md` + component READMEs | shared | Repo entrypoint should stop directing users into `zonegen/`. |

## Dependency Rules

- `lib/*` crates may be depended on by `api/`, `bot/`, `map/`, and `tools/`.
- `api/` internals are not depended on by `map/`, `bot/`, or `tools/`.
- `map/` depends on shared crates such as `fishystuff_core`, `fishystuff_api`, and `fishystuff_client`, not on `fishystuff_server`.
- `tools/` depend on `lib/*` crates rather than runtime internals where avoidable.
- `bot/` should prefer shared contracts and the typed client over reimplementing server queries.
- `data/` is not a runtime dependency. Runtime components must not assume raw local data exists.
- Generated publishable artifacts belong under `site/assets/` or component-local generated output directories, not mixed into shared library or tooling source trees.

## Migration Sequence

1. Scaffold the new top-level directories, root workspace, and `data/` policy docs.
2. Move stable shared crates into `lib/` and update path dependencies first.
3. Move the API server and its deployment/runtime files into `api/`.
4. Move the Bevy runtime into `map/` and repoint the map build script under `tools/scripts/`.
5. Move offline tooling crates and script wrappers into `tools/`.
6. Reassess the existing top-level `bot/` crate, keep it runtime-only, and align it to the shared-client boundary.
7. Reduce `zonegen/` to migration shims, docs noting moved destinations, and any still-unmoved legacy content.

## Status After This Sweep

Completed in the current migration pass:

- root Cargo workspace added and now owns the active Rust crates
- shared crates moved to `lib/`
- API server plus SQL/config/deploy files moved to `api/`
- Bevy WASM runtime moved to `map/`
- offline tooling crates moved to `tools/`
- map build script moved to `tools/scripts/build_map.sh`
- runtime-serving image, terrain, and tile assets moved under `data/cdn/public/`
- active architecture and pipeline notes promoted from `zonegen/docs/` to root `docs/`
- tracked landmark CSVs moved to `data/landmarks/`
- `zonegen/` no longer contains the active Cargo workspace

Remaining `zonegen/` contents are now primarily:

- legacy local data under `zonegen/data/`
- `devenv` and other migration-era workspace residue
- small compatibility/config leftovers still being phased out

## Guardrails During Migration

- Do not create a second long-lived architecture split between root and `zonegen/`.
- Prefer moving crates whole rather than copying them.
- Keep bash wrappers thin; if a script contains data or transform logic, move that logic into Rust tooling.
- Keep `site/` as the owner of deployable static assets and browser-host files.
- When a move changes runtime/build entrypoints, update README/AGENTS/docs in the same sweep.
