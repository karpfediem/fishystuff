# Expected Inputs

The `data/` tree is for developer-local inputs and intermediate working sets, not deployable runtime assets.

Expected input categories:

- landmark CSVs
- ranking CSVs
- XLSX imports
- imagery and tileset source files
- terrain source tiles or full-resolution terrain source images
- scratch outputs and intermediate exports from ingestion or bake workflows

Rules:

- keep large raw inputs here instead of under runtime components
- do not make `api/`, `map/`, `bot/`, or `site/` depend on these files at runtime
- if tooling needs documented input names or directory conventions, document them in `data/spec/layout.md`
