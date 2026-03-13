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

Required Bunny FTP environment variables:

- `BUNNY_FTP_HOST`
- `BUNNY_FTP_PORT`
- `BUNNY_FTP_USER`
- `BUNNY_FTP_PASSWORD`

Optional:

- `BUNNY_REMOTE_ROOT`
  Defaults to the authenticated storage-zone root. Do not set this to `/`; Bunny's
  FTP endpoint expects sync targets relative to the logged-in zone root.
- `BUNNY_FTP_PARALLEL`
  Number of parallel file uploads to run during `cdn-push`. Defaults to `8`.
- `BUNNY_FTP_CONNECTION_LIMIT`
  Overall lftp connection cap. Defaults to `12`.
- `BUNNY_SYNC_STATE_FILE`
  Optional local manifest cache used to upload only changed CDN roots on later syncs.
  Defaults to `data/cdn/.last-push-manifest.tsv`.

These values should come from the local `.env`, which is loaded into the `devenv` shells via `dotenv.enable = true`.

`cdn-push` intentionally excludes local placeholder and metadata files such as
`.gitkeep` and `.cdn-metadata.json` from the Bunny upload. It also keeps a local
sync manifest so later pushes only re-scan and upload changed roots instead of
walking the whole CDN tree every time. Large image/tile trees are mirrored at
version-scoped roots such as `images/tiles/minimap/v1` instead of rescanning
all of `images/tiles/`. The `map/` subtree still syncs with delete semantics so
old hashed runtime bundles are cleaned up, while the other roots use
`mirror --continue --only-newer` so interrupted uploads can resume cleanly.
