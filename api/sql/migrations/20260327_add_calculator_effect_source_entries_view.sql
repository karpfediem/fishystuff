DROP VIEW IF EXISTS calculator_effect_source_entries;
CREATE VIEW calculator_effect_source_entries AS
SELECT source_key,
       'item' AS source_kind,
       item_id,
       NULL AS legacy_name_en,
       item_name_ko AS source_name_ko,
       COALESCE(effect_description_ko, item_description_ko) AS effect_description_ko
FROM calculator_consumable_effect_sources
UNION ALL
SELECT source_key,
       'lightstone_set' AS source_kind,
       NULL AS item_id,
       legacy_name_en,
       set_name_ko AS source_name_ko,
       effect_description_ko
FROM calculator_lightstone_effect_sources
WHERE legacy_name_en IS NOT NULL;
