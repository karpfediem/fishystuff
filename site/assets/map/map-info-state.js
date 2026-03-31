import {
  buildOverviewRowsForLayerSamples,
  buildTerritoryPaneFacts,
  buildTradePaneFacts,
  buildZonePaneFacts,
  preferredOverviewRow,
} from "./map-overview-facts.js";

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

function pointKindStatusText(pointKind, pointLabel) {
  const normalizedLabel = trimString(pointLabel);
  switch (normalizePointKind(pointKind)) {
    case "bookmark":
      return normalizedLabel || "Bookmark";
    case "waypoint":
      return normalizedLabel || "Waypoint";
    case "clicked":
      return "Clicked point";
    default:
      return "no selection";
  }
}

function pointKindIcon(pointKind) {
  switch (normalizePointKind(pointKind)) {
    case "bookmark":
      return "bookmark";
    case "waypoint":
      return "map-pin";
    case "clicked":
      return "hover-zone";
    default:
      return "information-circle";
  }
}

function formatTimestampUtc(tsUtc) {
  const tsMs = Number(tsUtc) * 1000;
  if (!Number.isFinite(tsMs) || tsMs <= 0) {
    return "";
  }
  const date = new Date(tsMs);
  const year = date.getUTCFullYear();
  const month = String(date.getUTCMonth() + 1).padStart(2, "0");
  const day = String(date.getUTCDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function formatDecimal(value, digits = 2) {
  const number = Number(value);
  return Number.isFinite(number) ? number.toFixed(digits) : "n/a";
}

function formatPercent(value, digits = 1) {
  const number = Number(value);
  return Number.isFinite(number) ? `${(number * 100).toFixed(digits)}%` : "n/a";
}

function formatZoneStatus(status) {
  const raw = trimString(status);
  if (!raw) {
    return "Unknown";
  }
  return raw
    .toLowerCase()
    .split(/[_\s-]+/g)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function buildFishLookup(runtimeFish) {
  return new Map(
    (Array.isArray(runtimeFish) ? runtimeFish : [])
      .filter((fish) => isPlainObject(fish))
      .map((fish) => [Number.parseInt(fish.fishId, 10), fish]),
  );
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
  const worldX = Number(selection?.worldX);
  const worldZ = Number(selection?.worldZ);
  if (Number.isFinite(worldX) && Number.isFinite(worldZ)) {
    return `${Math.round(worldX).toLocaleString("en-US")}, ${Math.round(worldZ).toLocaleString("en-US")}`;
  }
  return "Info";
}

function buildZoneEvidenceSummary(zoneStats) {
  if (!zoneStats) {
    return "No fish presence evidence loaded.";
  }
  const confidence = isPlainObject(zoneStats.confidence) ? zoneStats.confidence : {};
  const parts = [];
  const status = formatZoneStatus(confidence.status);
  if (status) {
    parts.push(status);
  }
  if (Number.isFinite(confidence.ess)) {
    parts.push(`ESS ${formatDecimal(confidence.ess, 1)}`);
  }
  if (Number.isFinite(confidence.totalWeight)) {
    parts.push(`weight ${formatDecimal(confidence.totalWeight, 2)}`);
  }
  const lastSeen = formatTimestampUtc(confidence.lastSeenTsUtc);
  if (lastSeen) {
    parts.push(`last seen ${lastSeen}`);
  } else if (Number.isFinite(confidence.ageDaysLast)) {
    parts.push(`last seen ${formatDecimal(confidence.ageDaysLast, 1)}d ago`);
  }
  if (Array.isArray(confidence.notes) && confidence.notes.length) {
    parts.push(confidence.notes.join(" · "));
  }
  return parts.join(" · ") || "No confidence data.";
}

function buildFishPresenceSection(selection, signals) {
  const zoneStats = isPlainObject(selection?.zoneStats) ? selection.zoneStats : null;
  const zoneRgb = zoneStats?.zoneRgb ?? buildZonePaneFacts(selection?.layerSamples, {}).find((row) => row.key === "zone");
  if (!zoneStats && !selection?.layerSamples?.length) {
    return null;
  }
  const fishLookup = buildFishLookup(signals?._map_runtime?.catalog?.fish);
  return {
    id: "fish-presence",
    kind: "evidence",
    title: "Fish Presence",
    statusText: trimString(signals?._map_runtime?.statuses?.zoneStatsStatus) || "zone evidence: idle",
    summary: buildZoneEvidenceSummary(zoneStats),
    note: "Evidence shares summarize observed fish presence here. They are not direct catch rates.",
    entries: (Array.isArray(zoneStats?.distribution) ? zoneStats.distribution : []).map((entry) => {
      const fishId = Number.parseInt(entry?.fishId, 10);
      const catalogFish = fishLookup.get(fishId) || null;
      return {
        fishId,
        itemId: catalogFish?.itemId ?? entry?.itemId ?? null,
        encyclopediaId: catalogFish?.encyclopediaId ?? entry?.encyclopediaId ?? null,
        name:
          trimString(catalogFish?.name) ||
          trimString(entry?.fishName) ||
          (Number.isFinite(fishId) ? `Fish ${fishId}` : "Unknown fish"),
        shareText: formatPercent(entry?.pMean, 1),
        detailText: `weight ${formatDecimal(entry?.evidenceWeight, 2)} · CI ${formatPercent(entry?.ciLow, 1)}-${formatPercent(entry?.ciHigh, 1)}`,
      };
    }),
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
      patch._map_runtime?.catalog?.fish != null ||
      patch._map_runtime?.statuses?.zoneStatsStatus != null ||
      patch._map_ui?.windowUi?.zoneInfo != null,
  );
}

export function buildInfoViewModel(signals, { zoneCatalog = [] } = {}) {
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
  const fishPresenceSection = buildFishPresenceSection(selection, signals);
  const panes = [
    paneDescriptor(
      "zone",
      "Zone",
      "hover-zone",
      zoneFacts.find((fact) => fact.key === "zone")?.value || "",
      [
        zoneFacts.length
          ? {
              id: "zone-facts",
              kind: "facts",
              title: "Zone",
              facts: zoneFacts,
            }
          : null,
        fishPresenceSection,
      ],
    ),
    paneDescriptor(
      "territory",
      "Territory",
      "hover-resources",
      territoryFacts[0]?.value || "",
      territoryFacts.length
        ? [
            {
              id: "territory-facts",
              kind: "facts",
              title: "Territory",
              facts: territoryFacts,
            },
          ]
        : [],
    ),
    paneDescriptor(
      "trade",
      "Trade",
      "trade-origin",
      tradeFacts[0]?.value || "",
      tradeFacts.length
        ? [
            {
              id: "trade-facts",
              kind: "facts",
              title: "Trade",
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
      titleIcon: pointKindIcon(pointKind),
      statusIcon: pointKindIcon(pointKind),
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
