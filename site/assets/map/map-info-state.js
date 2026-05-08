import {
  buildOverviewRowsForLayerSamples,
  buildTerritoryPaneFacts,
  buildTradePaneFacts,
  buildZonePaneFacts,
  preferredOverviewRow,
} from "./map-overview-facts.js";
import { buildPointSampleRows } from "./map-hover-facts.js";
import { mapText } from "./map-i18n.js";

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function trimString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function normalizeCatchMethods(value) {
  if (!Array.isArray(value)) {
    return [];
  }
  const methods = [];
  for (const rawMethod of value) {
    const normalized = trimString(rawMethod).toLowerCase();
    if ((normalized === "rod" || normalized === "harpoon") && !methods.includes(normalized)) {
      methods.push(normalized);
    }
  }
  return methods;
}

function normalizePointKind(value) {
  const normalized = trimString(value).toLowerCase();
  if (normalized === "bookmark" || normalized === "waypoint" || normalized === "clicked") {
    return normalized;
  }
  return "";
}

function normalizeSelectionLayerSamples(selection) {
  if (!Array.isArray(selection?.layerSamples)) {
    return [];
  }
  return selection.layerSamples.filter((sample) => isPlainObject(sample)).map((sample) => cloneJson(sample));
}

const INFO_WINDOW_TITLE_ICON = "inspect-fill";
const INFO_WINDOW_STATUS_ICON = "information-circle";
const HOTSPOTS_LAYER_ID = "hotspots";
const HOTSPOT_KIND = "hotspot";
const HOTSPOT_INFO_ICON = "fish-fill";
const HOTSPOT_UNSET_STAT_VALUE = "Not set";
const LANDMARK_LAYER_IDS = Object.freeze([
  "trade_npcs",
  "region_nodes",
  "bookmarks",
]);
const LANDMARK_STATUS_PRESENTATION_BY_KEY = Object.freeze({
  bookmark: { icon: "bookmark", labelKey: "info.status.bookmark" },
  hotspot: { icon: HOTSPOT_INFO_ICON, label: "Hotspot" },
  npc: { icon: "trade-origin", label: "NPC" },
  sample: { icon: "date-confirmed", label: "Sample" },
  trade_npc: { icon: "trade-origin", label: "NPC" },
  waypoint: { icon: "map-pin", labelKey: "info.status.waypoint" },
});
const LANDMARK_STATUS_PRESENTATION_BY_LAYER = Object.freeze({
  bookmarks: LANDMARK_STATUS_PRESENTATION_BY_KEY.bookmark,
  hotspots: LANDMARK_STATUS_PRESENTATION_BY_KEY.hotspot,
  trade_npcs: LANDMARK_STATUS_PRESENTATION_BY_KEY.trade_npc,
});

function landmarkStatusPresentationLabel(presentation) {
  return trimString(presentation?.label) || mapText(presentation?.labelKey);
}

function landmarkStatusPresentationFromKey(key) {
  return LANDMARK_STATUS_PRESENTATION_BY_KEY[trimString(key).toLowerCase()] || null;
}

function pointKindStatusText(pointKind, pointLabel) {
  const normalizedLabel = trimString(pointLabel);
  switch (normalizePointKind(pointKind)) {
    case "bookmark":
      return normalizedLabel || mapText("info.status.bookmark");
    case "waypoint":
      return normalizedLabel || mapText("info.status.waypoint");
    case "clicked":
      return mapText("info.status.clicked");
    default:
      return mapText("info.status.no_selection");
  }
}

function landmarkStatusPresentationFromSelection(selection, layerSamples) {
  const elementKindPresentation = landmarkStatusPresentationFromKey(selectionTargetElementKind(selection));
  if (elementKindPresentation) {
    return elementKindPresentation;
  }
  for (const sample of layerSamples) {
    for (const target of Array.isArray(sample?.targets) ? sample.targets : []) {
      const targetPresentation = landmarkStatusPresentationFromKey(target?.key);
      if (targetPresentation) {
        return targetPresentation;
      }
    }
  }
  const hotspotSample = layerSamples.find(isHotspotSample);
  if (hotspotSample) {
    return LANDMARK_STATUS_PRESENTATION_BY_KEY.hotspot;
  }
  const landmarkSample = layerSamples.find(isLandmarkSample);
  if (landmarkSample) {
    return (
      LANDMARK_STATUS_PRESENTATION_BY_LAYER[trimString(landmarkSample?.layerId)] ||
      LANDMARK_STATUS_PRESENTATION_BY_KEY.waypoint
    );
  }
  return null;
}

function selectionStatusDescriptor(selection, layerSamples) {
  const landmarkPresentation = landmarkStatusPresentationFromSelection(selection, layerSamples);
  const landmarkStatusText = landmarkStatusPresentationLabel(landmarkPresentation);
  if (landmarkStatusText) {
    return {
      statusIcon: trimString(landmarkPresentation?.icon) || INFO_WINDOW_STATUS_ICON,
      statusText: landmarkStatusText,
    };
  }
  const pointKind = normalizePointKind(selection?.pointKind);
  const targetPointLabel = selectionTargetPointLabel(selection);
  return {
    statusIcon: INFO_WINDOW_STATUS_ICON,
    statusText: pointKindStatusText(pointKind, targetPointLabel || selection?.pointLabel),
  };
}

function selectionDetailsTarget(selection) {
  return isPlainObject(selection?.detailsTarget) ? selection.detailsTarget : null;
}

function selectionTargetElementKind(selection) {
  return trimString(selectionDetailsTarget(selection)?.elementKind).toLowerCase();
}

function selectionTargetPointLabel(selection) {
  return trimString(selectionDetailsTarget(selection)?.pointLabel);
}

function titleFromSelection(selection, layerSamples, zoneCatalog, runtimeLayers) {
  const elementKind = selectionTargetElementKind(selection);
  const targetPointLabel = selectionTargetPointLabel(selection);
  const landmarkPresentation = landmarkStatusPresentationFromKey(elementKind);
  if (landmarkPresentation) {
    if (targetPointLabel) {
      return targetPointLabel;
    }
    return landmarkStatusPresentationLabel(landmarkPresentation);
  }
  const pointLabel = trimString(selection?.pointLabel);
  const pointKind = normalizePointKind(selection?.pointKind);
  if ((pointKind === "bookmark" || pointKind === "waypoint") && pointLabel) {
    return pointLabel;
  }
  const preferred = preferredOverviewRow(layerSamples, {
    zoneCatalog,
    runtimeLayers,
  });
  if (preferred?.value) {
    return preferred.value;
  }
  if (pointLabel) {
    return pointLabel;
  }
  return mapText("info.window_title");
}

function preferredPaneIdFromSelection(selection) {
  switch (selectionTargetElementKind(selection)) {
    case "hotspot":
      return "hotspot";
    case "sample":
    case "point":
      return "samples";
    default:
      return "";
  }
}

function paneIdExists(panes, paneId) {
  return Boolean(paneId) && panes.some((pane) => pane.id === paneId);
}

function zoneLootConditionKey(group, index) {
  const explicitKey = trimString(group?.conditionOptionKey);
  if (explicitKey) {
    return explicitKey;
  }
  const slotIdx = Number.parseInt(group?.slotIdx, 10) || index + 1;
  const label = trimString(group?.label) || `slot-${slotIdx}`;
  return `${slotIdx}:${label}`;
}

function clampZoneLootConditionIndex(value, count) {
  const index = Number.parseInt(value, 10);
  if (!Number.isInteger(index) || index < 0 || index >= count) {
    return null;
  }
  return index;
}

function normalizeRatesFromSignals(signals) {
  return signals?._map_ui?.windowUi?.settings?.normalizeRates !== false;
}

function zoneLootRateDisplayFields(row, normalizeRates = true) {
  const preferredText = normalizeRates
    ? trimString(row?.normalizedDropRateText)
    : trimString(row?.rawDropRateText);
  const preferredTooltip = normalizeRates
    ? trimString(row?.normalizedDropRateTooltip)
    : trimString(row?.rawDropRateTooltip);
  return {
    dropRateText: preferredText,
    dropRateTooltip: preferredTooltip,
  };
}

function normalizeZoneLootSpeciesRow(row, normalizeRates = true) {
  const rateFields = zoneLootRateDisplayFields(row, normalizeRates);
  return {
    ...cloneJson(row),
    ...rateFields,
    catchMethods: normalizeCatchMethods(row?.catchMethods),
  };
}

function normalizeZoneLootConditionOptions(group, normalizeRates = true) {
  if (!Array.isArray(group?.conditionOptions)) {
    return [];
  }
  return group.conditionOptions
    .filter((option) => isPlainObject(option))
    .map((option) => ({
      conditionText: trimString(option?.conditionText),
      conditionTooltip: trimString(option?.conditionTooltip),
      ...zoneLootRateDisplayFields(option, normalizeRates),
      dropRateSourceKind: trimString(option?.dropRateSourceKind),
      presenceText: trimString(option?.presenceText),
      presenceSourceKind: trimString(option?.presenceSourceKind),
      presenceTooltip: trimString(option?.presenceTooltip),
      active: option?.active === true,
      speciesRows: Array.isArray(option?.speciesRows)
        ? option.speciesRows.map((row) => normalizeZoneLootSpeciesRow(row, normalizeRates))
        : [],
    }))
    .filter((option) => option.conditionText || option.speciesRows.length > 0);
}

function rateLineageDetailFromTooltip(value) {
  const detail = trimString(value);
  if (!detail) {
    return "";
  }
  const lineageStarts = ["main group ", "subgroup "]
    .map((needle) => detail.indexOf(needle))
    .filter((index) => index >= 0);
  if (!lineageStarts.length) {
    return "";
  }
  return detail.slice(Math.min(...lineageStarts)).trim();
}

function zoneLootRowsRateLineageDetail(rows) {
  if (!Array.isArray(rows)) {
    return "";
  }
  const details = [];
  for (const row of rows) {
    const detail = rateLineageDetailFromTooltip(row?.dropRateTooltip);
    if (detail && !details.includes(detail)) {
      details.push(detail);
    }
  }
  return details.join(" | ");
}

function selectedZoneLootConditionIndex(options, conditionSelection, key) {
  if (!options.length) {
    return -1;
  }
  const explicitIndex = isPlainObject(conditionSelection)
    ? clampZoneLootConditionIndex(conditionSelection[key], options.length)
    : null;
  if (explicitIndex != null) {
    return explicitIndex;
  }
  const activeIndex = options.findIndex((option) => option.active === true);
  return activeIndex >= 0 ? activeIndex : 0;
}

function buildZoneLootGroups(summary, conditionSelection = {}, normalizeRates = true) {
  const groups = Array.isArray(summary?.groups) ? summary.groups : [];
  const speciesRows = Array.isArray(summary?.speciesRows) ? summary.speciesRows : [];
  if (!groups.length && !speciesRows.length) {
    return [];
  }
  return groups
    .map((group, index) => {
      const slotIdx = Number.parseInt(group?.slotIdx, 10) || index + 1;
      const groupLabel = trimString(group?.label);
      const groupRateFields = zoneLootRateDisplayFields(group, normalizeRates);
      const conditionOptions = normalizeZoneLootConditionOptions(group, normalizeRates);
      const conditionOptionKey = zoneLootConditionKey(group, index);
      const conditionOptionIndex = selectedZoneLootConditionIndex(
        conditionOptions,
        conditionSelection,
        conditionOptionKey,
      );
      const selectedCondition =
        conditionOptionIndex >= 0 ? conditionOptions[conditionOptionIndex] : null;
      const selectedConditionRows = selectedCondition
        ? selectedCondition.speciesRows.map((row) => normalizeZoneLootSpeciesRow(row, normalizeRates))
        : null;
      const selectedConditionRateLineage =
        zoneLootRowsRateLineageDetail(selectedConditionRows);
      const fallbackRows = speciesRows
        .filter((row) => {
          const rowGroupLabel = trimString(row?.groupLabel);
          if (rowGroupLabel && groupLabel && rowGroupLabel === groupLabel) {
            return true;
          }
          return (Number.parseInt(row?.slotIdx, 10) || 0) === slotIdx;
        })
        .map((row) => normalizeZoneLootSpeciesRow(row, normalizeRates));
      const selectedConditionRateFields = selectedCondition || null;
      return {
        slotIdx,
        label: groupLabel,
        fillColor: trimString(group?.fillColor),
        strokeColor: trimString(group?.strokeColor),
        textColor: trimString(group?.textColor),
        dropRateText: trimString(selectedConditionRateFields?.dropRateText) || groupRateFields.dropRateText,
        dropRateSourceKind:
          trimString(selectedCondition?.dropRateSourceKind) || trimString(group?.dropRateSourceKind),
        dropRateTooltip:
          selectedConditionRateLineage ||
          trimString(selectedConditionRateFields?.dropRateTooltip) ||
          groupRateFields.dropRateTooltip,
        presenceText: trimString(selectedCondition?.presenceText) || trimString(group?.presenceText),
        presenceSourceKind:
          trimString(selectedCondition?.presenceSourceKind) || trimString(group?.presenceSourceKind),
        presenceTooltip: trimString(selectedCondition?.presenceTooltip) || trimString(group?.presenceTooltip),
        conditionText: trimString(selectedCondition?.conditionText) || trimString(group?.conditionText),
        conditionTooltip:
          trimString(selectedCondition?.conditionTooltip) || trimString(group?.conditionTooltip),
        catchMethods: normalizeCatchMethods(group?.catchMethods),
        conditionOptions,
        conditionOptionIndex,
        conditionOptionKey,
        rows: selectedConditionRows || fallbackRows,
      };
    })
    .filter((group) => group.label);
}

function zoneLootMethodLabel(method) {
  return method === "harpoon"
    ? mapText("info.zone_loot.method.harpoon")
    : mapText("info.zone_loot.method.fishing");
}

function zoneLootMethodNote(method) {
  return method === "harpoon"
    ? mapText("info.zone_loot.method_note.harpoon")
    : mapText("info.zone_loot.method_note.fishing");
}

function rowMatchesZoneLootMethod(row, method) {
  const methods = normalizeCatchMethods(row?.catchMethods);
  if (method === "harpoon") {
    return methods.includes("harpoon");
  }
  return methods.length === 0 || methods.includes("rod");
}

function groupMatchesZoneLootMethod(group, method) {
  const methods = normalizeCatchMethods(group?.catchMethods);
  if (method === "harpoon") {
    return methods.includes("harpoon");
  }
  return methods.length === 0 || methods.includes("rod");
}

function buildZoneLootMethodProfiles(groups) {
  const profiles = [];

  const fishingGroups = groups
    .map((group) => ({
      ...cloneJson(group),
      rows: (Array.isArray(group?.rows) ? group.rows : []).filter((row) => rowMatchesZoneLootMethod(row, "rod")),
    }))
    .filter(
      (group) =>
        groupMatchesZoneLootMethod(group, "rod") &&
        (group.rows.length > 0 || trimString(group?.dropRateText)),
    );
  if (fishingGroups.length) {
    profiles.push({
      method: "rod",
      label: zoneLootMethodLabel("rod"),
      note: zoneLootMethodNote("rod"),
      groups: fishingGroups,
    });
  }

  const harpoonGroups = groups
    .map((group) => ({
      ...cloneJson(group),
      rows: (Array.isArray(group?.rows) ? group.rows : [])
        .filter((row) => rowMatchesZoneLootMethod(row, "harpoon")),
    }))
    .filter(
      (group) =>
        groupMatchesZoneLootMethod(group, "harpoon") &&
        (group.rows.length > 0 || trimString(group?.dropRateText)),
    );
  if (harpoonGroups.length) {
    profiles.push({
      method: "harpoon",
      label: zoneLootMethodLabel("harpoon"),
      note: zoneLootMethodNote("harpoon"),
      groups: harpoonGroups,
    });
  }

  return profiles;
}

function buildZoneLootSection(
  summary,
  status,
  conditionSelection = {},
  normalizeRates = true,
) {
  const normalizedStatus = trimString(status || "idle");
  if (normalizedStatus === "idle" && !isPlainObject(summary)) {
    return null;
  }
  const groups = buildZoneLootGroups(summary, conditionSelection, normalizeRates);
  const profiles = buildZoneLootMethodProfiles(groups);
  const available = summary?.available === true && profiles.some((profile) => profile.groups.length > 0);
  const statusText =
    normalizedStatus === "loaded"
      ? mapText("info.zone_loot.status.loaded")
      : normalizedStatus === "loading"
        ? mapText("info.zone_loot.status.loading")
        : normalizedStatus === "error"
          ? mapText("info.zone_loot.status.error")
          : normalizedStatus
            ? mapText("info.zone_loot.status.other", { status: normalizedStatus })
            : mapText("info.zone_loot.status.idle");

  return {
    id: "zone-loot",
    kind: "zone-loot",
    title: mapText("info.zone_loot.title"),
    statusText,
    summary: trimString(summary?.profileLabel) || mapText("info.zone_loot.summary.default"),
    dataQualityNote: trimString(summary?.dataQualityNote),
    note: trimString(summary?.note),
    groups,
    profiles,
    available,
  };
}

function buildPointSampleSection(selection, stateBundle, zoneCatalog) {
  const rows = buildPointSampleRows({ source: selection, stateBundle, zoneCatalog });
  if (!rows.length) {
    return null;
  }
  return {
    id: "point-samples",
    kind: "point-samples",
    title: "Ranking Samples",
    rows,
  };
}

function integerValue(value) {
  const parsed = Number.parseInt(value, 10);
  return Number.isInteger(parsed) ? parsed : null;
}

function fishItemIconUrl(itemId) {
  const parsed = integerValue(itemId);
  if (parsed == null || parsed <= 0) {
    return "";
  }
  const resolver = globalThis.window?.__fishystuffResolveFishItemIconUrl;
  if (typeof resolver === "function") {
    return trimString(resolver(parsed));
  }
  return `/images/items/${String(parsed).padStart(8, "0")}.webp`;
}

function detailSectionFacts(section) {
  return (Array.isArray(section?.facts) ? section.facts : [])
    .filter((fact) => isPlainObject(fact))
    .map((fact) => ({
      key: trimString(fact.key),
      label: trimString(fact.label),
      value: trimString(fact.value),
      icon: trimString(fact.icon),
    }))
    .filter((fact) => fact.key && fact.value);
}

function firstFactValue(facts, key) {
  return facts.find((fact) => fact.key === key)?.value || "";
}

function hotspotMetric(label, value, icon) {
  const normalizedValue = trimString(value);
  return {
    label,
    value: normalizedValue || HOTSPOT_UNSET_STAT_VALUE,
    icon,
  };
}

function isHotspotSample(sample) {
  return (
    trimString(sample?.layerId) === HOTSPOTS_LAYER_ID ||
    trimString(sample?.kind) === HOTSPOT_KIND
  );
}

function hotspotDetailSection(sample) {
  return (Array.isArray(sample?.detailSections) ? sample.detailSections : [])
    .find((section) => isPlainObject(section) && trimString(section.kind) === "hotspot") || null;
}

function formatHotspotLifeLevel(value) {
  const raw = integerValue(value);
  if (raw == null || raw < 0) {
    return "";
  }
  const levelIndex = raw + 1;
  const tiers = [
    ["Beginner", 10],
    ["Apprentice", 10],
    ["Skilled", 10],
    ["Professional", 10],
    ["Artisan", 10],
    ["Master", 30],
    ["Guru", 100],
  ];
  let remaining = levelIndex;
  for (const [tier, count] of tiers) {
    if (remaining <= count) {
      return `${tier} ${remaining}`;
    }
    remaining -= count;
  }
  return `Guru ${Math.max(1, remaining + 100)}`;
}

function formatHotspotSeconds(value) {
  const ms = integerValue(value);
  if (ms == null || ms < 0) {
    return "";
  }
  const seconds = ms / 1000;
  return `${seconds.toFixed(1).replace(/\.0$/, "")}s`;
}

function formatHotspotAverageSeconds(minValue, maxValue) {
  const minMs = integerValue(minValue);
  const maxMs = integerValue(maxValue);
  if (minMs == null || maxMs == null || minMs < 0 || maxMs < 0) {
    return "";
  }
  return formatHotspotSeconds(Math.round((minMs + maxMs) / 2));
}

function formatHotspotDuration(value) {
  const ms = integerValue(value);
  if (ms == null || ms < 0) {
    return "";
  }
  const totalSeconds = Math.round(ms / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  }
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

function formatHotspotRate(value) {
  const rate = Number(value);
  if (!Number.isFinite(rate) || rate < 0) {
    return "";
  }
  const percent = rate / 10_000;
  return `${percent.toFixed(3).replace(/\.?0+$/, "")}%`;
}

function hotspotGradeTone(gradeType) {
  switch (integerValue(gradeType)) {
    case 4:
      return "red";
    case 3:
      return "yellow";
    case 2:
      return "blue";
    case 1:
      return "green";
    case 0:
      return "white";
    default:
      return "unknown";
  }
}

function parseHotspotJsonPayload(value) {
  let payload = null;
  try {
    payload = JSON.parse(value);
  } catch (_error) {
    return null;
  }
  return isPlainObject(payload) ? payload : null;
}

function parseHotspotLootPayload(payload, keySuffix = "", { slotIdx = 0, groupLabel = "" } = {}) {
  if (!isPlainObject(payload)) {
    return null;
  }
  const itemId = integerValue(payload.itemId);
  const iconItemId = integerValue(payload.iconItemId) || itemId;
  const label = trimString(payload.label) || trimString(payload.name);
  if (!itemId || !label) {
    return null;
  }
  const iconGradeTone = trimString(payload.iconGradeTone) || hotspotGradeTone(payload.gradeType);
  const dropRateText = trimString(payload.dropRateText) || formatHotspotRate(payload.selectRate);
  const dropRateTooltip = trimString(payload.dropRateTooltip);
  return {
    key: `hotspot-loot:${keySuffix}:${itemId}:${trimString(payload.selectRate)}`,
    itemId,
    label,
    slotIdx: integerValue(payload.slotIdx) || slotIdx,
    groupLabel: trimString(payload.groupLabel) || groupLabel,
    iconUrl: iconItemId ? fishItemIconUrl(iconItemId) : "",
    iconGradeTone,
    fillColor: trimString(payload.fillColor),
    strokeColor: trimString(payload.strokeColor),
    textColor: trimString(payload.textColor),
    dropRateText,
    dropRateSourceKind: trimString(payload.dropRateSourceKind),
    dropRateTooltip,
    rawDropRateText: trimString(payload.rawDropRateText) || dropRateText,
    rawDropRateTooltip: trimString(payload.rawDropRateTooltip) || dropRateTooltip,
    normalizedDropRateText: trimString(payload.normalizedDropRateText) || dropRateText,
    normalizedDropRateTooltip: trimString(payload.normalizedDropRateTooltip) || dropRateTooltip,
    catchMethods: normalizeCatchMethods(payload.catchMethods),
  };
}

function parseHotspotLootFact(fact) {
  if (fact.key !== "loot_item") {
    return null;
  }
  return parseHotspotLootPayload(parseHotspotJsonPayload(fact.value));
}

function parseHotspotLootGroupFact(fact, index) {
  if (fact.key !== "loot_group") {
    return null;
  }
  const payload = parseHotspotJsonPayload(fact.value);
  if (!payload) {
    return null;
  }
  const slotIdx = integerValue(payload.slotIdx) || index + 1;
  const label = trimString(payload.label) || `Group ${index + 1}`;
  const conditionOptions = (Array.isArray(payload.conditionOptions) ? payload.conditionOptions : [])
    .filter((option) => isPlainObject(option))
    .map((option, optionIndex) => {
      const rows = (Array.isArray(option.speciesRows) ? option.speciesRows : [])
        .map((row, rowIndex) =>
          parseHotspotLootPayload(
            row,
            `${trimString(option.conditionKey) || optionIndex}:${rowIndex}`,
            { slotIdx, groupLabel: label },
          ),
        )
        .filter(Boolean);
      const optionDropRateText = trimString(option.dropRateText);
      const optionDropRateTooltip = trimString(option.dropRateTooltip);
      return {
        conditionText: trimString(option.conditionText),
        conditionTooltip: trimString(option.conditionTooltip),
        dropRateText: optionDropRateText,
        dropRateSourceKind: trimString(option.dropRateSourceKind),
        dropRateTooltip: optionDropRateTooltip,
        rawDropRateText: trimString(option.rawDropRateText) || optionDropRateText,
        rawDropRateTooltip: trimString(option.rawDropRateTooltip) || optionDropRateTooltip,
        normalizedDropRateText: trimString(option.normalizedDropRateText) || optionDropRateText,
        normalizedDropRateTooltip: trimString(option.normalizedDropRateTooltip) || optionDropRateTooltip,
        active: option.active === true,
        speciesRows: rows,
      };
    })
    .filter((option) => option.conditionText || option.speciesRows.length > 0);
  if (!conditionOptions.length) {
    return null;
  }
  const groupDropRateText = trimString(payload.dropRateText);
  const groupDropRateTooltip = trimString(payload.dropRateTooltip);
  return {
    slotIdx,
    label,
    conditionOptionKey: trimString(payload.conditionOptionKey),
    fillColor: trimString(payload.fillColor),
    strokeColor: trimString(payload.strokeColor),
    textColor: trimString(payload.textColor),
    dropRateText: groupDropRateText,
    dropRateSourceKind: trimString(payload.dropRateSourceKind),
    dropRateTooltip: groupDropRateTooltip,
    rawDropRateText: trimString(payload.rawDropRateText) || groupDropRateText,
    rawDropRateTooltip: trimString(payload.rawDropRateTooltip) || groupDropRateTooltip,
    normalizedDropRateText: trimString(payload.normalizedDropRateText) || groupDropRateText,
    normalizedDropRateTooltip: trimString(payload.normalizedDropRateTooltip) || groupDropRateTooltip,
    catchMethods: normalizeCatchMethods(payload.catchMethods),
    conditionOptions,
  };
}

function buildHotspotSection(layerSamples, conditionSelection = {}, normalizeRates = true) {
  const sample = layerSamples.find(isHotspotSample);
  const section = hotspotDetailSection(sample);
  if (!sample || !section) {
    return null;
  }
  const facts = detailSectionFacts(section);
  const fishName = firstFactValue(facts, "primary_fish") || trimString(sample?.targets?.[0]?.label);
  const fishItemId = integerValue(firstFactValue(facts, "primary_fish_item_id"));
  const hotspotId = firstFactValue(facts, "hotspot_id");
  const metadataSource = firstFactValue(facts, "metadata_source");
  const sourceMetadataStats = firstFactValue(facts, "source_metadata_stats");
  const target = Array.isArray(sample?.targets) ? sample.targets[0] : null;
  const worldX = Number(target?.worldX);
  const worldZ = Number(target?.worldZ);
  const focusAction = Number.isFinite(worldX) && Number.isFinite(worldZ)
    ? {
        kind: "focus-world-point",
        focusWorldPoint: {
          worldX,
          worldZ,
          elementKind: "hotspot",
          pointKind: "waypoint",
          pointLabel: trimString(sample?.targets?.[0]?.label) || fishName,
        },
      }
    : null;
  const metricRows = [
    hotspotMetric("Min. Catches", firstFactValue(facts, "min_fish_count"), ""),
    hotspotMetric("Max. Catches", firstFactValue(facts, "max_fish_count"), ""),
    hotspotMetric(
      "Catchable at",
      formatHotspotLifeLevel(firstFactValue(facts, "available_fishing_level")),
      "fish-fill",
    ),
    hotspotMetric(
      "Visible at",
      formatHotspotLifeLevel(firstFactValue(facts, "observe_fishing_level")),
      "eye",
    ),
  ];
  const minWaitTimeMs = firstFactValue(facts, "min_wait_time_ms");
  const maxWaitTimeMs = firstFactValue(facts, "max_wait_time_ms");
  const biteTime = {
    minimum: formatHotspotSeconds(minWaitTimeMs) || HOTSPOT_UNSET_STAT_VALUE,
    average: formatHotspotAverageSeconds(minWaitTimeMs, maxWaitTimeMs) || HOTSPOT_UNSET_STAT_VALUE,
    maximum: formatHotspotSeconds(maxWaitTimeMs) || HOTSPOT_UNSET_STAT_VALUE,
  };
  const lifetime = formatHotspotDuration(firstFactValue(facts, "point_remain_time_ms")) || HOTSPOT_UNSET_STAT_VALUE;
  const lootGroups = facts
    .filter((fact) => fact.key === "loot_group")
    .map((fact, index) => parseHotspotLootGroupFact(fact, index))
    .filter(Boolean);
  const lootRows = facts.map(parseHotspotLootFact).filter(Boolean);
  const fallbackLootRows = !lootRows.length && fishItemId && fishName
    ? [{
        key: `hotspot-loot:${fishItemId}`,
        itemId: fishItemId,
        label: fishName,
        iconUrl: fishItemIconUrl(fishItemId),
        iconGradeTone: "unknown",
        dropRateText: "",
      }]
    : [];
  const groups = lootGroups.length
    ? buildZoneLootGroups(
        {
          groups: lootGroups,
          speciesRows: [],
        },
        conditionSelection,
        normalizeRates,
      )
    : (lootRows.length ? lootRows : fallbackLootRows).length
      ? [{
          label: "Group 1",
          rows: lootRows.length ? lootRows : fallbackLootRows,
        }]
      : [];
  const profiles = buildZoneLootMethodProfiles(groups);
  return {
    id: "hotspot-details",
    kind: "hotspot",
    title: "Hotspot Details",
    fishName,
    fishItemId,
    hotspotId,
    iconUrl: fishItemId ? fishItemIconUrl(fishItemId) : "",
    focusAction,
    metadataSource,
    sourceMetadataStats,
    metrics: metricRows,
    biteTime,
    lifetime,
    groups,
    profiles,
  };
}

function normalizeDetailFacts(sample) {
  if (!Array.isArray(sample?.detailSections)) {
    return [];
  }
  const rows = [];
  for (const section of sample.detailSections) {
    if (!isPlainObject(section) || trimString(section.kind) !== "facts") {
      continue;
    }
    const sectionTitle = trimString(section.title);
    for (const fact of Array.isArray(section.facts) ? section.facts : []) {
      if (!isPlainObject(fact)) {
        continue;
      }
      const key = trimString(fact.key);
      const label = trimString(fact.label) || sectionTitle || key.replace(/_/g, " ");
      const value = trimString(fact.value);
      if (!key || !label || !value) {
        continue;
      }
      rows.push({
        key,
        label,
        value,
        icon: trimString(fact.icon),
        statusIcon: trimString(fact.statusIcon),
        statusIconTone: trimString(fact.statusIconTone),
      });
    }
  }
  return rows;
}

function isLandmarkSample(sample) {
  const layerId = trimString(sample?.layerId);
  const kind = trimString(sample?.kind);
  if (kind === "waypoint" || LANDMARK_LAYER_IDS.includes(layerId)) {
    return true;
  }
  return false;
}

function landmarkFactIcon(layerId, fact) {
  if (trimString(fact?.icon)) {
    return trimString(fact.icon);
  }
  if (trimString(layerId) === "trade_npcs") {
    return "trade-origin";
  }
  return "map-pin";
}

function buildLandmarkPaneFacts(layerSamples) {
  const facts = [];
  const seen = new Set();
  for (const sample of layerSamples.filter(isLandmarkSample)) {
    const layerId = trimString(sample?.layerId);
    for (const fact of normalizeDetailFacts(sample)) {
      const key = `${layerId}:${fact.key}:${fact.value}`;
      if (seen.has(key)) {
        continue;
      }
      seen.add(key);
      facts.push({
        key: `landmark:${layerId}:${fact.key}`,
        label: fact.label,
        value: fact.value,
        icon: landmarkFactIcon(layerId, fact),
        ...(fact.statusIcon ? { statusIcon: fact.statusIcon } : {}),
        ...(fact.statusIconTone ? { statusIconTone: fact.statusIconTone } : {}),
      });
    }
  }
  return facts;
}

function paneDescriptor(id, label, icon, summary, sections) {
  return {
    id,
    label,
    icon,
    summary: trimString(summary),
    sections: sections.filter(Boolean),
  };
}

export function patchTouchesInfoSignals(patch) {
  if (!isPlainObject(patch)) {
    return false;
  }
  return Boolean(
    patch._map_runtime?.selection != null ||
      patch._map_runtime?.catalog?.layers != null ||
      patch._map_runtime?.catalog?.fish != null ||
      patch._map_ui?.windowUi?.zoneInfo != null ||
      patch._map_ui?.windowUi?.settings?.normalizeRates != null,
  );
}

export function buildInfoViewModel(
  signals,
  {
    zoneCatalog = [],
    zoneLootSummary = null,
    zoneLootStatus = "idle",
    zoneLootConditionSelection = {},
    normalizeRates = null,
    tradeNpcMapCatalog = null,
    tradeNpcMapStatus = "idle",
  } = {},
) {
  const selection = isPlainObject(signals?._map_runtime?.selection)
    ? cloneJson(signals._map_runtime.selection)
    : {};
  const layerSamples = normalizeSelectionLayerSamples(selection);
  const runtimeLayers = Array.isArray(signals?._map_runtime?.catalog?.layers)
    ? cloneJson(signals._map_runtime.catalog.layers)
    : [];
  const runtimeFish = Array.isArray(signals?._map_runtime?.catalog?.fish)
    ? cloneJson(signals._map_runtime.catalog.fish)
    : [];
  const pointSampleStateBundle = {
    state: {
      catalog: {
        fish: runtimeFish,
      },
    },
  };
  const zoneFacts = buildZonePaneFacts(layerSamples, {
    zoneCatalog,
    runtimeLayers,
  });
  const landmarkFacts = buildLandmarkPaneFacts(layerSamples);
  const territoryFacts = buildTerritoryPaneFacts(layerSamples, { runtimeLayers });
  const tradeFacts = buildTradePaneFacts(layerSamples, {
    runtimeLayers,
    tradeNpcMapCatalog,
    tradeNpcMapStatus,
  });
  const zoneLootSection = buildZoneLootSection(
    zoneLootSummary,
    zoneLootStatus,
    zoneLootConditionSelection,
    typeof normalizeRates === "boolean" ? normalizeRates : normalizeRatesFromSignals(signals),
  );
  const pointSampleSection = buildPointSampleSection(
    selection,
    pointSampleStateBundle,
    zoneCatalog,
  );
  const hotspotSection = buildHotspotSection(
    layerSamples,
    zoneLootConditionSelection,
    typeof normalizeRates === "boolean" ? normalizeRates : normalizeRatesFromSignals(signals),
  );
  const panes = [
    pointSampleSection
      ? paneDescriptor(
          "samples",
          "Samples",
          "date-confirmed",
          pointSampleSection.rows[0]?.fishName || "",
          [pointSampleSection],
        )
      : null,
    hotspotSection
      ? paneDescriptor(
          "hotspot",
          "Hotspot",
          HOTSPOT_INFO_ICON,
          hotspotSection.fishName || "",
          [hotspotSection],
        )
      : null,
    paneDescriptor(
      "landmark",
      mapText("info.pane.landmark"),
      landmarkFacts[0]?.icon || "map-pin",
      landmarkFacts[0]?.value || "",
      landmarkFacts.length
        ? [
            {
              id: "landmark-facts",
              kind: "facts",
              title: mapText("info.section.landmark"),
              facts: landmarkFacts,
            },
          ]
        : [],
    ),
    paneDescriptor(
      "zone",
      mapText("info.pane.zone"),
      "hover-zone",
      zoneFacts.find((fact) => fact.key === "zone")?.value || "",
      [
        zoneFacts.length
          ? {
              id: "zone-facts",
              kind: "facts",
              title: mapText("info.section.zone"),
              facts: zoneFacts,
            }
          : null,
        zoneLootSection,
      ],
    ),
    paneDescriptor(
      "territory",
      mapText("info.pane.territory"),
      "hover-resources",
      territoryFacts[0]?.value || "",
      territoryFacts.length
        ? [
            {
              id: "territory-facts",
              kind: "facts",
              title: mapText("info.section.territory"),
              facts: territoryFacts,
            },
          ]
        : [],
    ),
    paneDescriptor(
      "trade",
      mapText("info.pane.trade"),
      "trade-origin",
      tradeFacts[0]?.value || "",
      tradeFacts.length
        ? [
            {
              id: "trade-facts",
              kind: "facts",
              title: mapText("info.section.trade"),
              facts: tradeFacts,
            },
          ]
        : [],
    ),
  ].filter((pane) => pane && pane.sections.length > 0);

  const requestedPaneId = trimString(signals?._map_ui?.windowUi?.zoneInfo?.tab);
  const preferredPaneId = preferredPaneIdFromSelection(selection);
  const fallbackPaneId = paneIdExists(panes, preferredPaneId)
    ? preferredPaneId
    : panes[0]?.id || "";
  const activePaneId =
    paneIdExists(panes, requestedPaneId)
      ? requestedPaneId
      : fallbackPaneId;
  const pointKind = normalizePointKind(selection?.pointKind);
  const statusDescriptor = selectionStatusDescriptor(selection, layerSamples);
  const overviewRows = buildOverviewRowsForLayerSamples(layerSamples, {
    zoneCatalog,
    runtimeLayers,
  });
  return {
    descriptor: {
      title: titleFromSelection(selection, layerSamples, zoneCatalog, runtimeLayers),
      titleIcon: INFO_WINDOW_TITLE_ICON,
      statusIcon: statusDescriptor.statusIcon,
      statusText: statusDescriptor.statusText,
      pointKind,
      overviewRows,
    },
    panes,
    activePaneId,
    activePane: panes.find((pane) => pane.id === activePaneId) || null,
    empty: panes.length === 0,
  };
}
