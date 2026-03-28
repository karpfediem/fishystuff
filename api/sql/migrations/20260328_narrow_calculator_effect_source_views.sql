DROP VIEW IF EXISTS calculator_consumable_item_targets;
CREATE VIEW calculator_consumable_item_targets AS
SELECT CAST(id AS SIGNED) AS item_id,
       type AS item_type,
       name AS legacy_name_en
FROM items
WHERE id IS NOT NULL
  AND type IN ('food', 'buff');

DROP VIEW IF EXISTS calculator_item_skill_sources;
CREATE VIEW calculator_item_skill_sources AS
SELECT CONCAT('item:', CAST(target.item_id AS CHAR)) AS source_key,
       target.item_id,
       NULLIF(TRIM(it.`ItemName`), '') AS item_name_ko,
       NULLIF(TRIM(it.`IconImageFile`), '') AS item_icon_file,
       NULLIF(TRIM(it.`Description`), '') AS item_description_ko,
       'skill' AS skill_source,
       TRIM(it.`SkillNo`) AS skill_no
FROM calculator_consumable_item_targets target
JOIN item_table it
  ON CAST(it.`Index` AS SIGNED) = target.item_id
WHERE TRIM(COALESCE(it.`SkillNo`, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT CONCAT('item:', CAST(target.item_id AS CHAR)),
       target.item_id,
       NULLIF(TRIM(it.`ItemName`), ''),
       NULLIF(TRIM(it.`IconImageFile`), ''),
       NULLIF(TRIM(it.`Description`), ''),
       'sub_skill',
       TRIM(it.`SubSkillNo`)
FROM calculator_consumable_item_targets target
JOIN item_table it
  ON CAST(it.`Index` AS SIGNED) = target.item_id
WHERE TRIM(COALESCE(it.`SubSkillNo`, '')) REGEXP '^[0-9]+$';

DROP VIEW IF EXISTS calculator_target_skill_nos;
CREATE VIEW calculator_target_skill_nos AS
SELECT DISTINCT skill_no
FROM calculator_item_skill_sources
WHERE TRIM(COALESCE(skill_no, '')) REGEXP '^[0-9]+$';

DROP VIEW IF EXISTS calculator_target_skill_rows;
CREATE VIEW calculator_target_skill_rows AS
SELECT TRIM(sk.`SkillNo`) AS skill_no,
       TRIM(sk.`SkillLevel`) AS skill_level,
       TRIM(sk.`Buff0`) AS buff0,
       TRIM(sk.`Buff1`) AS buff1,
       TRIM(sk.`Buff2`) AS buff2,
       TRIM(sk.`Buff3`) AS buff3,
       TRIM(sk.`Buff4`) AS buff4,
       TRIM(sk.`Buff5`) AS buff5,
       TRIM(sk.`Buff6`) AS buff6,
       TRIM(sk.`Buff7`) AS buff7,
       TRIM(sk.`Buff8`) AS buff8,
       TRIM(sk.`Buff9`) AS buff9
FROM skill_table_new sk
JOIN calculator_target_skill_nos target
  ON TRIM(COALESCE(sk.`SkillNo`, '')) = target.skill_no
WHERE TRIM(COALESCE(sk.`SkillLevel`, '')) REGEXP '^[0-9]+$';

DROP VIEW IF EXISTS calculator_skill_buffs;
CREATE VIEW calculator_skill_buffs AS
SELECT sk.skill_no,
       sk.skill_level,
       0 AS buff_slot,
       sk.buff0 AS buff_id
FROM calculator_target_skill_rows sk
WHERE TRIM(COALESCE(sk.buff0, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT sk.skill_no, sk.skill_level, 1, sk.buff1
FROM calculator_target_skill_rows sk
WHERE TRIM(COALESCE(sk.buff1, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT sk.skill_no, sk.skill_level, 2, sk.buff2
FROM calculator_target_skill_rows sk
WHERE TRIM(COALESCE(sk.buff2, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT sk.skill_no, sk.skill_level, 3, sk.buff3
FROM calculator_target_skill_rows sk
WHERE TRIM(COALESCE(sk.buff3, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT sk.skill_no, sk.skill_level, 4, sk.buff4
FROM calculator_target_skill_rows sk
WHERE TRIM(COALESCE(sk.buff4, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT sk.skill_no, sk.skill_level, 5, sk.buff5
FROM calculator_target_skill_rows sk
WHERE TRIM(COALESCE(sk.buff5, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT sk.skill_no, sk.skill_level, 6, sk.buff6
FROM calculator_target_skill_rows sk
WHERE TRIM(COALESCE(sk.buff6, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT sk.skill_no, sk.skill_level, 7, sk.buff7
FROM calculator_target_skill_rows sk
WHERE TRIM(COALESCE(sk.buff7, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT sk.skill_no, sk.skill_level, 8, sk.buff8
FROM calculator_target_skill_rows sk
WHERE TRIM(COALESCE(sk.buff8, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT sk.skill_no, sk.skill_level, 9, sk.buff9
FROM calculator_target_skill_rows sk
WHERE TRIM(COALESCE(sk.buff9, '')) REGEXP '^[0-9]+$';

DROP VIEW IF EXISTS calculator_target_skilltype_rows;
CREATE VIEW calculator_target_skilltype_rows AS
SELECT TRIM(stype.`SkillNo`) AS skill_no,
       NULLIF(TRIM(stype.`SkillName`), '') AS skill_name_ko,
       NULLIF(TRIM(stype.`Desc`), '') AS skill_description_ko
FROM skilltype_table_new stype
JOIN calculator_target_skill_nos target
  ON TRIM(COALESCE(stype.`SkillNo`, '')) = target.skill_no;

DROP VIEW IF EXISTS calculator_target_buff_ids;
CREATE VIEW calculator_target_buff_ids AS
SELECT DISTINCT buff_id
FROM calculator_skill_buffs
WHERE TRIM(COALESCE(buff_id, '')) REGEXP '^[0-9]+$';

DROP VIEW IF EXISTS calculator_target_buff_rows;
CREATE VIEW calculator_target_buff_rows AS
SELECT TRIM(bt.`Index`) AS buff_id,
       NULLIF(TRIM(bt.`BuffName`), '') AS buff_name_ko,
       NULLIF(TRIM(bt.`Description`), '') AS buff_description_ko,
       NULLIF(TRIM(bt.`ModuleType`), '') AS buff_module_type,
       bt.`Param0` AS buff_param0,
       bt.`Param1` AS buff_param1,
       bt.`Param2` AS buff_param2,
       bt.`Param3` AS buff_param3,
       bt.`Param4` AS buff_param4,
       bt.`Param5` AS buff_param5,
       bt.`Param6` AS buff_param6,
       bt.`Param7` AS buff_param7,
       bt.`Param8` AS buff_param8,
       bt.`Param9` AS buff_param9
FROM buff_table bt
JOIN calculator_target_buff_ids target
  ON TRIM(COALESCE(bt.`Index`, '')) = target.buff_id;

DROP VIEW IF EXISTS calculator_consumable_effects;
CREATE VIEW calculator_consumable_effects AS
SELECT src.source_key,
       src.item_id,
       src.item_name_ko,
       src.item_icon_file,
       src.item_description_ko,
       src.skill_source,
       src.skill_no,
       stype.skill_name_ko,
       stype.skill_description_ko,
       buffs.buff_slot,
       buffs.buff_id,
       bt.buff_name_ko,
       bt.buff_description_ko,
       bt.buff_module_type,
       bt.buff_param0,
       bt.buff_param1,
       bt.buff_param2,
       bt.buff_param3,
       bt.buff_param4,
       bt.buff_param5,
       bt.buff_param6,
       bt.buff_param7,
       bt.buff_param8,
       bt.buff_param9
FROM calculator_item_skill_sources src
LEFT JOIN calculator_target_skilltype_rows stype
  ON stype.skill_no = src.skill_no
LEFT JOIN calculator_skill_buffs buffs
  ON buffs.skill_no = src.skill_no
LEFT JOIN calculator_target_buff_rows bt
  ON bt.buff_id = buffs.buff_id;

DROP VIEW IF EXISTS calculator_consumable_effect_sources;
DROP VIEW IF EXISTS calculator_consumable_effect_lines;
CREATE VIEW calculator_consumable_effect_lines AS
SELECT source_key,
       item_id,
       NULLIF(TRIM(stype.skill_description_ko), '') AS effect_line
FROM calculator_item_skill_sources src
LEFT JOIN calculator_target_skilltype_rows stype
  ON stype.skill_no = src.skill_no
UNION ALL
SELECT src.source_key,
       src.item_id,
       NULLIF(TRIM(bt.`Description`), '') AS effect_line
FROM calculator_item_skill_sources src
JOIN calculator_target_skill_rows sk
  ON sk.skill_no = src.skill_no
LEFT JOIN buff_table bt
  ON TRIM(COALESCE(bt.`Index`, '')) = sk.buff0
WHERE TRIM(COALESCE(sk.buff0, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT src.source_key,
       src.item_id,
       NULLIF(TRIM(bt.`Description`), '') AS effect_line
FROM calculator_item_skill_sources src
JOIN calculator_target_skill_rows sk
  ON sk.skill_no = src.skill_no
LEFT JOIN buff_table bt
  ON TRIM(COALESCE(bt.`Index`, '')) = sk.buff1
WHERE TRIM(COALESCE(sk.buff1, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT src.source_key,
       src.item_id,
       NULLIF(TRIM(bt.`Description`), '') AS effect_line
FROM calculator_item_skill_sources src
JOIN calculator_target_skill_rows sk
  ON sk.skill_no = src.skill_no
LEFT JOIN buff_table bt
  ON TRIM(COALESCE(bt.`Index`, '')) = sk.buff2
WHERE TRIM(COALESCE(sk.buff2, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT src.source_key,
       src.item_id,
       NULLIF(TRIM(bt.`Description`), '') AS effect_line
FROM calculator_item_skill_sources src
JOIN calculator_target_skill_rows sk
  ON sk.skill_no = src.skill_no
LEFT JOIN buff_table bt
  ON TRIM(COALESCE(bt.`Index`, '')) = sk.buff3
WHERE TRIM(COALESCE(sk.buff3, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT src.source_key,
       src.item_id,
       NULLIF(TRIM(bt.`Description`), '') AS effect_line
FROM calculator_item_skill_sources src
JOIN calculator_target_skill_rows sk
  ON sk.skill_no = src.skill_no
LEFT JOIN buff_table bt
  ON TRIM(COALESCE(bt.`Index`, '')) = sk.buff4
WHERE TRIM(COALESCE(sk.buff4, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT src.source_key,
       src.item_id,
       NULLIF(TRIM(bt.`Description`), '') AS effect_line
FROM calculator_item_skill_sources src
JOIN calculator_target_skill_rows sk
  ON sk.skill_no = src.skill_no
LEFT JOIN buff_table bt
  ON TRIM(COALESCE(bt.`Index`, '')) = sk.buff5
WHERE TRIM(COALESCE(sk.buff5, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT src.source_key,
       src.item_id,
       NULLIF(TRIM(bt.`Description`), '') AS effect_line
FROM calculator_item_skill_sources src
JOIN calculator_target_skill_rows sk
  ON sk.skill_no = src.skill_no
LEFT JOIN buff_table bt
  ON TRIM(COALESCE(bt.`Index`, '')) = sk.buff6
WHERE TRIM(COALESCE(sk.buff6, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT src.source_key,
       src.item_id,
       NULLIF(TRIM(bt.`Description`), '') AS effect_line
FROM calculator_item_skill_sources src
JOIN calculator_target_skill_rows sk
  ON sk.skill_no = src.skill_no
LEFT JOIN buff_table bt
  ON TRIM(COALESCE(bt.`Index`, '')) = sk.buff7
WHERE TRIM(COALESCE(sk.buff7, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT src.source_key,
       src.item_id,
       NULLIF(TRIM(bt.`Description`), '') AS effect_line
FROM calculator_item_skill_sources src
JOIN calculator_target_skill_rows sk
  ON sk.skill_no = src.skill_no
LEFT JOIN buff_table bt
  ON TRIM(COALESCE(bt.`Index`, '')) = sk.buff8
WHERE TRIM(COALESCE(sk.buff8, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT src.source_key,
       src.item_id,
       NULLIF(TRIM(bt.`Description`), '') AS effect_line
FROM calculator_item_skill_sources src
JOIN calculator_target_skill_rows sk
  ON sk.skill_no = src.skill_no
LEFT JOIN buff_table bt
  ON TRIM(COALESCE(bt.`Index`, '')) = sk.buff9
WHERE TRIM(COALESCE(sk.buff9, '')) REGEXP '^[0-9]+$';

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
  FROM calculator_item_skill_sources
  GROUP BY source_key, item_id, item_name_ko, item_icon_file, item_description_ko
) base
LEFT JOIN (
  SELECT source_key,
         item_id,
         NULLIF(TRIM(GROUP_CONCAT(DISTINCT effect_line SEPARATOR '\n')), '') AS effect_description_ko
  FROM calculator_consumable_effect_lines
  WHERE effect_line IS NOT NULL
  GROUP BY source_key, item_id
) effect_texts
  ON effect_texts.source_key = base.source_key
 AND effect_texts.item_id = base.item_id;

DROP VIEW IF EXISTS calculator_lightstone_effect_sources;
CREATE VIEW calculator_lightstone_effect_sources AS
SELECT mapped.source_key,
       mapped.lightstone_set_id,
       mapped.set_name_ko,
       mapped.legacy_name_en,
       mapped.effect_description_ko
FROM (
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
  WHERE NULLIF(TRIM(COALESCE(set_name_ko, '')), '') IS NOT NULL
) mapped
JOIN (
  SELECT DISTINCT name AS legacy_name_en
  FROM items
  WHERE type = 'lightstone_set'
) targets
  ON targets.legacy_name_en = mapped.legacy_name_en;
