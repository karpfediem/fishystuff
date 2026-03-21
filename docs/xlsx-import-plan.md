# XLSX Import Plan (Exploration)

Date: 2026-02-26

## Workbooks discovered

- Fishing_Table.xlsx
  - Sheet: Fishing_Table
  - Rows ~277, Cols 18
  - SHA256: ddd912a2f48a7e77edeebe9e1df98881a959ca297bbf774de1f2e617d4ba75ec
- ItemMainGroup_Table.xlsx
  - Sheet: ItemMainGroup_Table
  - Rows ~22976, Cols 17
  - SHA256: 70ca0f0983f4e3ebd8d33334bd1c3e3f3e39f6a914d571a87795876ec0fb8a39
- ItemSubGroup_Table.xlsx
  - Sheet: ItemSubGroup_Table
  - Rows ~129083, Cols 19
  - SHA256: 94782e79a387d075533503ee0509772c1570ed1964176b733f3b9954c7e83f54

Other artifact:
- FishingTables.zip (not expanded/inspected in this pass)

Additional local legacy dataset:

- `data/data/excel/Fishing_Table.xlsx`
- `data/data/excel/ItemMainGroup_Table.xlsx`
- `data/data/excel/ItemSubGroup_Table.xlsx`
- `data/data/excel/Item_Table.xlsx`
- auxiliary fishing-adjacent workbooks such as `FloatFishing_Table.xlsx`, `FloatFishingPoint_Table.xlsx`, `Water_Table.xlsx`, `DiscardFish.xlsx`, and `FishingStatData.xlsx`

## Fishing-related sheets and candidate keys

- Fishing_Table (Fishing_Table.xlsx)
  - Candidate zone key: R, G, B (RGB color tuple)
  - Candidate group keys: DropID, DropIDHarpoon, DropIDNet
  - Candidate rate columns: DropRate1..DropRate5
  - Candidate subgroup refs: DropID1..DropID5
  - Timing: MinWaitTime, MaxWaitTime

- ItemMainGroup_Table (ItemMainGroup_Table.xlsx)
  - Primary key: ItemMainGroupKey
  - Candidate subgroup keys: ItemSubGroupKey0..ItemSubGroupKey3
  - Candidate rates/weights: SelectRate0..SelectRate3
  - Conditions: Condition0..Condition3
  - Other flags: DoSelectOnlyOne, RefreshStartHour, RefreshInterval

- ItemSubGroup_Table (ItemSubGroup_Table.xlsx)
  - Primary/foreign key: ItemSubGroupKey
  - Item key: ItemKey
  - Rates/quantities: SelectRate_0..SelectRate_2, MinCount_0..2, MaxCount_0..2
  - Modifiers/flags: EnchantLevel, DoPetAddDrop, DoSechiAddDrop
  - Pricing: ApplyRandomPrice, RentTime, PriceOption

## Header language notes

- All observed headers are English; no Korean column names found in the inspected sheets.

## Reports

- JSON reports live in data/import_reports/ keyed by the source file SHA256.

## Current durable conclusions

- The raw legacy XLSX dump under `data/data/excel/` is the durable source-schema backbone for:
  - `fishing_table`
  - `item_main_group_table`
  - `item_sub_group_table`
  - `item_table`
- `fishing_table` should remain the legacy RGB-to-slot baseline.
- `item_main_group_table` and `item_sub_group_table` are the correct merge targets for subgroup-resolution enrichment.
- The raw `ItemSubGroup_Table.xlsx` layout is structurally correct, but its `SelectRate_0..2` values are not sufficient on their own for usable subgroup item expansion in Dolt.
- Maintained import work should prefer enriching group tables over rewriting existing `fishing_table` RGB rows.

## Maintained import boundary

For durable import work:

1. Import the raw legacy zone-slot layer into `fishing_table`.
2. Preserve `fishing_table` row identities keyed by `R,G,B`.
3. Import raw legacy main/subgroup rows into `item_main_group_table` and `item_sub_group_table`.
4. Apply any later subgroup-baseline enrichment at the group-table layer, not by bulk-overwriting `fishing_table`.
5. Exclude user-entered placeholder group keys from maintained imports.

## Runtime state after subgroup baseline backfill

- `fishing_table`: `276` rows
- `item_main_group_table`: `405` rows
- `item_sub_group_table`: `1676` rows
- `item_main_group_options`: `469` rows
- `item_sub_group_item_variants`: `1330` rows

This means the local runtime can now resolve subgroup item variants, but the maintained source contract is still:

- raw legacy XLSX for the schema backbone
- explicit enrichment of group tables
- no blanket `fishing_table` rewrite
