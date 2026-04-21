WITH relevant_skill_desc AS (
  SELECT DISTINCT TRIM(COALESCE(`SkillNo`, '')) AS skill_no
  FROM skilltype_table_new
  WHERE (
    COALESCE(`Desc`, '') LIKE '%낚시%'
    OR COALESCE(`Desc`, '') LIKE '%자동 낚시%'
    OR COALESCE(`Desc`, '') LIKE '%희귀 어종%'
    OR COALESCE(`Desc`, '') LIKE '%대형 어종%'
    OR COALESCE(`Desc`, '') LIKE '%낚시 경험치%'
    OR COALESCE(`Desc`, '') LIKE '%생활 경험치%'
    OR COALESCE(`Desc`, '') LIKE '%낚시 숙련도%'
    OR COALESCE(`Desc`, '') LIKE '%생활 숙련도%'
    OR COALESCE(`Desc`, '') LIKE '%내구도 소모 감소 저항%'
  )
),
relevant_buff_desc AS (
  SELECT DISTINCT TRIM(COALESCE(`Index`, '')) AS buff_id
  FROM buff_table
  WHERE (
    COALESCE(`Description`, '') LIKE '%낚시%'
    OR COALESCE(`Description`, '') LIKE '%자동 낚시%'
    OR COALESCE(`Description`, '') LIKE '%희귀 어종%'
    OR COALESCE(`Description`, '') LIKE '%대형 어종%'
    OR COALESCE(`Description`, '') LIKE '%낚시 경험치%'
    OR COALESCE(`Description`, '') LIKE '%생활 경험치%'
    OR COALESCE(`Description`, '') LIKE '%낚시 숙련도%'
    OR COALESCE(`Description`, '') LIKE '%생활 숙련도%'
    OR COALESCE(`Description`, '') LIKE '%내구도 소모 감소 저항%'
    OR COALESCE(`BuffName`, '') LIKE '%낚시%'
    OR COALESCE(`BuffName`, '') LIKE '%자동 낚시%'
    OR COALESCE(`BuffName`, '') LIKE '%희귀 어종%'
    OR COALESCE(`BuffName`, '') LIKE '%대형 어종%'
    OR COALESCE(`BuffName`, '') LIKE '%낚시 경험치%'
    OR COALESCE(`BuffName`, '') LIKE '%생활 경험치%'
    OR COALESCE(`BuffName`, '') LIKE '%낚시 숙련도%'
    OR COALESCE(`BuffName`, '') LIKE '%생활 숙련도%'
    OR COALESCE(`BuffName`, '') LIKE '%내구도 소모 감소 저항%'
  )
),
relevant_skills AS (
  SELECT DISTINCT TRIM(COALESCE(`SkillNo`, '')) AS skill_no
  FROM skill_table_new
  WHERE
    TRIM(COALESCE(`SkillNo`, '')) IN (SELECT skill_no FROM relevant_skill_desc)
    OR TRIM(COALESCE(`Buff0`, '')) IN (SELECT buff_id FROM relevant_buff_desc)
    OR TRIM(COALESCE(`Buff1`, '')) IN (SELECT buff_id FROM relevant_buff_desc)
    OR TRIM(COALESCE(`Buff2`, '')) IN (SELECT buff_id FROM relevant_buff_desc)
    OR TRIM(COALESCE(`Buff3`, '')) IN (SELECT buff_id FROM relevant_buff_desc)
    OR TRIM(COALESCE(`Buff4`, '')) IN (SELECT buff_id FROM relevant_buff_desc)
    OR TRIM(COALESCE(`Buff5`, '')) IN (SELECT buff_id FROM relevant_buff_desc)
    OR TRIM(COALESCE(`Buff6`, '')) IN (SELECT buff_id FROM relevant_buff_desc)
    OR TRIM(COALESCE(`Buff7`, '')) IN (SELECT buff_id FROM relevant_buff_desc)
    OR TRIM(COALESCE(`Buff8`, '')) IN (SELECT buff_id FROM relevant_buff_desc)
    OR TRIM(COALESCE(`Buff9`, '')) IN (SELECT buff_id FROM relevant_buff_desc)
),
skill_linked_items AS (
  SELECT DISTINCT
    CAST(`Index` AS SIGNED) AS item_id,
    NULLIF(TRIM(`ItemName`), '') AS display_name,
    NULLIF(TRIM(`IconImageFile`), '') AS item_icon_file
  FROM item_table
  WHERE
    TRIM(COALESCE(`SkillNo`, '')) IN (SELECT skill_no FROM relevant_skills)
    OR TRIM(COALESCE(`SubSkillNo`, '')) IN (SELECT skill_no FROM relevant_skills)
),
description_linked_items AS (
  SELECT DISTINCT
    CAST(`Index` AS SIGNED) AS item_id,
    NULLIF(TRIM(`ItemName`), '') AS display_name,
    NULLIF(TRIM(`IconImageFile`), '') AS item_icon_file
  FROM item_table
  WHERE (
    COALESCE(`Description`, '') LIKE '%낚시%'
    OR COALESCE(`Description`, '') LIKE '%자동 낚시%'
    OR COALESCE(`Description`, '') LIKE '%희귀 어종%'
    OR COALESCE(`Description`, '') LIKE '%대형 어종%'
    OR COALESCE(`Description`, '') LIKE '%낚시 경험치%'
    OR COALESCE(`Description`, '') LIKE '%생활 경험치%'
    OR COALESCE(`Description`, '') LIKE '%낚시 숙련도%'
    OR COALESCE(`Description`, '') LIKE '%생활 숙련도%'
    OR COALESCE(`Description`, '') LIKE '%내구도 소모 감소 저항%'
  )
),
fishing_tool_items AS (
  SELECT DISTINCT
    CAST(`Index` AS SIGNED) AS item_id,
    NULLIF(TRIM(`ItemName`), '') AS display_name,
    NULLIF(TRIM(`IconImageFile`), '') AS item_icon_file
  FROM item_table
  WHERE (
    COALESCE(`ItemName`, '') LIKE '%낚싯대%'
    OR COALESCE(`ItemName`, '') LIKE '%부유찌%'
    OR COALESCE(`ItemName`, '') LIKE '%낚시 의자%'
    OR COALESCE(`Description`, '') LIKE '%물고기를 낚을 수%'
  )
),
all_target_items AS (
  SELECT * FROM skill_linked_items
  UNION DISTINCT
  SELECT * FROM description_linked_items
  UNION DISTINCT
  SELECT * FROM fishing_tool_items
)
SELECT
  item_id,
  display_name,
  item_icon_file
FROM all_target_items
WHERE item_icon_file IS NOT NULL
ORDER BY item_id;
