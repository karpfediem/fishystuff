DROP VIEW IF EXISTS calculator_lightstone_effect_sources;

CREATE VIEW calculator_lightstone_effect_sources AS
SELECT effects.source_key,
       effects.lightstone_set_id,
       effects.set_name_ko,
       (
         SELECT TRIM(
                  TRAILING ']'
                  FROM SUBSTRING_INDEX(NULLIF(TRIM(l.`text`), ''), '[', -1)
                )
         FROM languagedata_en l
         WHERE l.`id` = CAST(effects.skill_no AS SIGNED)
           AND COALESCE(l.`format`, '') = 'B'
           AND COALESCE(l.`unk`, '') = '10'
           AND COALESCE(l.`text`, '') LIKE 'Set % - [%'
         LIMIT 1
       ) AS source_name_en,
       effects.skill_icon_file,
       effects.effect_description_ko,
       NULLIF(
         CASE
           WHEN COALESCE(effects.effect_description_ko, '') LIKE '%자동 낚시 시간 감소 %'
             THEN ABS(CAST(
               REGEXP_SUBSTR(
                 REGEXP_SUBSTR(
                   effects.effect_description_ko,
                   '자동 낚시[^%]*[-+]?[0-9]+(\\.[0-9]+)?%'
                 ),
                 '[-+]?[0-9]+(\\.[0-9]+)?'
               ) AS DECIMAL(10, 4)
             )) / 100.0
           WHEN COALESCE(effects.effect_description_ko, '') LIKE '%자동 낚시 시간 %'
             THEN ABS(CAST(
               REGEXP_SUBSTR(
                 REGEXP_SUBSTR(
                   effects.effect_description_ko,
                   '자동 낚시[^%]*[-+]?[0-9]+(\\.[0-9]+)?%'
                 ),
                 '[-+]?[0-9]+(\\.[0-9]+)?'
               ) AS DECIMAL(10, 4)
             )) / 100.0
           ELSE 0
         END,
         0
       ) AS afr,
       NULLIF(
         CASE
           WHEN COALESCE(effects.effect_description_ko, '') LIKE '%희귀 어종을 낚을 확률 증가 %'
             THEN CAST(
               REGEXP_SUBSTR(
                 REGEXP_SUBSTR(
                   effects.effect_description_ko,
                   '희귀 어종[^%]*[-+]?[0-9]+(\\.[0-9]+)?%'
                 ),
                 '[-+]?[0-9]+(\\.[0-9]+)?'
               ) AS DECIMAL(10, 4)
             ) / 100.0
           ELSE 0
         END,
         0
       ) AS bonus_rare,
       NULLIF(
         CASE
           WHEN COALESCE(effects.effect_description_ko, '') LIKE '%대형 어종을 낚을 확률 증가 %'
             THEN CAST(
               REGEXP_SUBSTR(
                 REGEXP_SUBSTR(
                   effects.effect_description_ko,
                   '대형 어종[^%]*[-+]?[0-9]+(\\.[0-9]+)?%'
                 ),
                 '[-+]?[0-9]+(\\.[0-9]+)?'
               ) AS DECIMAL(10, 4)
             ) / 100.0
           ELSE 0
         END,
         0
       ) AS bonus_big,
       NULLIF(
         CASE
           WHEN COALESCE(effects.effect_description_ko, '') LIKE '%내구도 소모 감소 저항 %'
             THEN CAST(
               REGEXP_SUBSTR(
                 REGEXP_SUBSTR(
                   effects.effect_description_ko,
                   '내구도 소모 감소 저항[^%]*[-+]?[0-9]+(\\.[0-9]+)?%'
                 ),
                 '[-+]?[0-9]+(\\.[0-9]+)?'
               ) AS DECIMAL(10, 4)
             ) / 100.0
           ELSE 0
         END,
         0
       ) AS drr,
       NULLIF(
         CASE
           WHEN COALESCE(effects.effect_description_ko, '') LIKE '%낚시 경험치 획득량 %'
             THEN CAST(
               REGEXP_SUBSTR(
                 REGEXP_SUBSTR(
                   effects.effect_description_ko,
                   '낚시 경험치 획득량[^%]*[-+]?[0-9]+(\\.[0-9]+)?%'
                 ),
                 '[-+]?[0-9]+(\\.[0-9]+)?'
               ) AS DECIMAL(10, 4)
             ) / 100.0
           ELSE 0
         END,
         0
       ) AS exp_fish,
       NULLIF(
         CASE
           WHEN COALESCE(effects.effect_description_ko, '') LIKE '%생활 경험치 획득량 %'
             THEN CAST(
               REGEXP_SUBSTR(
                 REGEXP_SUBSTR(
                   effects.effect_description_ko,
                   '생활 경험치 획득량[^%]*[-+]?[0-9]+(\\.[0-9]+)?%'
                 ),
                 '[-+]?[0-9]+(\\.[0-9]+)?'
               ) AS DECIMAL(10, 4)
             ) / 100.0
           ELSE 0
         END,
         0
       ) AS exp_life
FROM calculator_lightstone_set_effects effects
WHERE NULLIF(TRIM(COALESCE(effects.set_name_ko, '')), '') IS NOT NULL;
