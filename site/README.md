# site

Zine static site and deployable browser-host assets.

This component should own:

- page layouts and content
- the `/map` page shell
- published static assets under `site/assets/`

Generated map bundle outputs are served from `site/assets/map/`, while hand-edited browser-host source should also stay under `site/assets/map/`.

Runtime image, terrain, icon, and tile assets are CDN-served from `data/cdn/public/` locally and `https://cdn.fishystuff.fish/` in production. The site build no longer copies a runtime image tree into `.out`.

For local map development, run the site preview and the local CDN server together:

- repo root: `just cdn-serve`
- `site/`: `just watch`
