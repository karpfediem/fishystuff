# site

Zine static site and deployable browser-host assets.

This component should own:

- page layouts and content
- the `/map` page shell
- published static assets under `site/assets/`

Generated map bundle outputs are served from `site/assets/map/`, while hand-edited browser-host source should also stay under `site/assets/map/`.

Runtime image, terrain, and tile assets live under `site/assets/images/`. Zine does not accept directories in `.static_assets`, so site release flows must copy that tree into the final output after `zine release`.

For local map development, do not rely on the plain `zine` dev server alone. It does not publish the full `site/assets/images/` runtime tree. Use `just watch`, which rebuilds `.out`, copies runtime images into it, and serves the generated release output on `http://127.0.0.1:1990/`.
