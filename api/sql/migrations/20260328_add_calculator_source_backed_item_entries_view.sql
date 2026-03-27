DROP VIEW IF EXISTS calculator_source_backed_item_entries;
CREATE VIEW calculator_source_backed_item_entries AS
SELECT effects.source_key,
       effects.source_kind,
       effects.item_id,
       COALESCE(legacy_item.type, 'buff') AS item_type,
       legacy_item.name AS legacy_name_en,
       effects.source_name_ko,
       source_meta.item_icon_file,
       legacy_item.icon_id AS legacy_icon_id,
       COALESCE(source_meta.endurance_limit, legacy_item.durability) AS durability,
       legacy_item.fish_multiplier,
       effects.effect_description_ko
FROM calculator_effect_source_entries effects
LEFT JOIN items legacy_item
  ON effects.source_kind = 'item'
 AND legacy_item.id = effects.item_id
 AND legacy_item.type IN ('food', 'buff')
LEFT JOIN calculator_item_source_metadata source_meta
  ON effects.source_kind = 'item'
 AND source_meta.item_id = effects.item_id
WHERE effects.source_kind = 'item'
UNION ALL
SELECT effects.source_key,
       effects.source_kind,
       NULL AS item_id,
       COALESCE(legacy_item.type, 'lightstone_set') AS item_type,
       effects.legacy_name_en AS legacy_name_en,
       effects.source_name_ko,
       NULL AS item_icon_file,
       legacy_item.icon_id AS legacy_icon_id,
       legacy_item.durability,
       legacy_item.fish_multiplier,
       effects.effect_description_ko
FROM calculator_effect_source_entries effects
LEFT JOIN items legacy_item
  ON effects.source_kind = 'lightstone_set'
 AND legacy_item.name = effects.legacy_name_en
 AND legacy_item.type = 'lightstone_set'
WHERE effects.source_kind = 'lightstone_set'
  AND effects.legacy_name_en IS NOT NULL;
