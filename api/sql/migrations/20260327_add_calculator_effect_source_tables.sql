CREATE TABLE IF NOT EXISTS buff_table (
  `Index` VARCHAR(255) NOT NULL,
  `BuffName` TEXT NULL,
  `Category` TEXT NULL,
  `CategoryLevel` TEXT NULL,
  `Level` TEXT NULL,
  `Group` TEXT NULL,
  `ConditionType` TEXT NULL,
  `ModuleType` TEXT NULL,
  `BuffType` TEXT NULL,
  `DisplayOrder` TEXT NULL,
  `IsAbsolute` TEXT NULL,
  `IsOverlapped` TEXT NULL,
  `ApplyRate` TEXT NULL,
  `ValidityTime` TEXT NULL,
  `RepeatTime` TEXT NULL,
  `LimitCount` TEXT NULL,
  `NeedEquipType` TEXT NULL,
  `NewNeedEquipType` TEXT NULL,
  `RemoveOnDead` TEXT NULL,
  `OnlyApplyToCharacter` TEXT NULL,
  `ApplyToGroup` TEXT NULL,
  `IsToggle` TEXT NULL,
  `Param0` TEXT NULL,
  `Param1` TEXT NULL,
  `Param2` TEXT NULL,
  `Param3` TEXT NULL,
  `Param4` TEXT NULL,
  `Param5` TEXT NULL,
  `Param6` TEXT NULL,
  `Param7` TEXT NULL,
  `Param8` TEXT NULL,
  `Param9` TEXT NULL,
  `BuffEffect` TEXT NULL,
  `BuffIcon` TEXT NULL,
  `IsDisplay` TEXT NULL,
  `Description` TEXT NULL,
  `AmplificationType` TEXT NULL,
  `AmplificationRate` TEXT NULL,
  `UseMaxAttackType` TEXT NULL,
  `UIEffect` TEXT NULL,
  `DontApplyBuffKeyList` TEXT NULL,
  `SiegeOnlyBuff` TEXT NULL,
  `IsDevelopingContents` TEXT NULL,
  `EraseBuffType` TEXT NULL,
  PRIMARY KEY (`Index`)
);

CREATE TABLE IF NOT EXISTS skill_table_new (
  `SkillNo` VARCHAR(255) NOT NULL,
  `SkillLevel` VARCHAR(255) NOT NULL,
  `ActionName` TEXT NULL,
  `PreviewActionName` TEXT NULL,
  `IsPrompt_ForLearning` TEXT NULL,
  `SkillPoint_ForLearning` TEXT NULL,
  `PcLevel_ForLearning` TEXT NULL,
  `NeedMoney_ForLearning` TEXT NULL,
  `NeedItemID_ForLearning` TEXT NULL,
  `NeedItemCount_ForLearning` TEXT NULL,
  `NeedSkillNo0_ForLearning` TEXT NULL,
  `NeedSkillLevelNo0_ForLearning` TEXT NULL,
  `NeedSkillNo1_ForLearning` TEXT NULL,
  `NeedSkillLevelNo1_ForLearning` TEXT NULL,
  `RequireHP` TEXT NULL,
  `RequireMP` TEXT NULL,
  `RequireSP` TEXT NULL,
  `RequireSubResourcePoint` TEXT NULL,
  `SubResourceType` TEXT NULL,
  `NeedItemID` TEXT NULL,
  `NeedItemCount` TEXT NULL,
  `IsGlobalCycle` TEXT NULL,
  `ReuseGroup` TEXT NULL,
  `ReuseCycle` TEXT NULL,
  `isExpiredInOffline` TEXT NULL,
  `ApplyNumber` TEXT NULL,
  `DoCheckHit` TEXT NULL,
  `VariedHit` TEXT NULL,
  `BuffApplyRate` TEXT NULL,
  `StunValue` TEXT NULL,
  `Buff0` TEXT NULL,
  `Buff1` TEXT NULL,
  `Buff2` TEXT NULL,
  `Buff3` TEXT NULL,
  `Buff4` TEXT NULL,
  `Buff5` TEXT NULL,
  `Buff6` TEXT NULL,
  `Buff7` TEXT NULL,
  `Buff8` TEXT NULL,
  `Buff9` TEXT NULL,
  `Desc` TEXT NULL,
  `PatternDesc` TEXT NULL,
  `UsableInCoolTime` TEXT NULL,
  `ContentsGroupKey` TEXT NULL,
  `BalanceChannelSkillNo` TEXT NULL,
  `BlackSkillNo` TEXT NULL,
  `AdrenalinPoint` TEXT NULL,
  `IsDevelopingContents` TEXT NULL,
  `IsFusion` TEXT NULL,
  `DoNotTranslate` TEXT NULL,
  `EnableInWater` TEXT NULL,
  `DummyMonsterDistance` TEXT NULL,
  `LinkToolTipSkillNo` TEXT NULL,
  `RelatedCoolTimeSkillNo` TEXT NULL,
  `SpecialWeaponHitEffect` TEXT NULL,
  `TargetSearchAngle` TEXT NULL,
  `ExceptCoolTime` TEXT NULL,
  PRIMARY KEY (`SkillNo`, `SkillLevel`)
);

CREATE TABLE IF NOT EXISTS skilltype_table_new (
  `SkillNo` VARCHAR(255) NOT NULL,
  `SkillName` TEXT NULL,
  `SkillShortName` TEXT NULL,
  `SkillOwnerType` TEXT NULL,
  `IsToggle` TEXT NULL,
  `SkillType` TEXT NULL,
  `UiDisplayType` TEXT NULL,
  `UpgradeSkillType` TEXT NULL,
  `ForbidSkillType` TEXT NULL,
  `IsSettableQuickSlot` TEXT NULL,
  `WeaponEnduranceDecreseRate` TEXT NULL,
  `IsSiegeArea` TEXT NULL,
  `IsUsableFieldOnly` TEXT NULL,
  `AreaCheck` TEXT NULL,
  `UsableWeaponType` TEXT NULL,
  `RequireEquipType` TEXT NULL,
  `NewRequireEquipType` TEXT NULL,
  `EquipedItemID` TEXT NULL,
  `DisableOnBattle` TEXT NULL,
  `IsTargetDead` TEXT NULL,
  `ActionName` TEXT NULL,
  `IsTestimonialSkill` TEXT NULL,
  `IconImageFile` TEXT NULL,
  `Control` TEXT NULL,
  `PatternControl` TEXT NULL,
  `Desc` TEXT NULL,
  `0` TEXT NULL,
  `1` TEXT NULL,
  `2` TEXT NULL,
  `3` TEXT NULL,
  `4` TEXT NULL,
  `5` TEXT NULL,
  `6` TEXT NULL,
  `7` TEXT NULL,
  `8` TEXT NULL,
  `9` TEXT NULL,
  `10` TEXT NULL,
  `11` TEXT NULL,
  `12` TEXT NULL,
  `13` TEXT NULL,
  `14` TEXT NULL,
  `15` TEXT NULL,
  `16` TEXT NULL,
  `17` TEXT NULL,
  `18` TEXT NULL,
  `19` TEXT NULL,
  `20` TEXT NULL,
  `21` TEXT NULL,
  `22` TEXT NULL,
  `23` TEXT NULL,
  `24` TEXT NULL,
  `25` TEXT NULL,
  `26` TEXT NULL,
  `27` TEXT NULL,
  `28` TEXT NULL,
  `29` TEXT NULL,
  `30` TEXT NULL,
  `31` TEXT NULL,
  `32` TEXT NULL,
  `33` TEXT NULL,
  `34` TEXT NULL,
  `35` TEXT NULL,
  `36` TEXT NULL,
  `Pet_horse` TEXT NULL,
  `Pet_camel` TEXT NULL,
  `Pet_donkey` TEXT NULL,
  `Pet_elephant` TEXT NULL,
  `Pet_carriage2` TEXT NULL,
  `Pet_carriage4` TEXT NULL,
  `Pet_boat` TEXT NULL,
  `Pet_Cat` TEXT NULL,
  `Pet_Dog` TEXT NULL,
  `Pet_goat` TEXT NULL,
  `Pet_raft` TEXT NULL,
  `Pet_boatfishing` TEXT NULL,
  `PieceCountOfPartialSkill` TEXT NULL,
  `ContentsGroupKey` TEXT NULL,
  `Condition` TEXT NULL,
  `skillAwakeningType` TEXT NULL,
  `SkillCommandCheck` TEXT NULL,
  `SkillAlert` TEXT NULL,
  `IsDevelopingContents` TEXT NULL,
  `DoNotTranslate` TEXT NULL,
  `BranchType` TEXT NULL,
  `SkillUITreeGroup` TEXT NULL,
  `SkillUITreeGroupLevel` TEXT NULL,
  `SkillCoolTimeUICheck` TEXT NULL,
  `NeedBuffModuleType` TEXT NULL,
  PRIMARY KEY (`SkillNo`)
);

CREATE TABLE IF NOT EXISTS lightstone_set_option (
  `Index` VARCHAR(255) NOT NULL,
  `LightStone1` TEXT NULL,
  `LightStone2` TEXT NULL,
  `LightStone3` TEXT NULL,
  `LightStone4` TEXT NULL,
  `SetOptionSkillNo` TEXT NULL,
  `Description` TEXT NULL,
  PRIMARY KEY (`Index`)
);

CREATE TABLE IF NOT EXISTS pet_table (
  `CharacterKey` VARCHAR(255) NOT NULL,
  `PetChangeLookKey` TEXT NULL,
  `IsIndexed` TEXT NULL,
  `Race` TEXT NULL,
  `Kind` TEXT NULL,
  `Tier` TEXT NULL,
  `DefaultActionIndex` TEXT NULL,
  `BoneScale` TEXT NULL,
  `RequireEXPType` TEXT NULL,
  `Level` TEXT NULL,
  `Hunger` TEXT NULL,
  `Skill_0` TEXT NULL,
  `Skill_1` TEXT NULL,
  `BaseSkill` TEXT NULL,
  `EquipSkillAquireKey` TEXT NULL,
  `EquipSkill_0` TEXT NULL,
  `EquipSkill_1` TEXT NULL,
  `EquipSkill_2` TEXT NULL,
  `EquipSkill_3` TEXT NULL,
  `EquipSkill_4` TEXT NULL,
  `EquipSkill_5` TEXT NULL,
  `EquipSkill_6` TEXT NULL,
  `EquipSkill_7` TEXT NULL,
  `EquipSkill_8` TEXT NULL,
  `EquipSkill_9` TEXT NULL,
  `EquipSkill_10` TEXT NULL,
  `EquipSkill_11` TEXT NULL,
  `EquipSkill_12` TEXT NULL,
  `EquipSkill_13` TEXT NULL,
  `EquipSkill_14` TEXT NULL,
  `Action_0` TEXT NULL,
  `Action_1` TEXT NULL,
  `Action_2` TEXT NULL,
  `Action_3` TEXT NULL,
  `Action_4` TEXT NULL,
  `Action_5` TEXT NULL,
  `Action_6` TEXT NULL,
  `Action_7` TEXT NULL,
  `Action_8` TEXT NULL,
  `Action_9` TEXT NULL,
  `IconImageFile1` TEXT NULL,
  `basePriceForPetMarket` TEXT NULL,
  `minestPercentForPetMarket` TEXT NULL,
  `maxestPercentForPetMarket` TEXT NULL,
  `PetType` TEXT NULL,
  `IsJokerPetUse` TEXT NULL,
  `PcRoomType` TEXT NULL,
  PRIMARY KEY (`CharacterKey`)
);

CREATE TABLE IF NOT EXISTS pet_skill_table (
  `PetSkillNo` VARCHAR(255) NOT NULL,
  `PetSkillType` TEXT NULL,
  `Level` VARCHAR(255) NOT NULL,
  `Param0` TEXT NULL,
  `Param1` TEXT NULL,
  `NoDuplicateSkill` TEXT NULL,
  PRIMARY KEY (`PetSkillNo`, `Level`)
);

CREATE TABLE IF NOT EXISTS pet_base_skill_table (
  `Index` VARCHAR(255) NOT NULL,
  `GroupNo` TEXT NULL,
  `SkillNo` TEXT NULL,
  `NoDuplicateSkill` TEXT NULL,
  PRIMARY KEY (`Index`)
);

CREATE TABLE IF NOT EXISTS pet_setstats_table (
  `Tier` VARCHAR(255) NOT NULL,
  `PetCount` VARCHAR(255) NOT NULL,
  `Param0` TEXT NULL,
  `Param1` TEXT NULL,
  `Param2` TEXT NULL,
  PRIMARY KEY (`Tier`, `PetCount`)
);

CREATE TABLE IF NOT EXISTS pet_equipskill_table (
  `Index` VARCHAR(255) NOT NULL,
  `GroupNo` TEXT NULL,
  `SkillNo` TEXT NULL,
  PRIMARY KEY (`Index`)
);

CREATE TABLE IF NOT EXISTS pet_grade_table (
  `Race` VARCHAR(255) NOT NULL,
  `Kind` VARCHAR(255) NOT NULL,
  `DEV` TEXT NULL,
  `KOR_REAL` TEXT NULL,
  `JPN_REAL` TEXT NULL,
  `RUS_REAL` TEXT NULL,
  `NA_REAL` TEXT NULL,
  `TW_REAL` TEXT NULL,
  `SA_REAL` TEXT NULL,
  `KOR_2` TEXT NULL,
  `TH_REAL` TEXT NULL,
  `ID_REAL` TEXT NULL,
  `TR_REAL` TEXT NULL,
  `PS_REAL` TEXT NULL,
  `XB_REAL` TEXT NULL,
  `GT_REAL` TEXT NULL,
  `LV_REAL` TEXT NULL,
  `CS_REAL` TEXT NULL,
  `ASIA_REAL` TEXT NULL,
  `DD_SERVER` TEXT NULL,
  PRIMARY KEY (`Race`, `Kind`)
);

CREATE TABLE IF NOT EXISTS pet_exp_table (
  `PetExpTableKey` VARCHAR(255) NOT NULL,
  `Level` VARCHAR(255) NOT NULL,
  `Exp` TEXT NULL,
  PRIMARY KEY (`PetExpTableKey`, `Level`)
);

CREATE TABLE IF NOT EXISTS upgradepet_looting_percent (
  `Tier` VARCHAR(255) NOT NULL,
  `Percent` TEXT NULL,
  PRIMARY KEY (`Tier`)
);

DROP VIEW IF EXISTS calculator_skill_buffs;
CREATE VIEW calculator_skill_buffs AS
SELECT TRIM(`SkillNo`) AS skill_no,
       TRIM(`SkillLevel`) AS skill_level,
       0 AS buff_slot,
       TRIM(`Buff0`) AS buff_id
FROM skill_table_new
WHERE TRIM(COALESCE(`SkillNo`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`SkillLevel`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`Buff0`, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT TRIM(`SkillNo`), TRIM(`SkillLevel`), 1, TRIM(`Buff1`)
FROM skill_table_new
WHERE TRIM(COALESCE(`SkillNo`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`SkillLevel`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`Buff1`, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT TRIM(`SkillNo`), TRIM(`SkillLevel`), 2, TRIM(`Buff2`)
FROM skill_table_new
WHERE TRIM(COALESCE(`SkillNo`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`SkillLevel`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`Buff2`, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT TRIM(`SkillNo`), TRIM(`SkillLevel`), 3, TRIM(`Buff3`)
FROM skill_table_new
WHERE TRIM(COALESCE(`SkillNo`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`SkillLevel`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`Buff3`, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT TRIM(`SkillNo`), TRIM(`SkillLevel`), 4, TRIM(`Buff4`)
FROM skill_table_new
WHERE TRIM(COALESCE(`SkillNo`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`SkillLevel`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`Buff4`, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT TRIM(`SkillNo`), TRIM(`SkillLevel`), 5, TRIM(`Buff5`)
FROM skill_table_new
WHERE TRIM(COALESCE(`SkillNo`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`SkillLevel`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`Buff5`, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT TRIM(`SkillNo`), TRIM(`SkillLevel`), 6, TRIM(`Buff6`)
FROM skill_table_new
WHERE TRIM(COALESCE(`SkillNo`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`SkillLevel`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`Buff6`, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT TRIM(`SkillNo`), TRIM(`SkillLevel`), 7, TRIM(`Buff7`)
FROM skill_table_new
WHERE TRIM(COALESCE(`SkillNo`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`SkillLevel`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`Buff7`, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT TRIM(`SkillNo`), TRIM(`SkillLevel`), 8, TRIM(`Buff8`)
FROM skill_table_new
WHERE TRIM(COALESCE(`SkillNo`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`SkillLevel`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`Buff8`, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT TRIM(`SkillNo`), TRIM(`SkillLevel`), 9, TRIM(`Buff9`)
FROM skill_table_new
WHERE TRIM(COALESCE(`SkillNo`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`SkillLevel`, '')) REGEXP '^[0-9]+$'
  AND TRIM(COALESCE(`Buff9`, '')) REGEXP '^[0-9]+$';

DROP VIEW IF EXISTS calculator_item_skill_sources;
CREATE VIEW calculator_item_skill_sources AS
SELECT CONCAT('item:', CAST(`Index` AS CHAR)) AS source_key,
       CAST(`Index` AS SIGNED) AS item_id,
       NULLIF(TRIM(`ItemName`), '') AS item_name_ko,
       NULLIF(TRIM(`IconImageFile`), '') AS item_icon_file,
       NULLIF(TRIM(`Description`), '') AS item_description_ko,
       'skill' AS skill_source,
       TRIM(`SkillNo`) AS skill_no
FROM item_table
WHERE COALESCE(TRIM(`ItemType`), '') = '2'
  AND TRIM(COALESCE(`SkillNo`, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT CONCAT('item:', CAST(`Index` AS CHAR)),
       CAST(`Index` AS SIGNED),
       NULLIF(TRIM(`ItemName`), ''),
       NULLIF(TRIM(`IconImageFile`), ''),
       NULLIF(TRIM(`Description`), ''),
       'sub_skill',
       TRIM(`SubSkillNo`)
FROM item_table
WHERE COALESCE(TRIM(`ItemType`), '') = '2'
  AND TRIM(COALESCE(`SubSkillNo`, '')) REGEXP '^[0-9]+$';

DROP VIEW IF EXISTS calculator_consumable_effects;
CREATE VIEW calculator_consumable_effects AS
SELECT src.source_key,
       src.item_id,
       src.item_name_ko,
       src.item_icon_file,
       src.item_description_ko,
       src.skill_source,
       src.skill_no,
       NULLIF(TRIM(stype.`SkillName`), '') AS skill_name_ko,
       NULLIF(TRIM(stype.`Desc`), '') AS skill_description_ko,
       buffs.buff_slot,
       buffs.buff_id,
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
FROM calculator_item_skill_sources src
LEFT JOIN skilltype_table_new stype
  ON TRIM(COALESCE(stype.`SkillNo`, '')) = src.skill_no
LEFT JOIN calculator_skill_buffs buffs
  ON buffs.skill_no = src.skill_no
LEFT JOIN buff_table bt
  ON TRIM(COALESCE(bt.`Index`, '')) = buffs.buff_id
WHERE
  COALESCE(src.item_description_ko, '') LIKE '%낚시%'
  OR COALESCE(src.item_description_ko, '') LIKE '%희귀 어종%'
  OR COALESCE(src.item_description_ko, '') LIKE '%대형 어종%'
  OR COALESCE(src.item_description_ko, '') LIKE '%생활 경험치%'
  OR COALESCE(src.item_description_ko, '') LIKE '%생활 숙련도%'
  OR COALESCE(src.item_description_ko, '') LIKE '%내구도 소모 감소 저항%'
  OR COALESCE(stype.`Desc`, '') LIKE '%낚시%'
  OR COALESCE(stype.`Desc`, '') LIKE '%희귀 어종%'
  OR COALESCE(stype.`Desc`, '') LIKE '%대형 어종%'
  OR COALESCE(stype.`Desc`, '') LIKE '%생활 경험치%'
  OR COALESCE(stype.`Desc`, '') LIKE '%생활 숙련도%'
  OR COALESCE(stype.`Desc`, '') LIKE '%내구도 소모 감소 저항%'
  OR COALESCE(bt.`BuffName`, '') LIKE '%낚시%'
  OR COALESCE(bt.`Description`, '') LIKE '%낚시%'
  OR COALESCE(bt.`Description`, '') LIKE '%희귀 어종%'
  OR COALESCE(bt.`Description`, '') LIKE '%대형 어종%'
  OR COALESCE(bt.`Description`, '') LIKE '%생활 경험치%'
  OR COALESCE(bt.`Description`, '') LIKE '%생활 숙련도%'
  OR COALESCE(bt.`Description`, '') LIKE '%내구도 소모 감소 저항%';

DROP VIEW IF EXISTS calculator_lightstone_set_effects;
CREATE VIEW calculator_lightstone_set_effects AS
SELECT CONCAT('lightstone-set:', CAST(ls.`Index` AS CHAR)) AS source_key,
       TRIM(ls.`Index`) AS lightstone_set_id,
       TRIM(
         SUBSTRING_INDEX(
           SUBSTRING_INDEX(
             REPLACE(REPLACE(COALESCE(ls.`Description`, ''), '<PAColor0xffd2ffad>', ''), '<PAOldColor>', ''),
             ']',
             1
           ),
           '[',
           -1
         )
       ) AS set_name_ko,
       NULLIF(TRIM(stype.`SkillName`), '') AS skill_name_ko,
       NULLIF(TRIM(stype.`Desc`), '') AS skill_description_ko,
       NULLIF(TRIM(stype.`IconImageFile`), '') AS skill_icon_file,
       TRIM(ls.`SetOptionSkillNo`) AS skill_no,
       ls.`LightStone1`,
       ls.`LightStone2`,
       ls.`LightStone3`,
       ls.`LightStone4`,
       NULLIF(TRIM(ls.`Description`), '') AS effect_description_ko
FROM lightstone_set_option ls
LEFT JOIN skilltype_table_new stype
  ON TRIM(COALESCE(stype.`SkillNo`, '')) = TRIM(COALESCE(ls.`SetOptionSkillNo`, ''))
WHERE
  COALESCE(ls.`Description`, '') LIKE '%낚시%'
  OR COALESCE(ls.`Description`, '') LIKE '%희귀 어종%'
  OR COALESCE(ls.`Description`, '') LIKE '%대형 어종%'
  OR COALESCE(ls.`Description`, '') LIKE '%생활 경험치%'
  OR COALESCE(ls.`Description`, '') LIKE '%생활 숙련도%'
  OR COALESCE(ls.`Description`, '') LIKE '%내구도 소모 감소 저항%'
  OR COALESCE(stype.`Desc`, '') LIKE '%낚시%'
  OR COALESCE(stype.`Desc`, '') LIKE '%희귀 어종%'
  OR COALESCE(stype.`Desc`, '') LIKE '%대형 어종%'
  OR COALESCE(stype.`Desc`, '') LIKE '%생활 경험치%'
  OR COALESCE(stype.`Desc`, '') LIKE '%생활 숙련도%'
  OR COALESCE(stype.`Desc`, '') LIKE '%내구도 소모 감소 저항%';

DROP VIEW IF EXISTS calculator_pet_skill_sources;
CREATE VIEW calculator_pet_skill_sources AS
SELECT 'base_skill' AS source_type,
       TRIM(`Index`) AS source_id,
       TRIM(`GroupNo`) AS group_no,
       NULL AS tier,
       NULL AS pet_count,
       TRIM(`SkillNo`) AS skill_no
FROM pet_base_skill_table
WHERE TRIM(COALESCE(`SkillNo`, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT 'equip_skill',
       TRIM(`Index`),
       TRIM(`GroupNo`),
       NULL,
       NULL,
       TRIM(`SkillNo`)
FROM pet_equipskill_table
WHERE TRIM(COALESCE(`SkillNo`, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT 'set_bonus',
       NULL,
       NULL,
       TRIM(`Tier`),
       TRIM(`PetCount`),
       TRIM(`Param0`)
FROM pet_setstats_table
WHERE TRIM(COALESCE(`Param0`, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT 'set_bonus',
       NULL,
       NULL,
       TRIM(`Tier`),
       TRIM(`PetCount`),
       TRIM(`Param1`)
FROM pet_setstats_table
WHERE TRIM(COALESCE(`Param1`, '')) REGEXP '^[0-9]+$'
UNION ALL
SELECT 'set_bonus',
       NULL,
       NULL,
       TRIM(`Tier`),
       TRIM(`PetCount`),
       TRIM(`Param2`)
FROM pet_setstats_table
WHERE TRIM(COALESCE(`Param2`, '')) REGEXP '^[0-9]+$';

DROP VIEW IF EXISTS calculator_pet_skill_options;
CREATE VIEW calculator_pet_skill_options AS
SELECT src.source_type,
       src.source_id,
       src.group_no,
       src.tier,
       src.pet_count,
       src.skill_no,
       CASE
         WHEN COALESCE(stype.`SkillName`, '') LIKE '%자동 낚시%'
           OR COALESCE(stype.`Desc`, '') LIKE '%자동 낚시%' THEN 'special'
         WHEN COALESCE(stype.`SkillName`, '') LIKE '%낚시 경험치%'
           OR COALESCE(stype.`Desc`, '') LIKE '%낚시 경험치%' THEN 'skill'
         WHEN COALESCE(stype.`SkillName`, '') LIKE '%생활 경험치%'
           OR COALESCE(stype.`Desc`, '') LIKE '%생활 경험치%'
           OR COALESCE(stype.`SkillName`, '') LIKE '%생활 숙련도%'
           OR COALESCE(stype.`Desc`, '') LIKE '%생활 숙련도%'
           OR COALESCE(stype.`SkillName`, '') LIKE '%내구도 소모 감소 저항%'
           OR COALESCE(stype.`Desc`, '') LIKE '%내구도 소모 감소 저항%' THEN 'talent'
         ELSE 'other'
       END AS option_kind,
       NULLIF(TRIM(stype.`SkillName`), '') AS skill_name_ko,
       NULLIF(TRIM(stype.`SkillShortName`), '') AS skill_short_name_ko,
       NULLIF(TRIM(stype.`Desc`), '') AS skill_description_ko,
       NULLIF(TRIM(stype.`IconImageFile`), '') AS skill_icon_file
FROM calculator_pet_skill_sources src
LEFT JOIN skilltype_table_new stype
  ON TRIM(COALESCE(stype.`SkillNo`, '')) = src.skill_no
WHERE
  COALESCE(stype.`SkillName`, '') LIKE '%낚시%'
  OR COALESCE(stype.`Desc`, '') LIKE '%낚시%'
  OR COALESCE(stype.`SkillName`, '') LIKE '%생활 경험치%'
  OR COALESCE(stype.`Desc`, '') LIKE '%생활 경험치%'
  OR COALESCE(stype.`SkillName`, '') LIKE '%생활 숙련도%'
  OR COALESCE(stype.`Desc`, '') LIKE '%생활 숙련도%'
  OR COALESCE(stype.`SkillName`, '') LIKE '%내구도 소모 감소 저항%'
  OR COALESCE(stype.`Desc`, '') LIKE '%내구도 소모 감소 저항%';
