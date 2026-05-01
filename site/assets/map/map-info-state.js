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
const LANDMARK_LAYER_IDS = Object.freeze(["trade_npcs", "region_nodes", "bookmarks"]);

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

function titleFromSelection(selection, layerSamples, zoneCatalog, runtimeLayers) {
  const pointLabel = trimString(selection?.pointLabel);
  if (pointLabel) {
    return pointLabel;
  }
  const preferred = preferredOverviewRow(layerSamples, {
    zoneCatalog,
    runtimeLayers,
  });
  if (preferred?.value) {
    return preferred.value;
  }
  return mapText("info.window_title");
}

function zoneLootConditionKey(group, index) {
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
  const activePaneId =
    requestedPaneId && panes.some((pane) => pane.id === requestedPaneId)
      ? requestedPaneId
      : panes[0]?.id || "";
  const pointKind = normalizePointKind(selection?.pointKind);
  const overviewRows = buildOverviewRowsForLayerSamples(layerSamples, {
    zoneCatalog,
    runtimeLayers,
  });
  return {
    descriptor: {
      title: titleFromSelection(selection, layerSamples, zoneCatalog, runtimeLayers),
      titleIcon: INFO_WINDOW_TITLE_ICON,
      statusIcon: INFO_WINDOW_STATUS_ICON,
      statusText: pointKindStatusText(pointKind, selection?.pointLabel),
      pointKind,
      overviewRows,
    },
    panes,
    activePaneId,
    activePane: panes.find((pane) => pane.id === activePaneId) || null,
    empty: panes.length === 0,
  };
}
