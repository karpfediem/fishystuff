DROP VIEW IF EXISTS calculator_consumable_effect_sources;
CREATE VIEW calculator_consumable_effect_sources AS
SELECT base.source_key,
       base.item_id,
       base.item_name_ko,
       base.item_icon_file,
       base.item_description_ko,
       effect_texts.effect_description_ko
FROM (
  SELECT source_key,
         item_id,
         item_name_ko,
         item_icon_file,
         item_description_ko
  FROM calculator_consumable_effects
  GROUP BY source_key, item_id, item_name_ko, item_icon_file, item_description_ko
) base
LEFT JOIN (
  SELECT source_key,
         item_id,
         NULLIF(TRIM(GROUP_CONCAT(DISTINCT effect_line SEPARATOR '\n')), '') AS effect_description_ko
  FROM (
    SELECT source_key,
           item_id,
           NULLIF(TRIM(skill_description_ko), '') AS effect_line
    FROM calculator_consumable_effects
    UNION ALL
    SELECT source_key,
           item_id,
           NULLIF(TRIM(buff_description_ko), '') AS effect_line
    FROM calculator_consumable_effects
  ) effect_rows
  WHERE effect_line IS NOT NULL
  GROUP BY source_key, item_id
) effect_texts
  ON effect_texts.source_key = base.source_key
 AND effect_texts.item_id = base.item_id;

DROP VIEW IF EXISTS calculator_lightstone_effect_sources;
CREATE VIEW calculator_lightstone_effect_sources AS
SELECT source_key,
       lightstone_set_id,
       set_name_ko,
       CASE TRIM(COALESCE(set_name_ko, ''))
         WHEN '신의 입질' THEN 'Nibbles'
         WHEN '고래의 입' THEN 'Whaling'
         WHEN '예리한 갈매기' THEN 'Sharp-Eyed Seagull'
         WHEN '선택과 집중 : 낚시' THEN 'Choice & Focus: Fishing'
         WHEN '대장장이의 축복' THEN 'Blacksmith''s Blessing'
         ELSE NULL
       END AS legacy_name_en,
       effect_description_ko
FROM calculator_lightstone_set_effects
WHERE NULLIF(TRIM(COALESCE(set_name_ko, '')), '') IS NOT NULL;
