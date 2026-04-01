# Community Zone Fish Workflow

Date: 2026-04-01

This document covers the maintainer workflow for adding or updating
community-maintained fish presence and rate guesses in Dolt.

These rows are written to `community_zone_fish_support` and are used by the
map and calculator summary surfaces. The workflow preserves structural
provenance when you know it:

- zone-only presence
- group-level presence
- subgroup-level presence
- community guessed in-group rates

## When To Use This

Use this workflow when you want to say things like:

- `"Pink Dolphin" appears in zone "Lake Flondor"`
- `"Pink Dolphin" has rate 1% in "Lake Flondor"`
- `"Leaffish" appears in zone "Edania - Longing Lake" inside the General group`

## Before You Start

Run these commands from the repo root, or from any subdirectory inside the same
Dolt checkout:

```bash
devenv shell
```

For the two community upsert commands, `--dolt-repo` is optional. If you omit
it, the tool walks up from the current directory until it finds a `.dolt`
directory.

If you are running the command outside the Dolt checkout, pass
`--dolt-repo /path/to/repo`.

## Two Commands

The workflow uses two commands:

- `upsert-community-zone-fish-presence`
  - adds or updates presence-only support
  - default support status is `confirmed`
- `upsert-community-zone-fish-guess`
  - adds or updates a community guessed rate
  - guessed rate is passed as a percent, for example `1` for `1%`

Both commands support:

- `--fish-name`
- `--item-id`

You can pass either one, or both. If you pass both, the tool verifies that they
resolve to the same fish.

## Scope Rules

Presence scope depends on which structural arguments you provide:

- no `--group` and no `--slot-idx`: zone-only presence
- `--group` or `--slot-idx`: group-level presence
- `--subgroup-key` together with a slot/group: subgroup-level presence

Group names map to the existing fishing slots:

- `prize` = slot `1`
- `rare` = slot `2`
- `high-quality` = slot `3`
- `general` = slot `4`
- `trash` = slot `5`

For guessed rates:

- if you omit both `--group` and `--slot-idx`, the tool defaults to `prize`
- if the chosen slot expands to multiple subgroups, pass `--subgroup-key`

## Common Examples

### 1. Zone presence only

Pink Dolphin appears in Lake Flondor:

```bash
cargo run -q -p fishystuff_dolt_import -- \
  upsert-community-zone-fish-presence \
  --zone-name "Lake Flondor" \
  --item-id 820986
```

The same command by fish name:

```bash
cargo run -q -p fishystuff_dolt_import -- \
  upsert-community-zone-fish-presence \
  --zone-name "Lake Flondor" \
  --fish-name "Pink Dolphin"
```

### 2. Community guessed rate

Pink Dolphin has a community guessed rate of `1%` in Lake Flondor:

```bash
cargo run -q -p fishystuff_dolt_import -- \
  upsert-community-zone-fish-guess \
  --zone-name "Lake Flondor" \
  --item-id 820986 \
  --guessed-rate-pct 1
```

This defaults to the `prize` slot when no group is given.

### 3. Group-level presence

Leaffish appears in Edania - Longing Lake inside the General group:

```bash
cargo run -q -p fishystuff_dolt_import -- \
  upsert-community-zone-fish-presence \
  --zone-name "Edania - Longing Lake" \
  --fish-name "Leaffish" \
  --group general
```

### 4. Safer lookup with both name and item ID

If you want an explicit cross-check during entry:

```bash
cargo run -q -p fishystuff_dolt_import -- \
  upsert-community-zone-fish-presence \
  --zone-name "Lake Flondor" \
  --fish-name "Pink Dolphin" \
  --item-id 820986
```

## Suggested Workflow

### 1. Make the upsert

Run the presence or guess command without `--commit` first.

### 2. Inspect the row

Spot check what was written:

```bash
dolt sql -q "
  SELECT
    source_id,
    zone_name,
    item_id,
    fish_name,
    support_status,
    claim_count,
    notes
  FROM community_zone_fish_support
  WHERE zone_name LIKE '%Lake Flondor%'
    AND item_id = 820986
  ORDER BY source_id;
"
```

The `notes` field is expected to carry structural provenance when you provided
it, for example:

- `slot_idx=4;item_main_group_key=...`
- `slot_idx=1;guessed_rate=0.01;item_main_group_key=...;subgroup_key=...`

### 3. Commit when satisfied

Either commit directly from the upsert command:

```bash
cargo run -q -p fishystuff_dolt_import -- \
  upsert-community-zone-fish-presence \
  --zone-name "Edania - Longing Lake" \
  --fish-name "Leaffish" \
  --group general \
  --commit \
  --commit-msg "Add community group presence for Leaffish in Edania - Longing Lake"
```

Or batch several edits together and commit afterward:

```bash
dolt add -A
dolt commit -m "Update community zone fish support"
```

## Troubleshooting

### Ambiguous zone name

If a short zone name matches multiple zones, rerun with the full zone name from
`zones_merged`.

### Ambiguous fish name

Use `--item-id`, or pass both `--fish-name` and `--item-id` so the tool can
verify the match.

### `--subgroup-key` rejected

`--subgroup-key` only works together with `--group` or `--slot-idx`, because
the tool validates that the subgroup belongs to the selected zone slot.

### Guessed rate needs subgroup disambiguation

If a slot has multiple subgroup options, rerun with `--subgroup-key <id>`.

## Verification In Product

After the row is present in Dolt:

- zone presence should show up on the map/calculator summary as community
  presence
- guessed rates should show up as community guesses

If the row looks correct in `community_zone_fish_support` but does not appear in
the product, inspect the structural lineage in `notes` first. The runtime now
uses those fields to place community support into the existing zone
main-group/subgroup structure.
