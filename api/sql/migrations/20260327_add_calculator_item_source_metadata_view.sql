DROP VIEW IF EXISTS calculator_item_source_metadata;
CREATE VIEW calculator_item_source_metadata AS
SELECT CAST(`Index` AS SIGNED) AS item_id,
       NULLIF(TRIM(`ItemName`), '') AS item_name_ko,
       NULLIF(TRIM(`IconImageFile`), '') AS item_icon_file,
       CASE
         WHEN TRIM(COALESCE(`EnduranceLimit`, '')) REGEXP '^[0-9]+$'
           THEN CAST(`EnduranceLimit` AS SIGNED)
         ELSE NULL
       END AS endurance_limit
FROM item_table;
