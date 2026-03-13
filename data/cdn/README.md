# CDN staging

Local CDN staging and publish payloads live here.

Expected working layout:

- `data/cdn/public/`
  Publish-ready file tree that mirrors the Bunny storage zone layout.
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

These values should come from the local `.env`, which is loaded into the `devenv` shells via `dotenv.enable = true`.
