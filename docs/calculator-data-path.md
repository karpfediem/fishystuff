# Temporary Calculator Data Path

Date: 2026-03-27

This document describes the current temporary path for getting the fishing
calculator onto mostly real data while the original PAZ-side table decoding is
still incomplete.

The long-term target remains:

1. original PAZ data
2. decoded original tables
3. normalized Dolt tables/views
4. calculator API
5. calculator UI

This document covers the temporary bridge in the middle.

## Current Shape

The calculator data problem splits into two layers:

1. Haul/zone/fish structure
2. Item, buff, and effect data

The first layer is already largely in Dolt. The second layer was the main gap.

## Runtime Sources

For calculator runtime reads, treat the database as the authority.

### Zone and haul structure

Use existing Dolt tables/views:

- `zones_merged`
  - zone catalog/search surface
  - curated zone name and RGB metadata live here
- `fishing_zone_slots`
  - per-zone slot weights
- `item_main_group_options`
  - main-group to subgroup expansion
- `item_sub_group_item_variants`
  - subgroup to item expansion
- `fish_table`
  - fish identity and icon data

This follows the two-dice model from
[`data/fishing_tables_101/fishing_tables_101.md`](/home/carp/code/fishystuff/data/fishing_tables_101/fishing_tables_101.md):

- dice 1: zone/group selection
- dice 2: subgroup/item selection

The calculator should prefer these Dolt surfaces over local CSV files at
runtime, because zone names and RGB metadata will be maintained in the
database.

### Item and effect layer

Temporary effect-source data is now imported from intermediate workbook files
under `data/data/excel/` into raw Dolt tables:

- `buff_table`
- `skill_table_new`
- `skilltype_table_new`
- `lightstone_set_option`
- `pet_table`
- `pet_skill_table`
- `pet_base_skill_table`
- `pet_setstats_table`
- `pet_equipskill_table`
- `pet_grade_table`
- `pet_exp_table`
- `upgradepet_looting_percent`

These raw tables are then exposed through calculator-focused views:

- `calculator_consumable_effects`
  - consumables and event foods
  - source chain: `item_table -> skill_table_new -> buff_table`
- `calculator_lightstone_set_effects`
  - fishing-relevant lightstone set effects
- `calculator_pet_skill_options`
  - fishing-relevant pet skill/talent/special options

Helper views used by those surfaces:

- `calculator_skill_buffs`
- `calculator_item_skill_sources`
- `calculator_pet_skill_sources`

## Temporary Workflow

### 1. Ensure the required schema surfaces exist

Run imports against a Dolt repo that already contains the required tables and
views.

```bash
devenv shell -- bash -lc \
  'dolt sql -q "
    SHOW FULL TABLES
  "'
```

Minimum spot check before running calculator imports:

```bash
devenv shell -- bash -lc \
  'dolt sql -q "
    SHOW FULL TABLES LIKE '\''item_table'\'';
    SHOW FULL TABLES LIKE '\''languagedata_en'\'';
    SHOW FULL TABLES LIKE '\''calculator_lightstone_effect_sources'\'';
  "'
```

For schema inspection and schema-history workflow, use Dolt directly. This repo
does not treat checked-in migration files as authoritative history.

### 2. Import the temporary effect workbooks

Run the importer against the intermediate workbook directory:

```bash
devenv shell -- cargo run -q -p fishystuff_dolt_import -- \
  import-calculator-effects-xlsx \
  --dolt-repo . \
  --excel-dir data/data/excel
```

What this importer currently reads:

- `Buff_Table.xlsx`
- `Skill_Table_New.xlsx`
- `SkillType_Table_New.xlsx`
- `LightStoneSetOption.xlsx`
- `Pet_Table.xlsx`
- `Pet_Skill_Table.xlsx`
- `Pet_BaseSkill_Table.xlsx`
- `Pet_SetStats_Table.xlsx`
- `Pet_EquipSkill_Table.xlsx`
- `Pet_EquipSkill_Aquire_Table.xlsx`
- `Pet_Grade_Table.xlsx`
- `Pet_Exp_Table.xlsx`
- `UpgradePet_Looting_Percent.xlsx`

Notes:

- this is intentionally a temporary path
- it uses workbook files already present locally
- it does not yet replace the future PAZ-derived/original import path

### 3. Commit the Dolt data changes

After import:

```bash
devenv shell -- bash -lc 'dolt add -A && dolt commit -m "Import calculator effect source workbooks"'
```

### 4. Validate the imported surfaces

Recommended spot checks:

```bash
devenv shell -- bash -lc 'dolt sql -q "select count(*) from calculator_lightstone_set_effects"'
devenv shell -- bash -lc 'dolt sql -q "select count(*) from calculator_pet_skill_options"'
devenv shell -- bash -lc 'dolt sql -q "select count(*) from calculator_consumable_effects"'
```

The wide consumable view is still a temporary analysis surface. Direct
`item_table -> skill_table_new -> buff_table` checks for known items are still
useful when validating behavior.

## What Is Solid Already

These areas are already on a reasonable data path:

- zone catalog/search data from `zones_merged`
- zone drop structure from `fishing_zone_slots`
- subgroup expansion from `item_main_group_options` and
  `item_sub_group_item_variants`
- fish identity/icon data from `fish_table`
- consumables/event foods from the `item -> skill -> buff` chain
- lightstone set effects from `lightstone_set_option`
- pet skill/talent/special options from pet tables plus `skilltype_table_new`

## What Is Still Temporary

These calculator categories are still not cleanly sourced from the intermediate
tables:

- rods
- floats
- chairs
- outfit effects
- backpack effects

The current assumption is that these may still need temporary fallback values
from the prototype workbook
[`data/fishing_tables_101/Fishing Setup.xlsx`](/home/carp/code/fishystuff/data/fishing_tables_101/Fishing%20Setup.xlsx)
until their original or intermediate numeric effect source is mapped properly.

## Important Boundary

The legacy calculator `items` table should not be treated as the long-term
source of truth. It is a curated attachment layer and mixes real items with
synthetic effects.

The intended direction is:

- keep Dolt as the runtime source
- derive calculator-facing item/effect catalogs from imported source data
- retire the hand-maintained item attachment path

## Next Steps

1. Add narrower derived tables/views on top of the temporary raw effect tables.
2. Repoint calculator catalog endpoints away from the legacy `items` table.
3. Fold in unresolved gear categories as soon as their effect source is known.
4. Replace this temporary workbook import path with decoded original PAZ data.

For the schema workflow around these temporary tables/views, see
[`docs/dolt-schema-workflow.md`](/home/carp/code/fishystuff/docs/dolt-schema-workflow.md).
