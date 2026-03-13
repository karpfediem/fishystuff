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
