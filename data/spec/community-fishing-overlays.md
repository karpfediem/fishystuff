# Community Fishing Overlays

Community-maintained fishing workbooks are imported as explicit overlays over
source-backed game tables. Keep the workbook itself as local developer input
under `data/`, and keep committed code/docs focused on the import contract.

## Subgroup Overlay

Current import-facing source:

- `data/data/Subgroups(no formulas).xlsx`
- sheet: `no formulas`
- expected source id: `community_subgroups_no_formulas_workbook`

The `no formulas` sheet in `data/data/Merged fish worksheet.xlsx` has the
same normalized overlay and unresolved-row semantics, but the standalone
workbook is cleaner for imports. Do not import the formula-backed `Subgroup`
sheet directly; it contains repeated dummy formula rows that are intentionally
not part of the overlay contract.

Use:

```sh
cargo run -p fishystuff_dolt_import -- import-community-subgroup-overlay-xlsx \
  --dolt-repo . \
  --subgroups-xlsx "data/data/Subgroups(no formulas).xlsx" \
  --emit-only
```

Remove `--emit-only` to import the overlay table. Add `--activate` to activate
the source for `item_sub_group_item_variants`.

The importer stores row-level provenance in `community_item_sub_group_overlay`:

- `source_id`, `source_label`, `source_sha256`
- `source_sheet`, `source_row`
- contributor-facing metadata such as grade, fish name, and `removed`/`added`
- normalized `item_sub_group_table` columns used by runtime views

Rows that cannot be normalized to the numeric
`ItemSubGroupKey`/`ItemKey`/`EnchantLevel` primary key are preserved in
`community_item_sub_group_unresolved_overlay` instead of being silently
dropped. This includes symbolic subgroup keys such as future/community labels
that need a later resolution step. The unresolved table keeps the source hash,
sheet, row number, reason, raw key cells, contributor metadata, and raw rate
cells.

Activation records the active subgroup overlay source in
`community_active_overlays`. The effective subgroup view keeps original
`item_sub_group_table` rows unless the active overlay provides the same
`ItemSubGroupKey`/`ItemKey`/`EnchantLevel` key. Active overlay rows marked
`source_removed = 1` suppress the matching original row; active rows not marked
removed are included.

This preserves the original table, the community source hash, and the specific
source row that produced each overlay row.
