import {
  buildOverviewRowsForLayerSamples,
  buildTerritoryPaneFacts,
  buildTradePaneFacts,
  buildZonePaneFacts,
  preferredOverviewRow,
} from "./map-overview-facts.js";
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

function buildZoneLootGroups(summary) {
  const groups = Array.isArray(summary?.groups) ? summary.groups : [];
  const speciesRows = Array.isArray(summary?.speciesRows) ? summary.speciesRows : [];
  if (!groups.length && !speciesRows.length) {
    return [];
  }
  return groups
    .map((group, index) => ({
      slotIdx: Number.parseInt(group?.slotIdx, 10) || index + 1,
      label: trimString(group?.label),
      fillColor: trimString(group?.fillColor),
      strokeColor: trimString(group?.strokeColor),
      textColor: trimString(group?.textColor),
      dropRateText: trimString(group?.dropRateText),
      dropRateSourceKind: trimString(group?.dropRateSourceKind),
      dropRateTooltip: trimString(group?.dropRateTooltip),
      rows: speciesRows.filter((row) => {
        const rowGroupLabel = trimString(row?.groupLabel);
        const groupLabel = trimString(group?.label);
        if (rowGroupLabel && groupLabel && rowGroupLabel === groupLabel) {
          return true;
        }
        return (Number.parseInt(row?.slotIdx, 10) || 0) === (Number.parseInt(group?.slotIdx, 10) || index + 1);
      }),
    }))
    .filter((group) => group.label);
}

function buildZoneLootSection(summary, status) {
  const normalizedStatus = trimString(status || "idle");
  if (normalizedStatus === "idle" && !isPlainObject(summary)) {
    return null;
  }
  const groups = buildZoneLootGroups(summary);
  const available = summary?.available === true && groups.length > 0;
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
    note: trimString(summary?.note),
    groups,
    available,
  };
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
      patch._map_ui?.windowUi?.zoneInfo != null,
  );
}

export function buildInfoViewModel(signals, { zoneCatalog = [], zoneLootSummary = null, zoneLootStatus = "idle" } = {}) {
  const selection = isPlainObject(signals?._map_runtime?.selection)
    ? cloneJson(signals._map_runtime.selection)
    : {};
  const layerSamples = normalizeSelectionLayerSamples(selection);
  const runtimeLayers = Array.isArray(signals?._map_runtime?.catalog?.layers)
    ? cloneJson(signals._map_runtime.catalog.layers)
    : [];
  const zoneFacts = buildZonePaneFacts(layerSamples, {
    zoneCatalog,
    runtimeLayers,
  });
  const territoryFacts = buildTerritoryPaneFacts(layerSamples, { runtimeLayers });
  const tradeFacts = buildTradePaneFacts(layerSamples, { runtimeLayers });
  const zoneLootSection = buildZoneLootSection(zoneLootSummary, zoneLootStatus);
  const panes = [
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
  ].filter((pane) => pane.sections.length > 0);

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
