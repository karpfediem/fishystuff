CREATE TABLE IF NOT EXISTS common_stat_data (
  `LifeLevel` INT NOT NULL,
  `BaseStat` INT NULL,
  PRIMARY KEY (`LifeLevel`)
);

CREATE TABLE IF NOT EXISTS fishing_stat_data (
  `Stat` INT NOT NULL,
  `HighDropRate1` INT NULL,
  PRIMARY KEY (`Stat`)
);

CREATE TABLE IF NOT EXISTS translate_stat (
  `Point` INT NOT NULL,
  `MovespeedPercent` INT NULL,
  `AttackspeedPercent` INT NULL,
  `CastingspeedPercent` INT NULL,
  `CriticalPercent` INT NULL,
  `DropItemPercent` INT NULL,
  `FishingPercent` INT NULL,
  `CollectionPercent` INT NULL,
  PRIMARY KEY (`Point`)
);

DROP TABLE IF EXISTS common_stat_data__typed;
CREATE TABLE common_stat_data__typed (
  `LifeLevel` INT NOT NULL,
  `BaseStat` INT NULL,
  PRIMARY KEY (`LifeLevel`)
);
INSERT INTO common_stat_data__typed (`LifeLevel`, `BaseStat`)
SELECT CAST(`LifeLevel` AS SIGNED), CAST(`BaseStat` AS SIGNED)
FROM common_stat_data
WHERE TRIM(COALESCE(`LifeLevel`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`BaseStat`, '')) REGEXP '^-?[0-9]+$';
DROP TABLE common_stat_data;
RENAME TABLE common_stat_data__typed TO common_stat_data;

DROP TABLE IF EXISTS fishing_stat_data__typed;
CREATE TABLE fishing_stat_data__typed (
  `Stat` INT NOT NULL,
  `HighDropRate1` INT NULL,
  PRIMARY KEY (`Stat`)
);
INSERT INTO fishing_stat_data__typed (`Stat`, `HighDropRate1`)
SELECT CAST(`Stat` AS SIGNED), CAST(`HighDropRate1` AS SIGNED)
FROM fishing_stat_data
WHERE TRIM(COALESCE(`Stat`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`HighDropRate1`, '')) REGEXP '^-?[0-9]+$';
DROP TABLE fishing_stat_data;
RENAME TABLE fishing_stat_data__typed TO fishing_stat_data;

DROP TABLE IF EXISTS translate_stat__typed;
CREATE TABLE translate_stat__typed (
  `Point` INT NOT NULL,
  `MovespeedPercent` INT NULL,
  `AttackspeedPercent` INT NULL,
  `CastingspeedPercent` INT NULL,
  `CriticalPercent` INT NULL,
  `DropItemPercent` INT NULL,
  `FishingPercent` INT NULL,
  `CollectionPercent` INT NULL,
  PRIMARY KEY (`Point`)
);
INSERT INTO translate_stat__typed (
  `Point`,
  `MovespeedPercent`,
  `AttackspeedPercent`,
  `CastingspeedPercent`,
  `CriticalPercent`,
  `DropItemPercent`,
  `FishingPercent`,
  `CollectionPercent`
)
SELECT CAST(`Point` AS SIGNED),
       CAST(`MovespeedPercent` AS SIGNED),
       CAST(`AttackspeedPercent` AS SIGNED),
       CAST(`CastingspeedPercent` AS SIGNED),
       CAST(`CriticalPercent` AS SIGNED),
       CAST(`DropItemPercent` AS SIGNED),
       CAST(`FishingPercent` AS SIGNED),
       CAST(`CollectionPercent` AS SIGNED)
FROM translate_stat
WHERE TRIM(COALESCE(`Point`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`MovespeedPercent`, '')) REGEXP '^-?[0-9]+$'
  AND TRIM(COALESCE(`AttackspeedPercent`, '')) REGEXP '^-?[0-9]+$'
  AND TRIM(COALESCE(`CastingspeedPercent`, '')) REGEXP '^-?[0-9]+$'
  AND TRIM(COALESCE(`CriticalPercent`, '')) REGEXP '^-?[0-9]+$'
  AND TRIM(COALESCE(`DropItemPercent`, '')) REGEXP '^-?[0-9]+$'
  AND TRIM(COALESCE(`FishingPercent`, '')) REGEXP '^-?[0-9]+$'
  AND TRIM(COALESCE(`CollectionPercent`, '')) REGEXP '^-?[0-9]+$';
DROP TABLE translate_stat;
RENAME TABLE translate_stat__typed TO translate_stat;

DROP VIEW IF EXISTS calculator_lifeskill_base_stat_curve;
CREATE VIEW calculator_lifeskill_base_stat_curve AS
SELECT `LifeLevel` AS life_level,
       `BaseStat` AS base_stat
FROM common_stat_data
WHERE `BaseStat` IS NOT NULL;

DROP VIEW IF EXISTS calculator_stat_translation_curve;
CREATE VIEW calculator_stat_translation_curve AS
SELECT `Point` AS stat_point,
       `MovespeedPercent` AS move_speed_percent_raw,
       `AttackspeedPercent` AS attack_speed_percent_raw,
       `CastingspeedPercent` AS casting_speed_percent_raw,
       `CriticalPercent` AS critical_percent_raw,
       `DropItemPercent` AS drop_item_percent_raw,
       `FishingPercent` AS fishing_percent_raw,
       `CollectionPercent` AS collection_percent_raw,
       `MovespeedPercent` / 1000000.0 AS move_speed_rate,
       `AttackspeedPercent` / 1000000.0 AS attack_speed_rate,
       `CastingspeedPercent` / 1000000.0 AS casting_speed_rate,
       `CriticalPercent` / 1000000.0 AS critical_rate,
       `DropItemPercent` / 1000000.0 AS drop_item_rate,
       `FishingPercent` / 1000000.0 AS fishing_rate,
       `CollectionPercent` / 1000000.0 AS collection_rate
FROM translate_stat;

DROP VIEW IF EXISTS calculator_fishing_mastery_high_drop_curve;
CREATE VIEW calculator_fishing_mastery_high_drop_curve AS
SELECT `Stat` AS fishing_mastery,
       `HighDropRate1` AS high_drop_rate_raw,
       `HighDropRate1` / 1000000.0 AS high_drop_rate
FROM fishing_stat_data
WHERE `HighDropRate1` IS NOT NULL;
