# data

Local developer working directory for large inputs, scratch outputs, and non-runtime datasets.

Most contents under this directory should remain gitignored. Keep committed documentation under `data/spec/`, allow only small explicit fixtures like `data/landmarks/*.csv`, and avoid teaching runtime components to rely on local raw data.

CDN staging and publish payloads should live under `data/cdn/`, not under `site/`. The actual CDN payload under `data/cdn/public/` is local working state and should stay gitignored apart from placeholder `.gitkeep` files.

Local observability working state also lives here under `data/vector/`. That
directory is runtime-generated local state for Vector process captures, archive
files, and checkpoints, and should remain gitignored.
