CREATE TABLE IF NOT EXISTS fishing_table (
  R TINYINT UNSIGNED NOT NULL,
  G TINYINT UNSIGNED NOT NULL,
  B TINYINT UNSIGNED NOT NULL,

  DropID BIGINT NULL,
  DropIDHarpoon BIGINT NULL,
  DropIDNet BIGINT NULL,

  DropRate1 BIGINT NULL,
  DropID1 BIGINT NULL,
  DropRate2 BIGINT NULL,
  DropID2 BIGINT NULL,
  DropRate3 BIGINT NULL,
  DropID3 BIGINT NULL,
  DropRate4 BIGINT NULL,
  DropID4 BIGINT NULL,
  DropRate5 BIGINT NULL,
  DropID5 BIGINT NULL,

  MinWaitTime BIGINT NULL,
  MaxWaitTime BIGINT NULL,

  PRIMARY KEY (R, G, B)
);

CREATE TABLE IF NOT EXISTS patches (
  patch_id VARCHAR(128) NOT NULL,
  start_date DATE NULL,
  start_ts_utc BIGINT NOT NULL,
  patch_name TEXT NULL,
  category TEXT NULL,
  sub_category TEXT NULL,
  key_values TEXT NULL,
  change_description TEXT NULL,
  impact TEXT NULL,
  region TEXT NULL,
  source_url TEXT NULL,
  PRIMARY KEY (patch_id),
  INDEX idx_patches_start_ts (start_ts_utc)
);

CREATE TABLE IF NOT EXISTS item_main_group_table (
  ItemMainGroupKey BIGINT NOT NULL,
  DoSelectOnlyOne TINYINT NULL,
  RefreshStartHour INT NULL,
  RefreshInterval INT NULL,
  PlantCraftResultCount INT NULL,

  SelectRate0 BIGINT NULL,
  Condition0 TEXT NULL,
  ItemSubGroupKey0 BIGINT NULL,

  SelectRate1 BIGINT NULL,
  Condition1 TEXT NULL,
  ItemSubGroupKey1 BIGINT NULL,

  SelectRate2 BIGINT NULL,
  Condition2 TEXT NULL,
  ItemSubGroupKey2 BIGINT NULL,

  SelectRate3 BIGINT NULL,
  Condition3 TEXT NULL,
  ItemSubGroupKey3 BIGINT NULL,

  PRIMARY KEY (ItemMainGroupKey)
);

CREATE TABLE IF NOT EXISTS item_sub_group_table (
  ItemSubGroupKey BIGINT NOT NULL,
  ItemKey BIGINT NOT NULL,
  EnchantLevel INT NOT NULL,
  DoPetAddDrop TINYINT NULL,
  DoSechiAddDrop TINYINT NULL,

  SelectRate_0 BIGINT NULL,
  MinCount_0 INT NULL,
  MaxCount_0 INT NULL,

  SelectRate_1 BIGINT NULL,
  MinCount_1 INT NULL,
  MaxCount_1 INT NULL,

  SelectRate_2 BIGINT NULL,
  MinCount_2 INT NULL,
  MaxCount_2 INT NULL,

  IntimacyVariation INT NULL,
  ExplorationPoint INT NULL,
  ApplyRandomPrice TINYINT NULL,
  RentTime INT NULL,
  PriceOption INT NULL,

  PRIMARY KEY (ItemSubGroupKey, ItemKey, EnchantLevel)
);

CREATE TABLE IF NOT EXISTS community_zone_fish_support (
  source_id VARCHAR(64) NOT NULL,
  source_label VARCHAR(128) NOT NULL,
  source_sha256 CHAR(64) NULL,
  zone_rgb INT UNSIGNED NOT NULL,
  zone_r TINYINT UNSIGNED NOT NULL,
  zone_g TINYINT UNSIGNED NOT NULL,
  zone_b TINYINT UNSIGNED NOT NULL,
  region_name TEXT NULL,
  zone_name TEXT NULL,
  item_id BIGINT NOT NULL,
  fish_name TEXT NULL,
  support_status VARCHAR(32) NOT NULL,
  claim_count INT NOT NULL DEFAULT 1,
  notes TEXT NULL,

  PRIMARY KEY (source_id, zone_rgb, item_id),
  KEY idx_community_zone_fish_support_rgb (zone_rgb),
  KEY idx_community_zone_fish_support_status (support_status),
  KEY idx_community_zone_fish_support_item (item_id)
);

CREATE TABLE IF NOT EXISTS item_table (
  `Index` BIGINT NOT NULL,
  `ItemName` TEXT NULL,
  `ItemType` TEXT NULL,
  `ItemClassify` TEXT NULL,
  `GradeType` TEXT NULL,
  `EquipType` TEXT NULL,
  `OccupiedEquipNo` TEXT NULL,
  `AwakenEquipType` TEXT NULL,
  `NewEquipType` TEXT NULL,
  `NewOccupiedEquipNo` TEXT NULL,
  `Weight` TEXT NULL,
  `EnduranceLimit` TEXT NULL,
  `IsStack` TEXT NULL,
  `DoApplyDirectly` TEXT NULL,
  `ExpirationPeriod` TEXT NULL,
  `VestedType` TEXT NULL,
  `IsUserVested` TEXT NULL,
  `IsDropable` TEXT NULL,
  `MinLevel` TEXT NULL,
  `MaxLevel` TEXT NULL,
  `LifeExpType` TEXT NULL,
  `LifeMinLevel` TEXT NULL,
  `SkillNo` TEXT NULL,
  `SubSkillNo` TEXT NULL,
  `IsRemovable` TEXT NULL,
  `IsForTrade` TEXT NULL,
  `TradeType` TEXT NULL,
  `DestroyProbability` TEXT NULL,
  `PriceType` TEXT NULL,
  `OriginalPrice` TEXT NULL,
  `SellPriceToNpc` TEXT NULL,
  `OriginalPriceRate_0` TEXT NULL,
  `OriginalPriceRate_1` TEXT NULL,
  `OriginalPriceRate_2` TEXT NULL,
  `PriceRate_0` TEXT NULL,
  `PriceRate_1` TEXT NULL,
  `PriceRate_2` TEXT NULL,
  `RepairPrice` TEXT NULL,
  `RepairTime` TEXT NULL,
  `doLogging` TEXT NULL,
  `Character_Key` TEXT NULL,
  `IconImageFile` TEXT NULL,
  `Description` TEXT NULL,
  `PatternDescription` TEXT NULL,
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
  `Pet_0` TEXT NULL,
  `Pet_1` TEXT NULL,
  `Pet_2` TEXT NULL,
  `Pet_3` TEXT NULL,
  `Pet_4` TEXT NULL,
  `Pet_5` TEXT NULL,
  `Pet_6` TEXT NULL,
  `Pet_7` TEXT NULL,
  `Pet_8` TEXT NULL,
  `Pet_9` TEXT NULL,
  `Pet_10` TEXT NULL,
  `Pet_11` TEXT NULL,
  `Pet_12` TEXT NULL,
  `Pet_13` TEXT NULL,
  `Pet_14` TEXT NULL,
  `Pet_15` TEXT NULL,
  `Pet_16` TEXT NULL,
  `Pet_17` TEXT NULL,
  `Pet_18` TEXT NULL,
  `Pet_19` TEXT NULL,
  `Pet_20` TEXT NULL,
  `Pet_21` TEXT NULL,
  `Pet_22` TEXT NULL,
  `Pet_23` TEXT NULL,
  `Pet_24` TEXT NULL,
  `Pet_25` TEXT NULL,
  `Pet_26` TEXT NULL,
  `Pet_27` TEXT NULL,
  `Pet_28` TEXT NULL,
  `Pet_29` TEXT NULL,
  `Pet_30` TEXT NULL,
  `SubType` TEXT NULL,
  `WeaponMaterial` TEXT NULL,
  `ArmorMaterial` TEXT NULL,
  `ItemMaterial` TEXT NULL,
  `ItemAccessLevel` TEXT NULL,
  `Incineration` TEXT NULL,
  `JewelGroupNumber` TEXT NULL,
  `JewelSubtractionNeedItem` TEXT NULL,
  `JewelSubtractionNeedItemCount` TEXT NULL,
  `JewelEquipType` TEXT NULL,
  `JewelColorType` TEXT NULL,
  `JewelDemolitionRate` TEXT NULL,
  `DropAudioIndex` TEXT NULL,
  `ContentsEventType` TEXT NULL,
  `ContentsEventParam1` TEXT NULL,
  `ContentsEventParam2` TEXT NULL,
  `ContentsEventParam3` TEXT NULL,
  `CommerceType` TEXT NULL,
  `CollectToolType` TEXT NULL,
  `CollectTime` TEXT NULL,
  `UseCondition` TEXT NULL,
  `KarmaType` TEXT NULL,
  `TargetType` TEXT NULL,
  `IsTargetAlive` TEXT NULL,
  `ItemActionNumber` TEXT NULL,
  `DropRateByKill` TEXT NULL,
  `PopupDesc` TEXT NULL,
  `PopupResultItem` TEXT NULL,
  `PopupResultBuff` TEXT NULL,
  `ExchangePosition` TEXT NULL,
  `NeedContribute` TEXT NULL,
  `IsAbandon` TEXT NULL,
  `NodeFreeTrade` TEXT NULL,
  `HideFromNote` TEXT NULL,
  `IsCash` TEXT NULL,
  `IsTerm` TEXT NULL,
  `isGuildStockable` TEXT NULL,
  `basePriceForItemMarket` TEXT NULL,
  `minestPercentForItemMarket` TEXT NULL,
  `maxestPercentForItemMarket` TEXT NULL,
  `maxRegisterCountForItemMarket` TEXT NULL,
  `basePriceForWorldMarket` TEXT NULL,
  `minestPercentForWorldMarket` TEXT NULL,
  `maxestPercentForWorldMarket` TEXT NULL,
  `maxRegisterCountForWorldMarket` TEXT NULL,
  `sellCountForWorldMarket` TEXT NULL,
  `addBuyCountForWorldMarket` TEXT NULL,
  `isForceDisplayWorldMarket` TEXT NULL,
  `IsDyeable` TEXT NULL,
  `IsNavigation` TEXT NULL,
  `IsNotifyWorld` TEXT NULL,
  `IsPersnalNotify` TEXT NULL,
  `CraftResultItem` TEXT NULL,
  `ExtractionCount` TEXT NULL,
  `CronCount` TEXT NULL,
  `IsExchangeItem` TEXT NULL,
  `IsPersonalTrade` TEXT NULL,
  `IsDisposalWareHouse` TEXT NULL,
  `AvatarType` TEXT NULL,
  `ExpirationPeriod Type` TEXT NULL,
  `ExpirationPeriod Param1` TEXT NULL,
  `ExpirationPeriod Param2` TEXT NULL,
  `AcquireAction` TEXT NULL,
  `RepairEnduranceCount` TEXT NULL,
  `IsClientUse` TEXT NULL,
  `RestoreType` TEXT NULL,
  `MarketCategory1` TEXT NULL,
  `MarketCategory2` TEXT NULL,
  `MarketFilter1` TEXT NULL,
  `MarketFilter2` TEXT NULL,
  `ConnectUi` TEXT NULL,
  `EnchantDifficulty` TEXT NULL,
  `DoNotTranslate` TEXT NULL,
  `SpecialEventType` TEXT NULL,
  `AttackType` TEXT NULL,
  `LimitDamageSiegeObject` TEXT NULL,
  `ItemProperties` TEXT NULL,
  `CronKey` TEXT NULL,
  `CronEnchantcontrol` TEXT NULL,
  `RandomOptionKey` TEXT NULL,
  `SeasonItemType` TEXT NULL,
  `TradeCountToUpdate` TEXT NULL,
  `AccumulatePassCount` TEXT NULL,
  `isDisplayWorldMarket` TEXT NULL,
  `isNoticeWorldMarket` TEXT NULL,
  `ItemWearSkill` TEXT NULL,
  `EnchantKey` TEXT NULL,
  `DropMeshKey` TEXT NULL,
  `IsDevelopingContents` TEXT NULL,
  `GrowthEquipType` TEXT NULL,
  `FairyFeedKey` TEXT NULL,
  `MarketPriceGroup` TEXT NULL,
  `BartarType` TEXT NULL,
  `ServantSupplyType` TEXT NULL,
  `ServantSupplyValue` TEXT NULL,
  `ItemMarketKey` TEXT NULL,
  `SellAtOnce` TEXT NULL,
  `ImpossibleCopy` TEXT NULL,
  `OlympicItemType` TEXT NULL,
  `ContentsGroupKey` TEXT NULL,
  `FamilyInventoryType` TEXT NULL,
  `SeasonAutoCloseItem` TEXT NULL,
  `SequenceUseItem` TEXT NULL,
  `InvenCategorySort` TEXT NULL,
  `BlackSpiritInven` TEXT NULL,
  `SearchItem` TEXT NULL,
  `StrKey` TEXT NULL,
  `IsDevelopingNo` TEXT NULL,
  `AdventureType` TEXT NULL,
  PRIMARY KEY (`Index`)
);

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

CREATE TABLE IF NOT EXISTS fish_table (
  encyclopedia_key BIGINT NOT NULL,
  item_key BIGINT NOT NULL,
  name TEXT NULL,
  icon TEXT NULL,
  encyclopedia_icon TEXT NULL,
  PRIMARY KEY (encyclopedia_key),
  INDEX idx_fish_table_item_key (item_key)
);

CREATE TABLE IF NOT EXISTS languagedata_en (
  `id` BIGINT NOT NULL,
  `unk` TEXT NULL,
  `text` TEXT NULL,
  `format` TEXT NULL
);

CREATE VIEW IF NOT EXISTS fish_names_ko AS
SELECT `Index` AS fish_id, `ItemName` AS name_ko
FROM item_table
WHERE (`ItemType` = '8' AND `ItemClassify` = '16')
   OR `Index` IN (40218, 44422, 820036);

DROP VIEW IF EXISTS fish_names_en;
CREATE VIEW fish_names_en AS
SELECT k.fish_id,
       COALESCE(NULLIF(l.`text`, ''), k.name_ko) AS name_en
FROM fish_names_ko k
LEFT JOIN languagedata_en l ON l.`id` = k.fish_id
  AND l.`format` = 'A'
  AND COALESCE(l.`unk`, '') = '';

CREATE VIEW IF NOT EXISTS fishing_zone_slots AS
SELECT R,G,B, 1 AS slot_idx, DropRate1 AS drop_rate, DropID1 AS item_main_group_key FROM fishing_table
UNION ALL SELECT R,G,B, 2, DropRate2, DropID2 FROM fishing_table
UNION ALL SELECT R,G,B, 3, DropRate3, DropID3 FROM fishing_table
UNION ALL SELECT R,G,B, 4, DropRate4, DropID4 FROM fishing_table
UNION ALL SELECT R,G,B, 5, DropRate5, DropID5 FROM fishing_table;

CREATE VIEW IF NOT EXISTS item_main_group_options AS
SELECT ItemMainGroupKey AS item_main_group_key, 0 AS option_idx, SelectRate0 AS select_rate, Condition0 AS condition_raw, ItemSubGroupKey0 AS item_sub_group_key
FROM item_main_group_table WHERE ItemSubGroupKey0 IS NOT NULL
UNION ALL
SELECT ItemMainGroupKey, 1, SelectRate1, Condition1, ItemSubGroupKey1
FROM item_main_group_table WHERE ItemSubGroupKey1 IS NOT NULL
UNION ALL
SELECT ItemMainGroupKey, 2, SelectRate2, Condition2, ItemSubGroupKey2
FROM item_main_group_table WHERE ItemSubGroupKey2 IS NOT NULL
UNION ALL
SELECT ItemMainGroupKey, 3, SelectRate3, Condition3, ItemSubGroupKey3
FROM item_main_group_table WHERE ItemSubGroupKey3 IS NOT NULL;

CREATE VIEW IF NOT EXISTS item_sub_group_item_variants AS
SELECT ItemSubGroupKey AS item_sub_group_key, ItemKey AS item_key, EnchantLevel AS enchant_level, 0 AS variant_idx,
       SelectRate_0 AS select_rate, MinCount_0 AS min_count, MaxCount_0 AS max_count
FROM item_sub_group_table WHERE SelectRate_0 IS NOT NULL AND SelectRate_0 > 0
UNION ALL
SELECT ItemSubGroupKey, ItemKey, EnchantLevel, 1, SelectRate_1, MinCount_1, MaxCount_1
FROM item_sub_group_table WHERE SelectRate_1 IS NOT NULL AND SelectRate_1 > 0
UNION ALL
SELECT ItemSubGroupKey, ItemKey, EnchantLevel, 2, SelectRate_2, MinCount_2, MaxCount_2
FROM item_sub_group_table WHERE SelectRate_2 IS NOT NULL AND SelectRate_2 > 0;

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

DROP VIEW IF EXISTS calculator_effect_source_entries;
CREATE VIEW calculator_effect_source_entries AS
SELECT source_key,
       'item' AS source_kind,
       item_id,
       NULL AS legacy_name_en,
       item_name_ko AS source_name_ko,
       COALESCE(effect_description_ko, item_description_ko) AS effect_description_ko
FROM calculator_consumable_effect_sources
UNION ALL
SELECT source_key,
       'lightstone_set' AS source_kind,
       NULL AS item_id,
       legacy_name_en,
       set_name_ko AS source_name_ko,
       effect_description_ko
FROM calculator_lightstone_effect_sources
WHERE legacy_name_en IS NOT NULL;

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

CREATE TABLE IF NOT EXISTS layers (
  layer_id VARCHAR(64) NOT NULL,
  name VARCHAR(128) NOT NULL,
  enabled TINYINT(1) NOT NULL DEFAULT 1,
  ui_display_order INT NOT NULL DEFAULT 0,
  visible_default TINYINT(1) NOT NULL DEFAULT 1,
  opacity_default DOUBLE NOT NULL DEFAULT 1.0,
  z_base DOUBLE NOT NULL DEFAULT 0.0,
  transform_kind VARCHAR(32) NOT NULL,
  affine_a DOUBLE NULL,
  affine_b DOUBLE NULL,
  affine_tx DOUBLE NULL,
  affine_c DOUBLE NULL,
  affine_d DOUBLE NULL,
  affine_ty DOUBLE NULL,
  tileset_manifest_url VARCHAR(512) NOT NULL,
  tile_url_template VARCHAR(512) NOT NULL,
  tileset_version VARCHAR(64) NOT NULL DEFAULT '',
  tile_px INT NOT NULL DEFAULT 512,
  max_level INT NOT NULL DEFAULT 0,
  y_flip TINYINT(1) NOT NULL DEFAULT 0,
  request_weight DOUBLE NOT NULL DEFAULT 1.0,
  pick_mode VARCHAR(32) NOT NULL DEFAULT 'none',
  layer_kind VARCHAR(32) NOT NULL DEFAULT 'tiled_raster',
  field_source_url VARCHAR(512) NULL,
  field_source_revision VARCHAR(128) NULL,
  field_color_mode VARCHAR(32) NOT NULL DEFAULT 'rgb_u24',
  field_metadata_source_url VARCHAR(512) NULL,
  field_metadata_source_revision VARCHAR(128) NULL,
  vector_source_url VARCHAR(512) NULL,
  vector_source_revision VARCHAR(128) NULL,
  vector_geometry_space VARCHAR(32) NOT NULL DEFAULT 'map_pixels',
  vector_style_mode VARCHAR(64) NOT NULL DEFAULT 'feature_property_palette',
  vector_feature_id_property VARCHAR(128) NULL,
  vector_color_property VARCHAR(128) NULL,
  lod_target_tiles INT NOT NULL DEFAULT 220,
  lod_hysteresis_hi DOUBLE NOT NULL DEFAULT 280.0,
  lod_hysteresis_lo DOUBLE NOT NULL DEFAULT 160.0,
  lod_margin_tiles INT NOT NULL DEFAULT 1,
  lod_enable_refine TINYINT(1) NOT NULL DEFAULT 0,
  lod_refine_debounce_ms INT NOT NULL DEFAULT 0,
  lod_max_detail_tiles INT NOT NULL DEFAULT 0,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (layer_id)
);

CREATE TABLE IF NOT EXISTS layer_configs (
  map_version_id VARCHAR(64) NOT NULL,
  layer_id VARCHAR(64) NOT NULL,
  enabled_override TINYINT(1) NULL,
  visible_default_override TINYINT(1) NULL,
  opacity_default_override DOUBLE NULL,
  z_base_override DOUBLE NULL,
  tileset_manifest_url_override VARCHAR(512) NULL,
  tile_url_template_override VARCHAR(512) NULL,
  tileset_version_override VARCHAR(64) NULL,
  vector_source_url_override VARCHAR(512) NULL,
  vector_source_revision_override VARCHAR(128) NULL,
  max_level_override INT NULL,
  request_weight_override DOUBLE NULL,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (map_version_id, layer_id),
  KEY idx_layer_configs_layer (layer_id)
);

CREATE TABLE IF NOT EXISTS region_group_meta (
  map_version_id VARCHAR(64) NOT NULL,
  region_group_id INT NOT NULL,
  color_rgb_u32 INT UNSIGNED NULL,
  feature_count INT NOT NULL DEFAULT 0,
  region_count INT NOT NULL DEFAULT 0,
  accessible_region_count INT NOT NULL DEFAULT 0,
  bbox_min_x DOUBLE NULL,
  bbox_min_y DOUBLE NULL,
  bbox_max_x DOUBLE NULL,
  bbox_max_y DOUBLE NULL,
  graph_world_x DOUBLE NULL,
  graph_world_z DOUBLE NULL,
  source VARCHAR(64) NOT NULL DEFAULT '',
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (map_version_id, region_group_id)
);

CREATE TABLE IF NOT EXISTS region_group_regions (
  map_version_id VARCHAR(64) NOT NULL,
  region_group_id INT NOT NULL,
  region_id INT NOT NULL,
  trade_origin_region INT NULL,
  is_accessible TINYINT(1) NOT NULL DEFAULT 0,
  waypoint INT NULL,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (map_version_id, region_group_id, region_id),
  KEY idx_region_group_regions_region (map_version_id, region_id)
);

INSERT INTO layers (
  layer_id, name, enabled, ui_display_order, visible_default, opacity_default, z_base,
  transform_kind, affine_a, affine_b, affine_tx, affine_c, affine_d, affine_ty,
  tileset_manifest_url, tile_url_template, tileset_version,
  tile_px, max_level, y_flip, request_weight, pick_mode,
  layer_kind, field_source_url, field_source_revision, field_color_mode,
  field_metadata_source_url, field_metadata_source_revision,
  vector_source_url, vector_source_revision, vector_geometry_space, vector_style_mode,
  vector_feature_id_property, vector_color_property,
  lod_target_tiles, lod_hysteresis_hi, lod_hysteresis_lo, lod_margin_tiles,
  lod_enable_refine, lod_refine_debounce_ms, lod_max_detail_tiles
) VALUES
  (
    'minimap', 'Minimap', 1, 0, 1, 1.0, 0.0,
    'affine_to_world', 100.0, 0.0, 0.0, 0.0, 100.0, 0.0,
    '/images/tiles/minimap/v1/tileset.json', '/images/tiles/minimap/v1/{level}/rader_{x}_{y}.png', 'v1',
    128, 6, 1, 1.0, 'none',
    'tiled_raster', NULL, NULL, 'rgb_u24', NULL, NULL, NULL, NULL, 'map_pixels', 'feature_property_palette', NULL, NULL,
    256, 320.0, 192.0, 2,
    1, 150, 256
  ),
  (
    'zone_mask', 'Zone Mask', 1, 10, 1, 0.35, 10.0,
    'identity_map_space', NULL, NULL, NULL, NULL, NULL, NULL,
    '/images/tiles/mask/{map_version}/tileset.json', '/images/tiles/mask/{map_version}/{level}/{x}_{y}.png', 'v1',
    512, 0, 0, 0.7, 'exact_tile_pixel',
    'tiled_raster', NULL, NULL, 'rgb_u24',
    '/fields/zone_mask.{map_version}.meta.json', 'zone-meta-v1',
    NULL, NULL, 'map_pixels', 'feature_property_palette', NULL, NULL,
    300, 360.0, 220.0, 1,
    0, 0, 0
  ),
  (
    'region_groups', 'Region Groups', 1, 30, 0, 0.50, 30.0,
    'identity_map_space', NULL, NULL, NULL, NULL, NULL, NULL,
    '', '', '',
    512, 0, 0, 0.4, 'none',
    'vector_geojson', '/fields/region_groups.{map_version}.bin', 'rg-field-v1', 'debug_hash',
    '/fields/region_groups.{map_version}.meta.json', 'rg-meta-v1',
    '/region_groups/{map_version}.geojson', 'rg-v1', 'map_pixels', 'feature_property_palette', 'id', 'c',
    220, 280.0, 160.0, 1,
    0, 0, 0
  ),
  (
    'regions', 'Regions', 1, 31, 0, 0.35, 31.0,
    'identity_map_space', NULL, NULL, NULL, NULL, NULL, NULL,
    '', '', '',
    512, 0, 0, 0.45, 'none',
    'vector_geojson', '/fields/regions.{map_version}.bin', 'r-field-v1', 'debug_hash',
    '/fields/regions.{map_version}.meta.json', 'r-meta-v1',
    '/region_groups/regions.{map_version}.geojson', 'r-v1', 'map_pixels', 'feature_property_palette', 'r', 'c',
    220, 280.0, 160.0, 1,
    0, 0, 0
  )
ON DUPLICATE KEY UPDATE
  name = VALUES(name),
  enabled = VALUES(enabled),
  ui_display_order = VALUES(ui_display_order),
  visible_default = VALUES(visible_default),
  opacity_default = VALUES(opacity_default),
  z_base = VALUES(z_base),
  transform_kind = VALUES(transform_kind),
  affine_a = VALUES(affine_a),
  affine_b = VALUES(affine_b),
  affine_tx = VALUES(affine_tx),
  affine_c = VALUES(affine_c),
  affine_d = VALUES(affine_d),
  affine_ty = VALUES(affine_ty),
  tileset_manifest_url = VALUES(tileset_manifest_url),
  tile_url_template = VALUES(tile_url_template),
  tileset_version = VALUES(tileset_version),
  tile_px = VALUES(tile_px),
  max_level = VALUES(max_level),
  y_flip = VALUES(y_flip),
  request_weight = VALUES(request_weight),
  pick_mode = VALUES(pick_mode),
  layer_kind = VALUES(layer_kind),
  field_source_url = VALUES(field_source_url),
  field_source_revision = VALUES(field_source_revision),
  field_color_mode = VALUES(field_color_mode),
  field_metadata_source_url = VALUES(field_metadata_source_url),
  field_metadata_source_revision = VALUES(field_metadata_source_revision),
  vector_source_url = VALUES(vector_source_url),
  vector_source_revision = VALUES(vector_source_revision),
  vector_geometry_space = VALUES(vector_geometry_space),
  vector_style_mode = VALUES(vector_style_mode),
  vector_feature_id_property = VALUES(vector_feature_id_property),
  vector_color_property = VALUES(vector_color_property),
  lod_target_tiles = VALUES(lod_target_tiles),
  lod_hysteresis_hi = VALUES(lod_hysteresis_hi),
  lod_hysteresis_lo = VALUES(lod_hysteresis_lo),
  lod_margin_tiles = VALUES(lod_margin_tiles),
  lod_enable_refine = VALUES(lod_enable_refine),
  lod_refine_debounce_ms = VALUES(lod_refine_debounce_ms),
  lod_max_detail_tiles = VALUES(lod_max_detail_tiles);

INSERT INTO layer_configs (
  map_version_id, layer_id,
  z_base_override,
  tileset_manifest_url_override, tile_url_template_override, tileset_version_override,
  vector_source_url_override, vector_source_revision_override
) VALUES
  ('v1', 'minimap', 0.0, '/images/tiles/minimap/v1/tileset.json', '/images/tiles/minimap/v1/{level}/rader_{x}_{y}.png', 'v1', NULL, NULL),
  ('v1', 'zone_mask', 10.0, '/images/tiles/mask/v1/tileset.json', '/images/tiles/mask/v1/{level}/{x}_{y}.png', 'v1', NULL, NULL),
  ('v1', 'region_groups', 30.0, '', '', '', '/region_groups/v1.geojson', 'rg-v1'),
  ('v1', 'regions', 31.0, '', '', '', '/region_groups/regions.v1.geojson', 'r-v1')
ON DUPLICATE KEY UPDATE
  z_base_override = VALUES(z_base_override),
  tileset_manifest_url_override = VALUES(tileset_manifest_url_override),
  tile_url_template_override = VALUES(tile_url_template_override),
  tileset_version_override = VALUES(tileset_version_override),
  vector_source_url_override = VALUES(vector_source_url_override),
  vector_source_revision_override = VALUES(vector_source_revision_override);
