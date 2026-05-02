import { resolveApiBaseUrl } from "./map-host.js";
import { mapText } from "./map-i18n.js";

const DEFAULT_TRADE_NPCS_MAP_PATH = "/api/v1/trade_npcs/map";
const TRADE_DISTANCE_BONUS_SCALE = 68 / 1_000_000;
const ORIGIN_TARGET_KEY = "origin_node";

let cachedTradeNpcMapCatalog = null;
let cachedTradeNpcMapPromise = null;

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function trimString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function finiteNumber(value) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function integerValue(value) {
  const parsed = Number.parseInt(value, 10);
  return Number.isInteger(parsed) ? parsed : null;
}

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function normalizeLookupKey(value) {
  return trimString(value).toLowerCase().replace(/\s+/g, " ");
}

function slugifyLookupKey(value) {
  const normalized = normalizeLookupKey(value);
  const ascii =
    typeof normalized.normalize === "function"
      ? normalized.normalize("NFKD").replace(/[\u0300-\u036f]/g, "")
      : normalized;
  return ascii
    .replace(/['"]/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function normalizeSelectorList(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  const next = [];
  const seen = new Set();
  for (const value of values) {
    const normalized = trimString(value);
    const lookupKey = normalizeLookupKey(normalized);
    if (!lookupKey || seen.has(lookupKey)) {
      continue;
    }
    seen.add(lookupKey);
    next.push(normalized);
  }
  return next;
}

function regionIdFromLabel(value) {
  const label = trimString(value);
  if (!label) {
    return null;
  }
  const match = label.match(/\bR(\d+)\b/i) || label.match(/^\((\d+)\|/);
  if (!match) {
    return null;
  }
  const regionId = Number.parseInt(match[1], 10);
  return Number.isInteger(regionId) && regionId >= 0 ? regionId : null;
}

function normalizeCoordinatePair(source) {
  if (!isPlainObject(source)) {
    return { worldX: null, worldZ: null };
  }
  return {
    worldX: finiteNumber(source.worldX ?? source.world_x),
    worldZ: finiteNumber(source.worldZ ?? source.world_z),
  };
}

function normalizeRegionRef(source) {
  if (!isPlainObject(source)) {
    return {
      regionId: null,
      regionName: "",
      waypointId: null,
      waypointName: "",
      worldX: null,
      worldZ: null,
    };
  }
  const coords = normalizeCoordinatePair(source);
  return {
    regionId: integerValue(source.regionId ?? source.region_id),
    regionName: trimString(source.regionName ?? source.region_name),
    waypointId: integerValue(source.waypointId ?? source.waypoint_id),
    waypointName: trimString(source.waypointName ?? source.waypoint_name),
    ...coords,
  };
}

function formatRegionRef(region) {
  if (!region) {
    return "";
  }
  if (region.regionName && region.regionId != null) {
    return `${region.regionName} (R${region.regionId})`;
  }
  if (region.regionName) {
    return region.regionName;
  }
  if (region.regionId != null) {
    return `R${region.regionId}`;
  }
  return "";
}

function normalizeDetailFacts(sample) {
  if (!Array.isArray(sample?.detailSections)) {
    return [];
  }
  const facts = [];
  for (const section of sample.detailSections) {
    if (!isPlainObject(section) || trimString(section.kind) !== "facts") {
      continue;
    }
    for (const fact of Array.isArray(section.facts) ? section.facts : []) {
      if (!isPlainObject(fact)) {
        continue;
      }
      facts.push({
        key: trimString(fact.key),
        label: trimString(fact.label),
        value: trimString(fact.value),
        icon: trimString(fact.icon),
      });
    }
  }
  return facts;
}

function selectedOriginTarget(sample) {
  const targets = Array.isArray(sample?.targets) ? sample.targets : [];
  return (
    targets.find((target) => trimString(target?.key) === ORIGIN_TARGET_KEY) ||
    targets.find((target) => trimString(target?.label).toLowerCase().startsWith("origin:")) ||
    null
  );
}

export function selectedTradeOriginFromLayerSamples(layerSamples) {
  const regionSample = (Array.isArray(layerSamples) ? layerSamples : []).find(
    (sample) => trimString(sample?.layerId) === "regions",
  );
  if (!regionSample) {
    return null;
  }
  const facts = normalizeDetailFacts(regionSample);
  const originFact =
    facts.find((fact) => fact.key === "origin_region") ||
    facts.find((fact) => fact.key === "origin") ||
    null;
  const target = selectedOriginTarget(regionSample);
  const coords = normalizeCoordinatePair(target);
  const targetLabel = trimString(target?.label).replace(/^Origin:\s*/i, "");
  const label = originFact?.value || targetLabel;
  const regionId =
    regionIdFromLabel(originFact?.value) ??
    regionIdFromLabel(targetLabel) ??
    integerValue(regionSample.fieldId ?? regionSample.field_id);
  if (!label && regionId == null && (coords.worldX == null || coords.worldZ == null)) {
    return null;
  }
  return {
    regionId,
    label,
    worldX: coords.worldX,
    worldZ: coords.worldZ,
  };
}

function normalizeTradeNpcFeature(feature) {
  if (!isPlainObject(feature)) {
    return null;
  }
  const properties = isPlainObject(feature.properties) ? feature.properties : feature;
  const geometryCoordinates = Array.isArray(feature.geometry?.coordinates)
    ? feature.geometry.coordinates
    : [];
  const sellOrigin = normalizeRegionRef(
    properties.sellOrigin ??
      properties.sellDestinationTradeOrigin ??
      properties.sell_destination_trade_origin,
  );
  const assignedRegion = normalizeRegionRef(properties.assignedRegion ?? properties.assigned_region);
  const spawn = normalizeRegionRef(properties.spawn ?? properties.npcSpawn ?? properties.npc_spawn);
  const fallbackPoint = {
    worldX: finiteNumber(geometryCoordinates[0]),
    worldZ: finiteNumber(geometryCoordinates[1]),
  };
  const npcName = trimString(
    properties.npcName ?? properties.npc_name ?? properties.name ?? properties.label,
  );
  if (!npcName) {
    return null;
  }
  return {
    id: trimString(properties.id) || trimString(feature.id) || npcName,
    npcKey: integerValue(properties.npcKey ?? properties.npc_key),
    npcName,
    roleSource: trimString(properties.roleSource ?? properties.role_source),
    sourceTags: Array.isArray(properties.sourceTags ?? properties.source_tags)
      ? (properties.sourceTags ?? properties.source_tags).map(trimString).filter(Boolean)
      : [],
    sellOrigin,
    assignedRegion,
    spawn: {
      ...spawn,
      worldX: spawn.worldX ?? fallbackPoint.worldX,
      worldZ: spawn.worldZ ?? fallbackPoint.worldZ,
    },
    sellOriginLabel: trimString(properties.sellOriginLabel) || formatRegionRef(sellOrigin),
    assignedRegionLabel: trimString(properties.assignedRegionLabel) || formatRegionRef(assignedRegion),
  };
}

export function normalizeTradeNpcMapCatalog(raw) {
  const source = isPlainObject(raw) ? raw : {};
  return {
    metadata: isPlainObject(source.metadata) ? cloneJson(source.metadata) : {},
    features: Array.isArray(source.features)
      ? source.features.map(normalizeTradeNpcFeature).filter(Boolean)
      : [],
  };
}

function distanceBonusPercent(origin, destination) {
  if (
    !Number.isFinite(origin?.worldX) ||
    !Number.isFinite(origin?.worldZ) ||
    !Number.isFinite(destination?.sellOrigin?.worldX) ||
    !Number.isFinite(destination?.sellOrigin?.worldZ)
  ) {
    return null;
  }
  const dx = origin.worldX - destination.sellOrigin.worldX;
  const dz = origin.worldZ - destination.sellOrigin.worldZ;
  return Math.hypot(dx, dz) * TRADE_DISTANCE_BONUS_SCALE;
}

function focusWorldPointForNpc(destination) {
  if (!Number.isFinite(destination?.spawn?.worldX) || !Number.isFinite(destination?.spawn?.worldZ)) {
    return null;
  }
  return {
    elementKind: "npc",
    worldX: destination.spawn.worldX,
    worldZ: destination.spawn.worldZ,
    pointKind: "waypoint",
    pointLabel: destination.npcName,
  };
}

function tradeNpcMatchesSelector(destination, selector) {
  const normalizedSelector = normalizeLookupKey(selector);
  const slugSelector = slugifyLookupKey(selector);
  const numericSelector = Number.parseInt(selector, 10);
  if (Number.isInteger(numericSelector) && destination.npcKey === numericSelector) {
    return "exact";
  }
  const normalizedName = normalizeLookupKey(destination.npcName);
  const slugName = slugifyLookupKey(destination.npcName);
  if (normalizedName === normalizedSelector || slugName === slugSelector) {
    return "exact";
  }
  if (
    normalizedSelector &&
    (normalizedName.includes(normalizedSelector) ||
      (slugSelector && slugName.includes(slugSelector)))
  ) {
    return "partial";
  }
  return "";
}

export function tradeNpcFocusTargetForSelectors(selectors, rawCatalog) {
  const catalog = normalizeTradeNpcMapCatalog(rawCatalog);
  const normalizedSelectors = normalizeSelectorList(selectors);
  for (const selector of normalizedSelectors) {
    const exact = catalog.features.find((destination) => (
      tradeNpcMatchesSelector(destination, selector) === "exact" &&
      focusWorldPointForNpc(destination)
    ));
    if (exact) {
      return focusWorldPointForNpc(exact);
    }
    const partialMatches = catalog.features.filter((destination) => (
      tradeNpcMatchesSelector(destination, selector) === "partial" &&
      focusWorldPointForNpc(destination)
    ));
    if (partialMatches.length === 1) {
      return focusWorldPointForNpc(partialMatches[0]);
    }
  }
  return null;
}

export function formatTradeDistanceBonus(value) {
  const normalized = finiteNumber(value);
  return normalized == null ? "" : `${Math.max(0, normalized).toFixed(1)}%`;
}

export function tradeManagerRowsForOrigin(layerSamples, rawCatalog) {
  const origin = selectedTradeOriginFromLayerSamples(layerSamples);
  const catalog = normalizeTradeNpcMapCatalog(rawCatalog);
  if (!origin || !Number.isFinite(origin.worldX) || !Number.isFinite(origin.worldZ)) {
    return [];
  }
  return catalog.features
    .map((destination) => {
      const distanceBonus = distanceBonusPercent(origin, destination);
      if (distanceBonus == null) {
        return null;
      }
      return {
        ...destination,
        distanceBonus,
        distanceBonusText: formatTradeDistanceBonus(distanceBonus),
      };
    })
    .filter(Boolean)
    .sort((left, right) => {
      const distance = right.distanceBonus - left.distanceBonus;
      if (distance !== 0) {
        return distance;
      }
      const name = left.npcName.localeCompare(right.npcName);
      if (name !== 0) {
        return name;
      }
      return left.sellOriginLabel.localeCompare(right.sellOriginLabel);
    });
}

export function tradeManagerFactsForOrigin(layerSamples, rawCatalog, { status = "idle" } = {}) {
  const origin = selectedTradeOriginFromLayerSamples(layerSamples);
  if (!origin) {
    return [];
  }
  if (status === "loading") {
    return [
      {
        key: "trade_managers_status",
        icon: "information-circle",
        label: mapText("info.trade.managers"),
        value: mapText("info.trade.loading"),
      },
    ];
  }
  if (status === "error") {
    return [
      {
        key: "trade_managers_status",
        icon: "information-circle",
        label: mapText("info.trade.managers"),
        value: mapText("info.trade.unavailable"),
      },
    ];
  }
  const rows = tradeManagerRowsForOrigin(layerSamples, rawCatalog);
  if (!rows.length) {
    return [];
  }
  return [
    {
      key: "trade_manager_count",
      icon: "trade-origin",
      label: mapText("info.trade.managers"),
      value: mapText("info.trade.manager_count", { count: rows.length }),
    },
    ...rows.map((row) => ({
      key: `trade_manager:${row.id}`,
      variant: "trade-manager",
      icon: "trade-origin",
      label: row.npcName,
      value: [row.distanceBonusText, row.sellOriginLabel || row.assignedRegionLabel]
        .filter(Boolean)
        .join(" · "),
      distanceBonusText: row.distanceBonusText,
      tradeDistanceRegionLabel: row.sellOriginLabel || row.assignedRegionLabel,
      action: focusWorldPointForNpc(row)
        ? {
            kind: "focus-world-point",
            focusWorldPoint: focusWorldPointForNpc(row),
          }
        : null,
    })),
  ];
}

export async function loadTradeNpcMapCatalog({
  fetchImpl = globalThis.fetch,
  locationLike = globalThis.window?.location,
  force = false,
} = {}) {
  if (!force && cachedTradeNpcMapCatalog) {
    return cachedTradeNpcMapCatalog;
  }
  if (!force && cachedTradeNpcMapPromise) {
    return cachedTradeNpcMapPromise;
  }
  const baseUrl = resolveApiBaseUrl(locationLike);
  cachedTradeNpcMapPromise = Promise.resolve()
    .then(() => fetchImpl(`${baseUrl}${DEFAULT_TRADE_NPCS_MAP_PATH}`))
    .then(async (response) => {
      if (!response?.ok) {
        throw new Error(mapText("info.trade.request_failed", { status: response?.status || 0 }));
      }
      const catalog = normalizeTradeNpcMapCatalog(await response.json());
      cachedTradeNpcMapCatalog = catalog;
      return catalog;
    })
    .finally(() => {
      cachedTradeNpcMapPromise = null;
    });
  return cachedTradeNpcMapPromise;
}
