# Data Layout

Suggested developer-local layout:

- `data/landmarks/`
  Small tracked landmark CSV fixtures and manually curated map-reference tables.
- `data/ranking/`
  Ranking CSVs and related import inputs.
- `data/xlsx/`
  XLSX tables and extracted spreadsheet inputs.
- `data/imagery/`
  Large image sources, local raster inputs, and imagery staging files.
- `data/terrain/`
  Terrain source tiles, full-resolution terrain images, and terrain bake staging files.
- `data/cdn/`
  Local CDN staging tree, sync logs, and publish-ready payloads for `cdn.fishystuff.fish`.
- `data/scratch/`
  Intermediate exports, temporary manifests, reports, and ad hoc working outputs.

Guidelines:

- prefer stable, descriptive subdirectories over dumping files directly at `data/`
- treat this tree as developer-local state unless a file is explicitly documented as a tiny fixture
- when a tool produces publishable or deployable outputs, publish those to `site/assets/` or another runtime destination instead of treating `data/` as a serving root
