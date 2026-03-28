# Dolt Schema Workflow

Schema authority in this repo lives in Dolt.

- current schema: the checked-out Dolt working set
- schema history: Dolt commits
- historical inspection: Dolt refs such as commit hashes, branches, and tags

Do not use numbered SQL migrations as a second schema history.

## Principles

- Treat Dolt as the source of truth for both schema and data.
- Inspect the current schema directly from Dolt, not from checked-in SQL files.
- Record schema changes as Dolt commits.
- Do not maintain a second checked-in schema history.

## Everyday Commands

### See the current schema inventory

```bash
devenv shell -- bash -lc 'dolt sql -q "SHOW FULL TABLES"'
```

```bash
devenv shell -- bash -lc \
  'dolt sql -q "
    SELECT table_name, table_type
    FROM information_schema.tables
    WHERE table_schema = DATABASE()
    ORDER BY table_type, table_name
  "'
```

### Inspect one table or view definition

```bash
devenv shell -- bash -lc 'dolt sql -q "SHOW CREATE TABLE item_table"'
```

```bash
devenv shell -- bash -lc 'dolt sql -q "SHOW CREATE TABLE calculator_enchant_item_effect_entries"'
```

```bash
devenv shell -- bash -lc 'dolt sql -q "DESCRIBE calculator_lightstone_effect_sources"'
```

### Verify whether a table or view exists

```bash
devenv shell -- bash -lc 'dolt sql -q "SHOW TABLES LIKE '\''some_table_name'\''"'
```

### Inspect schema history

```bash
devenv shell -- bash -lc 'dolt log --stat'
```

```bash
devenv shell -- bash -lc 'dolt show <commit>'
```

If you need to understand an older schema shape, inspect the relevant Dolt ref
instead of replaying old SQL patches.

## Making Schema Changes

The supported workflow is:

1. Change the schema in your local Dolt repo.
2. Validate the affected queries, importers, and runtime paths.
3. Commit the schema change in Dolt.
4. Commit the matching code and documentation changes in git.

Typical flow:

```bash
devenv shell -- bash -lc 'dolt status'
devenv shell -- bash -lc 'dolt sql -q "SHOW FULL TABLES"'
devenv shell -- bash -lc 'dolt add -A && dolt commit -m "Describe schema change"'
git add docs/ api/ tools/
git commit -m "Describe schema change"
```

The Dolt commit is the canonical schema record. The git commit should explain
the code/docs change that goes with it.

## Import Tooling

Importers should run against a Dolt repo that already contains the desired
schema. If the repo is missing required objects, move to a Dolt ref that
contains them instead of replaying checked-in SQL patches.

## Why This Repo Uses Dolt This Way

- Dolt versions schema and data together.
- Historical schema changes are already inspectable through Dolt history.
- A second migration chain creates drift and duplicate maintenance.
- Runtime behavior should be tied to a real Dolt ref, not to a pile of replayed
  SQL files.
