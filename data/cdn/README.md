# CDN staging

Local CDN staging and publish payloads live here.

Expected working layout:

- `data/cdn/public/`
  Publish-ready file tree that mirrors the Bunny storage zone layout. Its contents are local working state and should stay gitignored apart from placeholder `.gitkeep` files.
  The map runtime bundle lives under `data/cdn/public/map/` as hashed `fishystuff_ui_bevy.<hash>.js` and `fishystuff_ui_bevy_bg.<hash>.wasm` files plus a stable `runtime-manifest.json`.
- `data/cdn/logs/`
  Optional local sync logs or ad hoc transfer output.

Production base URL:

- `https://cdn.fishystuff.fish`

Runtime note:

- Set the API/static asset base to `https://cdn.fishystuff.fish` for production so runtime tile, terrain, and map asset URLs resolve against the CDN instead of the site origin.

Required Bunny Storage HTTP environment variables:

- `BUNNY_STORAGE_ENDPOINT`
- `BUNNY_STORAGE_ZONE`
- `BUNNY_STORAGE_ACCESS_KEY`

Optional:

- `BUNNY_REMOTE_ROOT`
  Defaults to the authenticated storage-zone root. Do not set this to `/`; Bunny's
  storage path expects sync targets relative to the logged-in zone root.
- `BUNNY_STORAGE_PARALLEL`
  Number of parallel file uploads to run during `cdn-push`. Defaults to `8`.
- `BUNNY_SYNC_STATE_FILE`
  Optional local manifest cache used to upload only changed CDN roots on later syncs.
  Defaults to `data/cdn/.last-push-manifest.tsv`.
- `BUNNY_SYNC_ROOTS`
  Optional comma-separated root filter such as `map`. This bypasses the broad
  changed-root scan and pushes only the named CDN roots.

Legacy `BUNNY_FTP_*` variables are still accepted as fallbacks so existing local
SecretSpec providers keep working during the transition.

These values are declared in `/home/carp/code/fishystuff/secretspec.toml` under the `cdn`
profile. Populate them in your local SecretSpec provider and run Bunny syncs via
`secretspec run --profile cdn -- ./tools/scripts/push_bunnycdn.sh` or `just cdn-push`.

For a fast map-runtime-only publish path, use:

- `just cdn-sync-map`

That rebuilds the Bevy wasm/js runtime, refreshes staged `map/` host assets, and
pushes only the CDN `map/` subtree instead of scanning unrelated image roots.

`cdn-push` intentionally excludes local placeholder and metadata files such as
`.gitkeep` and `.cdn-metadata.json` from the Bunny upload. It keeps a local sync
manifest so normal runs can upload only locally changed files without asking the
remote for a full file listing. Only delete-semantic roots such as `map/` and
`region_groups/` perform a remote listing, and those are small. The local map
build retains hashed wasm/js bundles for `14` days by default before pruning
them, so older frontend caches can continue fetching the previous runtime for a
short window. Override that retention with `MAP_RUNTIME_RETENTION_DAYS` when
running the map build if needed.
