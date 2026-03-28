DROP VIEW IF EXISTS calculator_source_owned_enchant_item_effect_entries;
DROP VIEW IF EXISTS calculator_legacy_aligned_enchant_item_effect_entries;
DROP VIEW IF EXISTS calculator_source_backed_item_entries;
DROP VIEW IF EXISTS calculator_effect_source_entries;
DROP VIEW IF EXISTS calculator_lightstone_effect_sources;
DROP VIEW IF EXISTS calculator_item_source_metadata;

CREATE VIEW calculator_item_source_metadata AS
SELECT CAST(`Index` AS SIGNED) AS item_id,
       NULLIF(TRIM(`ItemName`), '') AS item_name_ko,
       TRIM(
         REPLACE(
           REPLACE(
             REPLACE(
               REPLACE(
                 REPLACE(NULLIF(TRIM(`ItemName`), ''), '[의상] ', ''),
                 '[이벤트] ',
                 ''
               ),
               '의 낚시 배낭',
               ' 낚시 배낭'
             ),
             '의 낚시복',
             ' 낚시복'
           ),
           '  ',
           ' '
         )
       ) AS normalized_item_name_ko,
       MAX(
         CASE
           WHEN COALESCE(l.`format`, '') = 'A'
             AND COALESCE(l.`unk`, '') = ''
             THEN NULLIF(TRIM(l.`text`), '')
           ELSE NULL
         END
       ) AS item_name_en,
       NULLIF(TRIM(`IconImageFile`), '') AS item_icon_file,
       CASE
         WHEN TRIM(COALESCE(`EnduranceLimit`, '')) REGEXP '^[0-9]+$'
           THEN CAST(`EnduranceLimit` AS SIGNED)
         ELSE NULL
       END AS endurance_limit
FROM item_table it
LEFT JOIN languagedata_en l
  ON l.`id` = CAST(it.`Index` AS SIGNED)
GROUP BY CAST(it.`Index` AS SIGNED),
         NULLIF(TRIM(it.`ItemName`), ''),
         TRIM(
           REPLACE(
             REPLACE(
               REPLACE(
                 REPLACE(
                   REPLACE(NULLIF(TRIM(it.`ItemName`), ''), '[의상] ', ''),
                   '[이벤트] ',
                   ''
                 ),
                 '의 낚시 배낭',
                 ' 낚시 배낭'
               ),
               '의 낚시복',
               ' 낚시복'
             ),
             '  ',
             ' '
           )
         ),
         NULLIF(TRIM(it.`IconImageFile`), ''),
         CASE
           WHEN TRIM(COALESCE(it.`EnduranceLimit`, '')) REGEXP '^[0-9]+$'
             THEN CAST(it.`EnduranceLimit` AS SIGNED)
           ELSE NULL
         END;

CREATE VIEW calculator_source_owned_enchant_item_effect_entries AS
SELECT CONCAT(
         'item:',
         CAST(COALESCE(meta_exact.item_id, meta_fallback.item_id) AS CHAR)
       ) AS source_key,
       COALESCE(meta_exact.item_id, meta_fallback.item_id) AS item_id,
       chosen.item_type,
       COALESCE(meta_exact.item_name_en, meta_fallback.item_name_en) AS source_name_en,
       COALESCE(meta_exact.item_name_ko, meta_fallback.item_name_ko) AS source_name_ko,
       COALESCE(meta_exact.item_icon_file, meta_fallback.item_icon_file) AS item_icon_file,
       COALESCE(
         chosen.durability,
         meta_exact.endurance_limit,
         meta_fallback.endurance_limit
       ) AS durability,
       chosen.afr,
       chosen.bonus_rare,
       chosen.bonus_big,
       chosen.drr,
       chosen.exp_fish,
       NULL AS exp_life
FROM (
  SELECT picked.item_name_ko,
         TRIM(
           REPLACE(
             REPLACE(
               REPLACE(
                 REPLACE(
                   REPLACE(picked.item_name_ko, '[의상] ', ''),
                   '[이벤트] ',
                   ''
                 ),
                 '의 낚시 배낭',
                 ' 낚시 배낭'
               ),
               '의 낚시복',
               ' 낚시복'
             ),
             '  ',
             ' '
           )
         ) AS normalized_item_name_ko,
         picked.item_type,
         MAX(picked.durability) AS durability,
         MAX(picked.afr) AS afr,
         MAX(picked.bonus_rare) AS bonus_rare,
         MAX(picked.bonus_big) AS bonus_big,
         MAX(picked.drr) AS drr,
         MAX(picked.exp_fish) AS exp_fish
  FROM calculator_enchant_item_effect_entries picked
  JOIN (
    SELECT item_name_ko,
           item_type,
           MAX(
             CASE
               WHEN COALESCE(enchant_level, '') REGEXP '^-?[0-9]+$'
                 THEN CAST(enchant_level AS SIGNED)
               ELSE 0
             END
           ) AS best_enchant
    FROM calculator_enchant_item_effect_entries
    WHERE item_type IN ('rod', 'float', 'chair', 'backpack', 'outfit')
    GROUP BY item_name_ko, item_type
  ) best
    ON best.item_name_ko = picked.item_name_ko
   AND best.item_type = picked.item_type
   AND (
     CASE
       WHEN COALESCE(picked.enchant_level, '') REGEXP '^-?[0-9]+$'
         THEN CAST(picked.enchant_level AS SIGNED)
       ELSE 0
     END
   ) = best.best_enchant
  WHERE picked.item_type IN ('rod', 'float', 'chair', 'backpack', 'outfit')
  GROUP BY picked.item_name_ko, picked.item_type
) chosen
LEFT JOIN calculator_item_source_metadata meta_exact
  ON meta_exact.item_name_ko = chosen.item_name_ko
LEFT JOIN (
  SELECT normalized_item_name_ko,
         MIN(item_id) AS item_id
  FROM calculator_item_source_metadata
  GROUP BY normalized_item_name_ko
  HAVING COUNT(*) = 1
) fallback_item
  ON meta_exact.item_id IS NULL
 AND fallback_item.normalized_item_name_ko = chosen.normalized_item_name_ko
LEFT JOIN calculator_item_source_metadata meta_fallback
  ON meta_fallback.item_id = fallback_item.item_id
WHERE COALESCE(meta_exact.item_id, meta_fallback.item_id) IS NOT NULL;

CREATE VIEW calculator_lightstone_effect_sources AS
SELECT source_key,
       lightstone_set_id,
       set_name_ko,
       NULL AS source_name_en,
       skill_icon_file,
       effect_description_ko
FROM calculator_lightstone_set_effects
WHERE NULLIF(TRIM(COALESCE(set_name_ko, '')), '') IS NOT NULL;
