DROP VIEW IF EXISTS calculator_fishing_effect_tooltips;
CREATE VIEW calculator_fishing_effect_tooltips AS
SELECT NULLIF(TRIM(`Key`), '') AS effect_macro,
       NULLIF(TRIM(`StringFormat`), '') AS tooltip_format_ko,
       CASE TRIM(COALESCE(`Key`, ''))
         WHEN 'AUTO_FISHING_REDUCE_TIME_DOWN' THEN 'afr'
         WHEN 'AUTO_FISHING_REDUCE_TIME_DOWN_2' THEN 'afr'
         WHEN 'AUTO_FISHING_REDUCE_TIME_DOWN_3' THEN 'afr'
         WHEN 'CHANCE_LARGE_SPECIES_FISH_INCRE' THEN 'bonus_big'
         WHEN 'CHANCE_RARE_SPECIES_FISH_INCRE' THEN 'bonus_rare'
         WHEN 'DUR_WEAPONS_CON_DOWN' THEN 'drr'
         WHEN 'FISHING_EXP_POINT_ADD' THEN 'exp_fish'
         WHEN 'LIFE_EXP_2' THEN 'exp_fish'
         WHEN 'FISHING_POINT' THEN 'fishing_potential_stage'
         WHEN 'LIFESTAT_FISHING_HOE' THEN 'fishing_mastery'
         WHEN 'LIFESTAT_FISHING_ALL_ADD' THEN 'fishing_mastery'
         WHEN 'FISHING_SIT_EFFECT_NORMAL' THEN 'fishing_mastery'
         WHEN 'FISHING_SIT_EFFECT_NORMAL2' THEN 'fishing_mastery'
         WHEN 'FISHING_SIT_EFFECT_NORMAL_44' THEN 'mixed'
         ELSE NULL
       END AS metric_kind,
       CASE
         WHEN TRIM(COALESCE(`Key`, '')) IN (
           'AUTO_FISHING_REDUCE_TIME_DOWN_3',
           'FISHING_SIT_EFFECT_NORMAL',
           'FISHING_SIT_EFFECT_NORMAL2',
           'FISHING_SIT_EFFECT_NORMAL_44'
         ) THEN 0
         ELSE 1
       END AS has_numeric_param
FROM tooltip_table
WHERE TRIM(COALESCE(`Key`, '')) IN (
  'AUTO_FISHING_REDUCE_TIME_DOWN',
  'AUTO_FISHING_REDUCE_TIME_DOWN_2',
  'AUTO_FISHING_REDUCE_TIME_DOWN_3',
  'CHANCE_LARGE_SPECIES_FISH_INCRE',
  'CHANCE_RARE_SPECIES_FISH_INCRE',
  'DUR_WEAPONS_CON_DOWN',
  'FISHING_EXP_POINT_ADD',
  'LIFE_EXP_2',
  'FISHING_POINT',
  'LIFESTAT_FISHING_HOE',
  'LIFESTAT_FISHING_ALL_ADD',
  'FISHING_SIT_EFFECT_NORMAL',
  'FISHING_SIT_EFFECT_NORMAL2',
  'FISHING_SIT_EFFECT_NORMAL_44'
);

DROP VIEW IF EXISTS calculator_enchant_fishing_item_sources;
CREATE VIEW calculator_enchant_fishing_item_sources AS
SELECT 'equipment' AS source_sheet,
       CASE
         WHEN COALESCE(`ItemName`, '') LIKE '%낚싯대%' THEN 'rod'
         WHEN COALESCE(`ItemName`, '') LIKE '%찌%' THEN 'float'
         WHEN COALESCE(`ItemName`, '') LIKE '%배낭%' THEN 'backpack'
         WHEN COALESCE(`ItemName`, '') LIKE '%낚시복%' OR COALESCE(`ItemName`, '') LIKE '%낚시모자%' THEN 'outfit'
         WHEN COALESCE(`ItemName`, '') LIKE '%낚시%' THEN 'fishing_item'
         ELSE 'fishing_item'
       END AS item_type_hint,
       CAST(`Index` AS SIGNED) AS source_item_key,
       NULLIF(TRIM(`ItemName`), '') AS item_name_ko,
       NULLIF(TRIM(`Enchant`), '') AS enchant_level,
       CASE
         WHEN TRIM(COALESCE(`EnduranceLimit`, '')) REGEXP '^[0-9]+$'
           THEN CAST(`EnduranceLimit` AS SIGNED)
         ELSE NULL
       END AS endurance_limit,
       CASE
         WHEN TRIM(COALESCE(`SkillNo`, '')) REGEXP '^[0-9]+$'
           THEN CAST(`SkillNo` AS SIGNED)
         ELSE NULL
       END AS skill_no,
       NULLIF(TRIM(`PatternDescription`), '') AS pattern_description,
       NULLIF(TRIM(`Description`), '') AS description_ko
FROM enchant_equipment
WHERE COALESCE(`ItemName`, '') LIKE '%낚시%'
   OR COALESCE(`ItemName`, '') LIKE '%찌%'
   OR COALESCE(`PatternDescription`, '') LIKE '%AUTO_FISHING_REDUCE_TIME_DOWN(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%AUTO_FISHING_REDUCE_TIME_DOWN_2(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%AUTO_FISHING_REDUCE_TIME_DOWN_3();%'
   OR COALESCE(`PatternDescription`, '') LIKE '%CHANCE_LARGE_SPECIES_FISH_INCRE(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%CHANCE_RARE_SPECIES_FISH_INCRE(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%LIFESTAT_FISHING_HOE(%'
UNION ALL
SELECT 'lifeequipment' AS source_sheet,
       CASE
         WHEN COALESCE(`ItemName`, '') LIKE '%의자%' THEN 'chair'
         WHEN COALESCE(`ItemName`, '') LIKE '%낚시복%' THEN 'outfit'
         WHEN COALESCE(`ItemName`, '') LIKE '%낚싯대%' THEN 'rod'
         WHEN COALESCE(`ItemName`, '') LIKE '%찌%' THEN 'float'
         WHEN COALESCE(`ItemName`, '') LIKE '%배낭%' THEN 'backpack'
         ELSE 'fishing_item'
       END AS item_type_hint,
       CAST(`Index` AS SIGNED) AS source_item_key,
       NULLIF(TRIM(`ItemName`), '') AS item_name_ko,
       NULLIF(TRIM(`Enchant`), '') AS enchant_level,
       CASE
         WHEN TRIM(COALESCE(`EnduranceLimit`, '')) REGEXP '^[0-9]+$'
           THEN CAST(`EnduranceLimit` AS SIGNED)
         ELSE NULL
       END AS endurance_limit,
       CASE
         WHEN TRIM(COALESCE(`SkillNo`, '')) REGEXP '^[0-9]+$'
           THEN CAST(`SkillNo` AS SIGNED)
         ELSE NULL
       END AS skill_no,
       NULLIF(TRIM(`PatternDescription`), '') AS pattern_description,
       NULLIF(TRIM(`Description`), '') AS description_ko
FROM enchant_lifeequipment
WHERE COALESCE(`ItemName`, '') LIKE '%낚시%'
   OR COALESCE(`PatternDescription`, '') LIKE '%FISHING_SIT_EFFECT_%'
   OR COALESCE(`PatternDescription`, '') LIKE '%LIFESTAT_FISHING_HOE(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%LIFESTAT_FISHING_ALL_ADD(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%FISHING_POINT(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%FISHING_EXP_POINT_ADD(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%AUTO_FISHING_REDUCE_TIME_DOWN(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%AUTO_FISHING_REDUCE_TIME_DOWN_2(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%AUTO_FISHING_REDUCE_TIME_DOWN_3();%'
UNION ALL
SELECT 'cash' AS source_sheet,
       CASE
         WHEN COALESCE(`ItemName`, '') LIKE '%배낭%' THEN 'backpack'
         WHEN COALESCE(`ItemName`, '') LIKE '%낚시모자%' OR COALESCE(`ItemName`, '') LIKE '%낚시복%' THEN 'outfit'
         WHEN COALESCE(`ItemName`, '') LIKE '%낚싯대%' THEN 'rod'
         WHEN COALESCE(`ItemName`, '') LIKE '%찌%' THEN 'float'
         WHEN COALESCE(`ItemName`, '') LIKE '%낚시%' THEN 'fishing_item'
         ELSE 'fishing_item'
       END AS item_type_hint,
       CAST(`Index` AS SIGNED) AS source_item_key,
       NULLIF(TRIM(`ItemName`), '') AS item_name_ko,
       NULLIF(TRIM(`Enchant`), '') AS enchant_level,
       CASE
         WHEN TRIM(COALESCE(`EnduranceLimit`, '')) REGEXP '^[0-9]+$'
           THEN CAST(`EnduranceLimit` AS SIGNED)
         ELSE NULL
       END AS endurance_limit,
       CASE
         WHEN TRIM(COALESCE(`SkillNo`, '')) REGEXP '^[0-9]+$'
           THEN CAST(`SkillNo` AS SIGNED)
         ELSE NULL
       END AS skill_no,
       NULLIF(TRIM(`PatternDescription`), '') AS pattern_description,
       NULLIF(TRIM(`Description`), '') AS description_ko
FROM enchant_cash
WHERE COALESCE(`ItemName`, '') LIKE '%낚시%'
   OR COALESCE(`PatternDescription`, '') LIKE '%LIFESTAT_FISHING_HOE(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%LIFESTAT_FISHING_ALL_ADD(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%FISHING_POINT(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%FISHING_EXP_POINT_ADD(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%AUTO_FISHING_REDUCE_TIME_DOWN(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%AUTO_FISHING_REDUCE_TIME_DOWN_3();%'
   OR COALESCE(`PatternDescription`, '') LIKE '%DUR_WEAPONS_CON_DOWN(%';

DROP VIEW IF EXISTS calculator_enchant_fishing_item_effects;
CREATE VIEW calculator_enchant_fishing_item_effects AS
SELECT src.source_sheet,
       src.item_type_hint,
       src.source_item_key,
       src.item_name_ko,
       src.enchant_level,
       src.skill_no,
       src.endurance_limit,
       src.pattern_description,
       NULLIF(
         CASE
           WHEN COALESCE(src.pattern_description, '') LIKE '%FISHING_POINT(%'
             THEN CAST(
               SUBSTRING_INDEX(
                 SUBSTRING(
                   src.pattern_description,
                   LOCATE('FISHING_POINT(', src.pattern_description) + CHAR_LENGTH('FISHING_POINT(')
                 ),
                 ')',
                 1
               ) AS DECIMAL(10, 4)
             )
           ELSE 0
         END,
         0
       ) AS fishing_speed_stage,
       NULLIF(
         (
           CASE
             WHEN COALESCE(src.pattern_description, '') LIKE '%LIFESTAT_FISHING_HOE(%'
               THEN CAST(
                 SUBSTRING_INDEX(
                   SUBSTRING(
                     src.pattern_description,
                     LOCATE('LIFESTAT_FISHING_HOE(', src.pattern_description)
                       + CHAR_LENGTH('LIFESTAT_FISHING_HOE(')
                   ),
                   ')',
                   1
                 ) AS DECIMAL(10, 4)
               )
             ELSE 0
           END
           + CASE
               WHEN COALESCE(src.pattern_description, '') LIKE '%LIFESTAT_FISHING_ALL_ADD(%'
                 THEN CAST(
                   SUBSTRING_INDEX(
                     SUBSTRING(
                       src.pattern_description,
                       LOCATE('LIFESTAT_FISHING_ALL_ADD(', src.pattern_description)
                         + CHAR_LENGTH('LIFESTAT_FISHING_ALL_ADD(')
                     ),
                     ')',
                     1
                   ) AS DECIMAL(10, 4)
                 )
               ELSE 0
             END
           + CASE
               WHEN COALESCE(src.pattern_description, '') LIKE '%FISHING_SIT_EFFECT_NORMAL_44();%' THEN 100
               ELSE 0
             END
           + CASE
               WHEN COALESCE(src.pattern_description, '') LIKE '%FISHING_SIT_EFFECT_NORMAL2();%' THEN 220
               ELSE 0
             END
           + CASE
               WHEN COALESCE(src.pattern_description, '') LIKE '%FISHING_SIT_EFFECT_NORMAL();%' THEN 100
               ELSE 0
             END
         ),
         0
       ) AS fishing_mastery,
       NULLIF(
         (
           CASE
             WHEN COALESCE(src.pattern_description, '') LIKE '%AUTO_FISHING_REDUCE_TIME_DOWN(%'
               THEN CAST(
                 SUBSTRING_INDEX(
                   SUBSTRING(
                     src.pattern_description,
                     LOCATE('AUTO_FISHING_REDUCE_TIME_DOWN(', src.pattern_description)
                       + CHAR_LENGTH('AUTO_FISHING_REDUCE_TIME_DOWN(')
                   ),
                   ')',
                   1
                 ) AS DECIMAL(10, 4)
               ) / 100.0
             ELSE 0
           END
           + CASE
               WHEN COALESCE(src.pattern_description, '') LIKE '%AUTO_FISHING_REDUCE_TIME_DOWN_2(%'
                 THEN CAST(
                   SUBSTRING_INDEX(
                     SUBSTRING(
                       src.pattern_description,
                       LOCATE('AUTO_FISHING_REDUCE_TIME_DOWN_2(', src.pattern_description)
                         + CHAR_LENGTH('AUTO_FISHING_REDUCE_TIME_DOWN_2(')
                     ),
                     ')',
                     1
                   ) AS DECIMAL(10, 4)
                 ) / 100.0
               ELSE 0
             END
           + CASE
               WHEN COALESCE(src.pattern_description, '') LIKE '%AUTO_FISHING_REDUCE_TIME_DOWN_3();%' THEN 0.8
               ELSE 0
             END
           + CASE
               WHEN COALESCE(src.pattern_description, '') LIKE '%FISHING_SIT_EFFECT_NORMAL_44();%' THEN 0.05
               ELSE 0
             END
         ),
         0
       ) AS afr,
       NULLIF(
         CASE
           WHEN COALESCE(src.pattern_description, '') LIKE '%CHANCE_LARGE_SPECIES_FISH_INCRE(%'
             THEN CAST(
               SUBSTRING_INDEX(
                 SUBSTRING(
                   src.pattern_description,
                   LOCATE('CHANCE_LARGE_SPECIES_FISH_INCRE(', src.pattern_description)
                     + CHAR_LENGTH('CHANCE_LARGE_SPECIES_FISH_INCRE(')
                 ),
                 ')',
                 1
               ) AS DECIMAL(10, 4)
             ) / 100.0
           ELSE 0
         END,
         0
       ) AS bonus_big,
       NULLIF(
         CASE
           WHEN COALESCE(src.pattern_description, '') LIKE '%CHANCE_RARE_SPECIES_FISH_INCRE(%'
             THEN CAST(
               SUBSTRING_INDEX(
                 SUBSTRING(
                   src.pattern_description,
                   LOCATE('CHANCE_RARE_SPECIES_FISH_INCRE(', src.pattern_description)
                     + CHAR_LENGTH('CHANCE_RARE_SPECIES_FISH_INCRE(')
                 ),
                 ')',
                 1
               ) AS DECIMAL(10, 4)
             ) / 100.0
           ELSE 0
         END,
         0
       ) AS bonus_rare,
       NULLIF(
         CASE
           WHEN COALESCE(src.pattern_description, '') LIKE '%DUR_WEAPONS_CON_DOWN(%'
             THEN CAST(
               SUBSTRING_INDEX(
                 SUBSTRING(
                   src.pattern_description,
                   LOCATE('DUR_WEAPONS_CON_DOWN(', src.pattern_description)
                     + CHAR_LENGTH('DUR_WEAPONS_CON_DOWN(')
                 ),
                 ')',
                 1
               ) AS DECIMAL(10, 4)
             ) / 100.0
           ELSE 0
         END,
         0
       ) AS drr,
       NULLIF(
         (
           CASE
             WHEN COALESCE(src.pattern_description, '') LIKE '%FISHING_EXP_POINT_ADD(%'
               THEN CAST(
                 SUBSTRING_INDEX(
                   SUBSTRING(
                     src.pattern_description,
                     LOCATE('FISHING_EXP_POINT_ADD(', src.pattern_description)
                       + CHAR_LENGTH('FISHING_EXP_POINT_ADD(')
                   ),
                   ')',
                   1
                 ) AS DECIMAL(10, 4)
               ) / 100.0
             ELSE 0
           END
           + CASE
               WHEN COALESCE(src.pattern_description, '') LIKE '%LIFE_EXP_2(%'
                 THEN CAST(
                   SUBSTRING_INDEX(
                     SUBSTRING(
                       src.pattern_description,
                       LOCATE('LIFE_EXP_2(', src.pattern_description) + CHAR_LENGTH('LIFE_EXP_2(')
                     ),
                     ')',
                     1
                   ) AS DECIMAL(10, 4)
                 ) / 100.0
               ELSE 0
             END
         ),
         0
       ) AS exp_fish,
       producttool.autofishing_time_reduction AS producttool_afr
FROM calculator_enchant_fishing_item_sources src
LEFT JOIN calculator_fishing_producttool_properties producttool
  ON producttool.source_item_key = src.source_item_key
 AND COALESCE(producttool.enchant_level, '') = COALESCE(src.enchant_level, '');

DROP VIEW IF EXISTS calculator_enchant_item_effect_entries;
CREATE VIEW calculator_enchant_item_effect_entries AS
SELECT CONCAT('enchant-source:', CAST(effects.source_item_key AS CHAR)) AS source_key,
       effects.source_item_key,
       NULL AS item_id,
       effects.item_type_hint AS item_type,
       effects.item_name_ko,
       effects.enchant_level,
       NULLIF(effects.endurance_limit, 0) AS durability,
       effects.afr AS afr,
       effects.bonus_rare,
       effects.bonus_big,
       effects.drr,
       effects.exp_fish,
       effects.fishing_mastery,
       effects.fishing_speed_stage,
       effects.pattern_description
FROM calculator_enchant_fishing_item_effects effects
WHERE COALESCE(
        effects.afr,
        effects.bonus_rare,
        effects.bonus_big,
        effects.drr,
        effects.exp_fish,
        effects.fishing_mastery,
        effects.fishing_speed_stage
      ) IS NOT NULL;

DROP VIEW IF EXISTS calculator_item_source_metadata;
CREATE VIEW calculator_item_source_metadata AS
SELECT CAST(`Index` AS SIGNED) AS item_id,
       NULLIF(TRIM(`ItemName`), '') AS item_name_ko,
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
         NULLIF(TRIM(it.`IconImageFile`), ''),
         CASE
           WHEN TRIM(COALESCE(it.`EnduranceLimit`, '')) REGEXP '^[0-9]+$'
             THEN CAST(it.`EnduranceLimit` AS SIGNED)
           ELSE NULL
         END;

DROP VIEW IF EXISTS calculator_source_owned_enchant_item_effect_entries;
CREATE VIEW calculator_source_owned_enchant_item_effect_entries AS
SELECT CONCAT('item:', CAST(meta.item_id AS CHAR)) AS source_key,
       meta.item_id,
       chosen.item_type,
       meta.item_name_en AS source_name_en,
       meta.item_name_ko AS source_name_ko,
       meta.item_icon_file,
       COALESCE(chosen.durability, meta.endurance_limit) AS durability,
       chosen.afr,
       chosen.bonus_rare,
       chosen.bonus_big,
       chosen.drr,
       chosen.exp_fish,
       NULL AS exp_life
FROM (
  SELECT picked.item_name_ko,
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
    WHERE item_type IN ('rod', 'float', 'chair', 'backpack')
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
  WHERE picked.item_type IN ('rod', 'float', 'chair', 'backpack')
  GROUP BY picked.item_name_ko, picked.item_type
) chosen
JOIN calculator_item_source_metadata meta
  ON meta.item_name_ko = chosen.item_name_ko;

DROP VIEW IF EXISTS calculator_lightstone_effect_sources;
CREATE VIEW calculator_lightstone_effect_sources AS
SELECT source_key,
       lightstone_set_id,
       set_name_ko,
       NULL AS legacy_name_en,
       NULL AS source_name_en,
       skill_icon_file,
       effect_description_ko
FROM calculator_lightstone_set_effects
WHERE NULLIF(TRIM(COALESCE(set_name_ko, '')), '') IS NOT NULL;
