# Dolt integration and historical tracking

## Goal

Use Dolt as the authoritative source of:
- zone metadata (`zones_merged` view)
- patch timeline
- any known group IDs / drop metadata

Support historical viewing by:
- selecting a Dolt commit hash (or tag/branch)
- exporting/querying tables “as of” that commit

Zone mask images (PNG) are stored outside Dolt (e.g., Git) and referenced by version.

## Recommended Dolt tables/views

- `zones_merged` (view)
- `patches` (table): patch_id, start_ts_utc, end_ts_utc?, notes
- `map_versions` (table): map_version_id, file_path, valid_from_ts?, valid_to_ts?, notes
- `item_table` (table): full Item_Table import (community XLSX)
- `fish_table` (table): encyclopedia_key ↔ item_key mapping + icon fields
- `fish_names_ko` (view): derived from `item_table` (`ItemType=8` + `ItemClassify=16` plus the 3 legacy IDs)
- `languagedata_en` (table): EN localization CSV import (`languagedata_en.csv`)
- `fish_names_en` (view): EN names with KO fallback if EN missing
  - uses `languagedata_en` where `format='A'` and `unk` empty

Seed patches table with `api/sql/patches_seed.csv` (derived from curated patch notes).

## Access patterns

### Option A: CLI export on demand (legacy)
At server start (or per request), run:

- `dolt sql -r csv -q "SELECT * FROM zones_merged;" > zones_merged.csv`
- `dolt sql -r csv -q "SELECT * FROM patches;" > patches.csv`

If historical commit `ref` is specified:
- `dolt checkout <ref>` temporarily in a separate working copy, or
- use Dolt “AS OF” query if available for your workflow.

Cache exports keyed by ref.

### Option B: Run `dolt sql-server` (recommended)
- Start `dolt sql-server` locally
- `fishystuff_server` queries via MySQL protocol using `[dolt_sql]` in `config.toml`
- `/api/zones?ref=<commit>` and `/api/zone_stats?ref=<commit>` use `AS OF '<ref>'` when provided

This keeps Dolt as the single source of truth without needing `--dolt-repo`.

## Verification queries

### fish_names_ko sanity checks

```
SELECT COUNT(*) AS fish_names_ko_count
FROM fish_names_ko;
```

```
SELECT COUNT(*) AS fish_names_ko_intersect
FROM fish_names_ko k
JOIN item_table t ON t.`Index` = k.fish_id
WHERE t.`ItemType` = '8' AND t.`ItemClassify` = '16';
```

```
SELECT COUNT(*) AS fish_names_ko_extra
FROM fish_names_ko
WHERE fish_id IN (40218, 44422, 820036);
```

### fish_names_en sanity checks

```
SELECT COUNT(*) AS fish_names_en_count
FROM fish_names_en;
```

```
SELECT fish_id, name_en
FROM fish_names_en
WHERE fish_id = 8480;
```

### Indexes

```
CREATE INDEX IF NOT EXISTS idx_languagedata_en_id ON languagedata_en (`id`);
```

## Consistency guarantees

Analytics results must be keyed by:
- Dolt ref (commit hash)
- map_version_id (zone mask image version)
- query time window and weighting params

This makes historical views reproducible.
