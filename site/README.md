# site

Zine static site and deployable browser-host assets.

This component should own:

- page layouts and content
- the `/map` page shell
- published static assets under `site/assets/`

Hand-edited browser-host source stays under `site/assets/map/`. The generated wasm/js map runtime bundle is emitted into `data/cdn/public/map/` with hashed filenames and loaded from the CDN, while the copied Bevy UI stylesheet remains at `site/assets/map/ui/fishystuff.css`.

Runtime image, terrain, icon, and tile assets are CDN-served from `data/cdn/public/` locally and `https://cdn.fishystuff.fish/` in production. The site build no longer copies a runtime image tree into `.out`.

For local map development, either run the pieces manually:

- repo root: `just cdn-serve`
- `site/`: `just watch`

Or start the full local stack from the repo root:

- `devenv up`
