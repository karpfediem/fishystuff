CREATE TABLE IF NOT EXISTS enchant_cash (
  `Index` VARCHAR(255) NOT NULL,
  `ItemName` TEXT NULL,
  `Enchant` VARCHAR(255) NOT NULL,
  `NeedEnchantItemID` TEXT NULL,
  `NeedEnchantItemCount` TEXT NULL,
  `BlackCountForItemMarket` TEXT NULL,
  `NeedPerfectEnchantItemCount` TEXT NULL,
  `EnchantSuccessRate` TEXT NULL,
  `EnchantAddRate` TEXT NULL,
  `EnchantFailType` TEXT NULL,
  `EnduranceLimit` TEXT NULL,
  `ReduceMaxEnduranceAtFail` TEXT NULL,
  `ReduceMaxEnduranceAtPerfectEnchant` TEXT NULL,
  `RecoverMaxEndurance` TEXT NULL,
  `DoNotCronKey` TEXT NULL,
  `NewPerfectEnchantItemKey` TEXT NULL,
  `NewPerfectEnchantItemCount` TEXT NULL,
  `NewPerfectEnchantDecraseMaxEndurance` TEXT NULL,
  `VariedStr` TEXT NULL,
  `VariedVit` TEXT NULL,
  `VariedWis` TEXT NULL,
  `VariedInt` TEXT NULL,
  `VariedDex` TEXT NULL,
  `VariedMaxHP` TEXT NULL,
  `VariedRecovHP` TEXT NULL,
  `VariedMaxMP` TEXT NULL,
  `VariedRecovMP` TEXT NULL,
  `VariedMaxFood` TEXT NULL,
  `VariedPossessableWeight` TEXT NULL,
  `VariedSuspension` TEXT NULL,
  `VariedCannonCoolTime` TEXT NULL,
  `VariedCannonAccuracy` TEXT NULL,
  `VariedCannonMaxLength` TEXT NULL,
  `VariedCannonMaxAngle` TEXT NULL,
  `VariedCriticalRate` TEXT NULL,
  `VariedMoveSpeedRate` TEXT NULL,
  `VariedAttackSpeedRate` TEXT NULL,
  `VariedCastingSpeedRate` TEXT NULL,
  `VariedCollectionSpeedRate` TEXT NULL,
  `VariedFishingSpeedRate` TEXT NULL,
  `VariedDropItemRate` TEXT NULL,
  `VariedSwimSpeedRate` TEXT NULL,
  `tribe0` TEXT NULL,
  `tribe1` TEXT NULL,
  `tribe2` TEXT NULL,
  `tribe3` TEXT NULL,
  `tribe4` TEXT NULL,
  `tribe5` TEXT NULL,
  `tribe6` TEXT NULL,
  `tribe7` TEXT NULL,
  `tribe8` TEXT NULL,
  `tribe9` TEXT NULL,
  `Socket` TEXT NULL,
  `EnchantBrokenRate` TEXT NULL,
  `EnchantDownRate` TEXT NULL,
  `AdJustStrenght` TEXT NULL,
  `AdJustSpeed` TEXT NULL,
  `AdJustHealth` TEXT NULL,
  `DDD` TEXT NULL,
  `AddedDDD` TEXT NULL,
  `DHIT` TEXT NULL,
  `DDV` TEXT NULL,
  `HDDV` TEXT NULL,
  `DPV` TEXT NULL,
  `HDPV` TEXT NULL,
  `RDD` TEXT NULL,
  `AddedRDD` TEXT NULL,
  `RHIT` TEXT NULL,
  `RDV` TEXT NULL,
  `HRDV` TEXT NULL,
  `RPV` TEXT NULL,
  `HRPV` TEXT NULL,
  `MDD` TEXT NULL,
  `AddedMDD` TEXT NULL,
  `MHIT` TEXT NULL,
  `MDV` TEXT NULL,
  `HMDV` TEXT NULL,
  `MPV` TEXT NULL,
  `HMPV` TEXT NULL,
  `HiddenMonsterDDD` TEXT NULL,
  `HiddenPcDDD` TEXT NULL,
  `HiddenMonsterDPV` TEXT NULL,
  `SkillNo` TEXT NULL,
  `Description` TEXT NULL,
  `PatternDescription` TEXT NULL,
  `IsNotifyWorld` TEXT NULL,
  `EnchantType` TEXT NULL,
  `ItemMarketAddedPrice` TEXT NULL,
  `EnchantAddRateByWorldMarket` TEXT NULL,
  `MarketFilter1` TEXT NULL,
  `MarketFilter2` TEXT NULL,
  `EnchantStackOnlyOne` TEXT NULL,
  `LifeMainType` TEXT NULL,
  `LifeSubType` TEXT NULL,
  `LifeStat` TEXT NULL,
  `ReservationExtraMaxRate` TEXT NULL,
  `ReservationExtraGetRate` TEXT NULL,
  `ReservationSellRate` TEXT NULL,
  `GroupLevel` TEXT NULL,
  `PreventCronCount` TEXT NULL,
  `Cronable` TEXT NULL,
  `ItemProtecter` TEXT NULL,
  `GuildRental` TEXT NULL,
  `NeedPerfectEnchantStack` TEXT NULL,
  `IsDevelopingContents` TEXT NULL,
  `MoonCount` TEXT NULL,
  `MoonSkillNo` TEXT NULL,
  PRIMARY KEY (`Index`, `Enchant`)
);

CREATE TABLE IF NOT EXISTS enchant_equipment (
  `Index` VARCHAR(255) NOT NULL,
  `ItemName` TEXT NULL,
  `Enchant` VARCHAR(255) NOT NULL,
  `NeedEnchantItemID` TEXT NULL,
  `NeedEnchantItemCount` TEXT NULL,
  `BlackCountForItemMarket` TEXT NULL,
  `NeedPerfectEnchantItemCount` TEXT NULL,
  `EnchantSuccessRate` TEXT NULL,
  `EnchantAddRate` TEXT NULL,
  `EnchantFailType` TEXT NULL,
  `EnduranceLimit` TEXT NULL,
  `ReduceMaxEnduranceAtFail` TEXT NULL,
  `ReduceMaxEnduranceAtPerfectEnchant` TEXT NULL,
  `RecoverMaxEndurance` TEXT NULL,
  `DoNotCronKey` TEXT NULL,
  `NewPerfectEnchantItemKey` TEXT NULL,
  `NewPerfectEnchantItemCount` TEXT NULL,
  `NewPerfectEnchantDecraseMaxEndurance` TEXT NULL,
  `VariedStr` TEXT NULL,
  `VariedVit` TEXT NULL,
  `VariedWis` TEXT NULL,
  `VariedInt` TEXT NULL,
  `VariedDex` TEXT NULL,
  `VariedMaxHP` TEXT NULL,
  `VariedRecovHP` TEXT NULL,
  `VariedMaxMP` TEXT NULL,
  `VariedRecovMP` TEXT NULL,
  `VariedMaxFood` TEXT NULL,
  `VariedPossessableWeight` TEXT NULL,
  `VariedSuspension` TEXT NULL,
  `VariedCannonCoolTime` TEXT NULL,
  `VariedCannonAccuracy` TEXT NULL,
  `VariedCannonMaxLength` TEXT NULL,
  `VariedCannonMaxAngle` TEXT NULL,
  `VariedCriticalRate` TEXT NULL,
  `VariedMoveSpeedRate` TEXT NULL,
  `VariedAttackSpeedRate` TEXT NULL,
  `VariedCastingSpeedRate` TEXT NULL,
  `VariedCollectionSpeedRate` TEXT NULL,
  `VariedFishingSpeedRate` TEXT NULL,
  `VariedDropItemRate` TEXT NULL,
  `VariedSwimSpeedRate` TEXT NULL,
  `tribe0` TEXT NULL,
  `tribe1` TEXT NULL,
  `tribe2` TEXT NULL,
  `tribe3` TEXT NULL,
  `tribe4` TEXT NULL,
  `tribe5` TEXT NULL,
  `tribe6` TEXT NULL,
  `tribe7` TEXT NULL,
  `tribe8` TEXT NULL,
  `tribe9` TEXT NULL,
  `Socket` TEXT NULL,
  `EnchantBrokenRate` TEXT NULL,
  `EnchantDownRate` TEXT NULL,
  `AdJustStrenght` TEXT NULL,
  `AdJustSpeed` TEXT NULL,
  `AdJustHealth` TEXT NULL,
  `DDD` TEXT NULL,
  `AddedDDD` TEXT NULL,
  `DHIT` TEXT NULL,
  `DDV` TEXT NULL,
  `HDDV` TEXT NULL,
  `DPV` TEXT NULL,
  `HDPV` TEXT NULL,
  `RDD` TEXT NULL,
  `AddedRDD` TEXT NULL,
  `RHIT` TEXT NULL,
  `RDV` TEXT NULL,
  `HRDV` TEXT NULL,
  `RPV` TEXT NULL,
  `HRPV` TEXT NULL,
  `MDD` TEXT NULL,
  `AddedMDD` TEXT NULL,
  `MHIT` TEXT NULL,
  `MDV` TEXT NULL,
  `HMDV` TEXT NULL,
  `MPV` TEXT NULL,
  `HMPV` TEXT NULL,
  `HiddenMonsterDDD` TEXT NULL,
  `HiddenPcDDD` TEXT NULL,
  `HiddenMonsterDPV` TEXT NULL,
  `SkillNo` TEXT NULL,
  `Description` TEXT NULL,
  `PatternDescription` TEXT NULL,
  `IsNotifyWorld` TEXT NULL,
  `EnchantType` TEXT NULL,
  `ItemMarketAddedPrice` TEXT NULL,
  `EnchantAddRateByWorldMarket` TEXT NULL,
  `MarketFilter1` TEXT NULL,
  `MarketFilter2` TEXT NULL,
  `EnchantStackOnlyOne` TEXT NULL,
  `LifeMainType` TEXT NULL,
  `LifeSubType` TEXT NULL,
  `LifeStat` TEXT NULL,
  `ReservationExtraMaxRate` TEXT NULL,
  `ReservationExtraGetRate` TEXT NULL,
  `ReservationSellRate` TEXT NULL,
  `GroupLevel` TEXT NULL,
  `PreventCronCount` TEXT NULL,
  `Cronable` TEXT NULL,
  `ItemProtecter` TEXT NULL,
  `GuildRental` TEXT NULL,
  `NeedPerfectEnchantStack` TEXT NULL,
  `IsDevelopingContents` TEXT NULL,
  `MoonCount` TEXT NULL,
  `MoonSkillNo` TEXT NULL,
  PRIMARY KEY (`Index`, `Enchant`)
);

CREATE TABLE IF NOT EXISTS enchant_lifeequipment (
  `Index` VARCHAR(255) NOT NULL,
  `ItemName` TEXT NULL,
  `Enchant` VARCHAR(255) NOT NULL,
  `NeedEnchantItemID` TEXT NULL,
  `NeedEnchantItemCount` TEXT NULL,
  `BlackCountForItemMarket` TEXT NULL,
  `NeedPerfectEnchantItemCount` TEXT NULL,
  `EnchantSuccessRate` TEXT NULL,
  `EnchantAddRate` TEXT NULL,
  `EnchantFailType` TEXT NULL,
  `EnduranceLimit` TEXT NULL,
  `ReduceMaxEnduranceAtFail` TEXT NULL,
  `ReduceMaxEnduranceAtPerfectEnchant` TEXT NULL,
  `RecoverMaxEndurance` TEXT NULL,
  `DoNotCronKey` TEXT NULL,
  `NewPerfectEnchantItemKey` TEXT NULL,
  `NewPerfectEnchantItemCount` TEXT NULL,
  `NewPerfectEnchantDecraseMaxEndurance` TEXT NULL,
  `VariedStr` TEXT NULL,
  `VariedVit` TEXT NULL,
  `VariedWis` TEXT NULL,
  `VariedInt` TEXT NULL,
  `VariedDex` TEXT NULL,
  `VariedMaxHP` TEXT NULL,
  `VariedRecovHP` TEXT NULL,
  `VariedMaxMP` TEXT NULL,
  `VariedRecovMP` TEXT NULL,
  `VariedMaxFood` TEXT NULL,
  `VariedPossessableWeight` TEXT NULL,
  `VariedSuspension` TEXT NULL,
  `VariedCannonCoolTime` TEXT NULL,
  `VariedCannonAccuracy` TEXT NULL,
  `VariedCannonMaxLength` TEXT NULL,
  `VariedCannonMaxAngle` TEXT NULL,
  `VariedCriticalRate` TEXT NULL,
  `VariedMoveSpeedRate` TEXT NULL,
  `VariedAttackSpeedRate` TEXT NULL,
  `VariedCastingSpeedRate` TEXT NULL,
  `VariedCollectionSpeedRate` TEXT NULL,
  `VariedFishingSpeedRate` TEXT NULL,
  `VariedDropItemRate` TEXT NULL,
  `VariedSwimSpeedRate` TEXT NULL,
  `tribe0` TEXT NULL,
  `tribe1` TEXT NULL,
  `tribe2` TEXT NULL,
  `tribe3` TEXT NULL,
  `tribe4` TEXT NULL,
  `tribe5` TEXT NULL,
  `tribe6` TEXT NULL,
  `tribe7` TEXT NULL,
  `tribe8` TEXT NULL,
  `tribe9` TEXT NULL,
  `Socket` TEXT NULL,
  `EnchantBrokenRate` TEXT NULL,
  `EnchantDownRate` TEXT NULL,
  `AdJustStrenght` TEXT NULL,
  `AdJustSpeed` TEXT NULL,
  `AdJustHealth` TEXT NULL,
  `DDD` TEXT NULL,
  `AddedDDD` TEXT NULL,
  `DHIT` TEXT NULL,
  `DDV` TEXT NULL,
  `HDDV` TEXT NULL,
  `DPV` TEXT NULL,
  `HDPV` TEXT NULL,
  `RDD` TEXT NULL,
  `AddedRDD` TEXT NULL,
  `RHIT` TEXT NULL,
  `RDV` TEXT NULL,
  `HRDV` TEXT NULL,
  `RPV` TEXT NULL,
  `HRPV` TEXT NULL,
  `MDD` TEXT NULL,
  `AddedMDD` TEXT NULL,
  `MHIT` TEXT NULL,
  `MDV` TEXT NULL,
  `HMDV` TEXT NULL,
  `MPV` TEXT NULL,
  `HMPV` TEXT NULL,
  `HiddenMonsterDDD` TEXT NULL,
  `HiddenPcDDD` TEXT NULL,
  `HiddenMonsterDPV` TEXT NULL,
  `SkillNo` TEXT NULL,
  `Description` TEXT NULL,
  `PatternDescription` TEXT NULL,
  `IsNotifyWorld` TEXT NULL,
  `EnchantType` TEXT NULL,
  `ItemMarketAddedPrice` TEXT NULL,
  `EnchantAddRateByWorldMarket` TEXT NULL,
  `MarketFilter1` TEXT NULL,
  `MarketFilter2` TEXT NULL,
  `EnchantStackOnlyOne` TEXT NULL,
  `LifeMainType` TEXT NULL,
  `LifeSubType` TEXT NULL,
  `LifeStat` TEXT NULL,
  `ReservationExtraMaxRate` TEXT NULL,
  `ReservationExtraGetRate` TEXT NULL,
  `ReservationSellRate` TEXT NULL,
  `GroupLevel` TEXT NULL,
  `PreventCronCount` TEXT NULL,
  `Cronable` TEXT NULL,
  `ItemProtecter` TEXT NULL,
  `GuildRental` TEXT NULL,
  `NeedPerfectEnchantStack` TEXT NULL,
  `IsDevelopingContents` TEXT NULL,
  `MoonCount` TEXT NULL,
  `MoonSkillNo` TEXT NULL,
  PRIMARY KEY (`Index`, `Enchant`)
);

CREATE TABLE IF NOT EXISTS tooltip_table (
  `Type` VARCHAR(255) NOT NULL,
  `Key` VARCHAR(255) NOT NULL,
  `StringFormat` TEXT NULL,
  `ParamCount` TEXT NULL,
  `DoNotTranslate` TEXT NULL,
  `Icon` TEXT NULL,
  PRIMARY KEY (`Type`, `Key`)
);

CREATE TABLE IF NOT EXISTS producttool_property (
  `ItemKey` VARCHAR(255) NOT NULL,
  `EnchantLevel` VARCHAR(255) NOT NULL,
  `ItemName` TEXT NULL,
  `Param_0` TEXT NULL,
  `Param_1` TEXT NULL,
  `Param_2` TEXT NULL,
  `Param_3` TEXT NULL,
  `AutofishingTimePercents` TEXT NULL,
  PRIMARY KEY (`ItemKey`, `EnchantLevel`)
);

DROP VIEW IF EXISTS calculator_fishing_effect_tooltips;
CREATE VIEW calculator_fishing_effect_tooltips AS
SELECT NULLIF(TRIM(`Key`), '') AS effect_macro,
       NULLIF(TRIM(`StringFormat`), '') AS tooltip_format_ko,
       CASE TRIM(COALESCE(`Key`, ''))
         WHEN 'AUTO_FISHING_REDUCE_TIME_DOWN_2' THEN 'afr'
         WHEN 'CHANCE_LARGE_SPECIES_FISH_INCRE' THEN 'bonus_big'
         WHEN 'CHANCE_RARE_SPECIES_FISH_INCRE' THEN 'bonus_rare'
         WHEN 'DUR_WEAPONS_CON_DOWN' THEN 'drr'
         WHEN 'FISHING_EXP_POINT_ADD' THEN 'exp_fish'
         WHEN 'LIFE_EXP_2' THEN 'exp_fish'
         WHEN 'FISHING_POINT' THEN 'fishing_potential_stage'
         WHEN 'LIFESTAT_FISHING_ALL_ADD' THEN 'fishing_mastery'
         WHEN 'FISHING_SIT_EFFECT_NORMAL' THEN 'fishing_mastery'
         WHEN 'FISHING_SIT_EFFECT_NORMAL2' THEN 'fishing_mastery'
         WHEN 'FISHING_SIT_EFFECT_NORMAL_44' THEN 'mixed'
         ELSE NULL
       END AS metric_kind,
       CASE
         WHEN TRIM(COALESCE(`Key`, '')) IN (
           'FISHING_SIT_EFFECT_NORMAL',
           'FISHING_SIT_EFFECT_NORMAL2',
           'FISHING_SIT_EFFECT_NORMAL_44'
         ) THEN 0
         ELSE 1
       END AS has_numeric_param
FROM tooltip_table
WHERE TRIM(COALESCE(`Key`, '')) IN (
  'AUTO_FISHING_REDUCE_TIME_DOWN_2',
  'CHANCE_LARGE_SPECIES_FISH_INCRE',
  'CHANCE_RARE_SPECIES_FISH_INCRE',
  'DUR_WEAPONS_CON_DOWN',
  'FISHING_EXP_POINT_ADD',
  'LIFE_EXP_2',
  'FISHING_POINT',
  'LIFESTAT_FISHING_ALL_ADD',
  'FISHING_SIT_EFFECT_NORMAL',
  'FISHING_SIT_EFFECT_NORMAL2',
  'FISHING_SIT_EFFECT_NORMAL_44'
);

DROP VIEW IF EXISTS calculator_fishing_producttool_properties;
CREATE VIEW calculator_fishing_producttool_properties AS
SELECT CAST(`ItemKey` AS SIGNED) AS source_item_key,
       NULLIF(TRIM(`EnchantLevel`), '') AS enchant_level,
       NULLIF(TRIM(`ItemName`), '') AS item_name_ko,
       CASE
         WHEN TRIM(COALESCE(`AutofishingTimePercents`, '')) REGEXP '^-?[0-9]+(\\.[0-9]+)?$'
           THEN CAST(`AutofishingTimePercents` AS DECIMAL(10, 4)) / 100.0
         ELSE NULL
       END AS autofishing_time_reduction
FROM producttool_property
WHERE COALESCE(`ItemName`, '') LIKE '%낚시%'
   OR TRIM(COALESCE(`AutofishingTimePercents`, '')) <> '';

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
   OR COALESCE(`PatternDescription`, '') LIKE '%AUTO_FISHING_REDUCE_TIME_DOWN_2(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%CHANCE_LARGE_SPECIES_FISH_INCRE(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%CHANCE_RARE_SPECIES_FISH_INCRE(%'
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
   OR COALESCE(`PatternDescription`, '') LIKE '%LIFESTAT_FISHING_ALL_ADD(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%FISHING_POINT(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%FISHING_EXP_POINT_ADD(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%AUTO_FISHING_REDUCE_TIME_DOWN_2(%'
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
   OR COALESCE(`PatternDescription`, '') LIKE '%LIFESTAT_FISHING_ALL_ADD(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%FISHING_POINT(%'
   OR COALESCE(`PatternDescription`, '') LIKE '%FISHING_EXP_POINT_ADD(%'
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
       COALESCE(effects.afr, effects.producttool_afr) AS afr,
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
        effects.producttool_afr,
        effects.bonus_rare,
        effects.bonus_big,
        effects.drr,
        effects.exp_fish,
        effects.fishing_mastery,
        effects.fishing_speed_stage
      ) IS NOT NULL;

DROP VIEW IF EXISTS calculator_legacy_aligned_enchant_item_effect_entries;
CREATE VIEW calculator_legacy_aligned_enchant_item_effect_entries AS
SELECT CONCAT(
         'item:',
         CAST(matched.item_id AS CHAR),
         ':enchant:',
         CAST(matched.enchant_level AS CHAR)
       ) AS source_key,
       matched.item_id,
       matched.item_type,
       matched.legacy_name_en,
       matched.source_name_ko,
       NULLIF(TRIM(item_source.`IconImageFile`), '') AS item_icon_file,
       legacy_item.icon_id AS legacy_icon_id,
       COALESCE(
         legacy_item.durability,
         CASE
           WHEN TRIM(COALESCE(item_source.`EnduranceLimit`, '')) REGEXP '^[0-9]+$'
             THEN CAST(item_source.`EnduranceLimit` AS SIGNED)
           ELSE NULL
         END,
         effect_match.durability
       ) AS durability,
       legacy_item.fish_multiplier,
       effect_match.afr,
       effect_match.bonus_rare,
       effect_match.bonus_big,
       effect_match.drr,
       effect_match.exp_fish,
       NULL AS exp_life
FROM (
  SELECT CAST(it.`Index` AS SIGNED) AS item_id,
         legacy_item.type AS item_type,
         legacy_item.name AS legacy_name_en,
         effects.item_name_ko AS source_name_ko,
         MIN(CAST(effects.enchant_level AS SIGNED)) AS enchant_level,
         MAX(effects.durability) AS durability,
         MAX(effects.afr) AS afr,
         MAX(effects.bonus_rare) AS bonus_rare,
         MAX(effects.bonus_big) AS bonus_big,
         MAX(effects.drr) AS drr,
         MAX(effects.exp_fish) AS exp_fish
  FROM calculator_enchant_item_effect_entries effects
  JOIN item_table it
    ON NULLIF(TRIM(it.`ItemName`), '') = effects.item_name_ko
  JOIN items legacy_item
    ON legacy_item.id = CAST(it.`Index` AS SIGNED)
   AND legacy_item.type IN ('rod', 'float', 'chair')
  WHERE COALESCE(effects.afr, 0) = COALESCE(legacy_item.afr, 0)
    AND COALESCE(effects.bonus_rare, 0) = COALESCE(legacy_item.bonus_rare, 0)
    AND COALESCE(effects.bonus_big, 0) = COALESCE(legacy_item.bonus_big, 0)
    AND COALESCE(effects.drr, 0) = COALESCE(legacy_item.drr, 0)
    AND COALESCE(effects.exp_fish, 0) = COALESCE(legacy_item.exp_fish, 0)
  GROUP BY CAST(it.`Index` AS SIGNED), legacy_item.type, legacy_item.name, effects.item_name_ko
) matched
JOIN calculator_enchant_item_effect_entries effect_match
  ON effect_match.item_name_ko = matched.source_name_ko
 AND CAST(effect_match.enchant_level AS SIGNED) = matched.enchant_level
LEFT JOIN items legacy_item
  ON legacy_item.id = matched.item_id
 AND legacy_item.type = matched.item_type
LEFT JOIN item_table item_source
  ON CAST(item_source.`Index` AS SIGNED) = matched.item_id;
