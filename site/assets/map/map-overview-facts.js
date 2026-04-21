import {
  zoneCatalogEntryForRgb,
  zoneDisplayNameFromCatalog,
} from "./map-zone-catalog.js";
import { mapText } from "./map-i18n.js";

export const PRIMARY_OVERVIEW_ROW_KEYS = Object.freeze(["zone", "resources", "origin"]);
const POINT_LABEL_PRIMARY_FACT_KEYS = Object.freeze([
  "zone",
  "resource_group",
  "resource_region",
  "origin_region",
  "origin_node",
  "resource_waypoint",
]);

const LAYER_FALLBACK_ORDER = Object.freeze({
  zone_mask: 10,
  region_groups: 20,
  regions: 30,
});

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function trimString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function normalizeLayerSamples(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  return values.filter((sample) => isPlainObject(sample));
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
    for (const fact of Array.isArray(section.facts) ? section.facts : []) {
      if (!isPlainObject(fact)) {
        continue;
      }
      const key = trimString(fact.key);
      const label = trimString(fact.label);
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

function detailFactByKeys(sample, factKeys) {
  const wanted = new Set((Array.isArray(factKeys) ? factKeys : []).map(trimString).filter(Boolean));
  if (!wanted.size) {
    return null;
  }
  return normalizeDetailFacts(sample).find((fact) => wanted.has(fact.key)) || null;
}

function preferredDetailFact(sample) {
  const facts = normalizeDetailFacts(sample);
  for (const key of POINT_LABEL_PRIMARY_FACT_KEYS) {
    const match = facts.find((fact) => fact.key === key);
    if (match) {
      return match;
    }
  }
  return facts.find((fact) => trimString(fact?.value)) || null;
}

function normalizeRgbTriplet(input) {
  if (Array.isArray(input) && input.length >= 3) {
    const [r, g, b] = input.map((value) => Number.parseInt(value, 10));
    return [r, g, b].every((value) => Number.isInteger(value) && value >= 0 && value <= 255)
      ? [r, g, b]
      : null;
  }
  if (isPlainObject(input)) {
    const r = Number.parseInt(input.r, 10);
    const g = Number.parseInt(input.g, 10);
    const b = Number.parseInt(input.b, 10);
    return [r, g, b].every((value) => Number.isInteger(value) && value >= 0 && value <= 255)
      ? [r, g, b]
      : null;
  }
  return null;
}

function rgbTripletFromZoneRgb(zoneRgbInput) {
  const zoneRgb = Number.parseInt(zoneRgbInput, 10);
  if (!Number.isInteger(zoneRgb) || zoneRgb < 0) {
    return null;
  }
  return [(zoneRgb >> 16) & 0xff, (zoneRgb >> 8) & 0xff, zoneRgb & 0xff];
}

function formatRgbTriplet(rgb) {
  return Array.isArray(rgb) ? rgb.join(",") : "";
}

function formatSwatchRgb(rgb) {
  return Array.isArray(rgb) ? `${rgb[0]} ${rgb[1]} ${rgb[2]}` : "";
}

function formatBiteTimeRange(zone) {
  const min = Number.parseInt(zone?.biteTimeMin, 10);
  const max = Number.parseInt(zone?.biteTimeMax, 10);
  if (!Number.isFinite(min) && !Number.isFinite(max)) {
    return "";
  }
  if (Number.isFinite(min) && Number.isFinite(max)) {
    return `${min}-${max} s`;
  }
  return `${Number.isFinite(min) ? min : max} s`;
}

function runtimeLayerIndexLookup(runtimeLayers) {
  const ordered = (Array.isArray(runtimeLayers) ? runtimeLayers : [])
    .filter((layer) => isPlainObject(layer) && trimString(layer.layerId))
    .slice()
    .sort((left, right) => {
      const leftOrder = Number.isFinite(Number(left.displayOrder)) ? Number(left.displayOrder) : 0;
      const rightOrder = Number.isFinite(Number(right.displayOrder)) ? Number(right.displayOrder) : 0;
      if (leftOrder !== rightOrder) {
        return leftOrder - rightOrder;
      }
      return trimString(left.layerId).localeCompare(trimString(right.layerId));
    });
  return new Map(ordered.map((layer, index) => [trimString(layer.layerId), index]));
}

export function zoneRgbFromSample(sample) {
  const rgbU32 = Number.parseInt(sample?.rgbU32, 10);
  if (Number.isInteger(rgbU32)) {
    return rgbU32;
  }
  const rgb = normalizeRgbTriplet(sample?.rgb);
  return rgb ? ((rgb[0] << 16) | (rgb[1] << 8) | rgb[2]) >>> 0 : null;
}

export function orderedLayerSamplesForOverview(layerSamples, runtimeLayers = []) {
  const orderLookup = runtimeLayerIndexLookup(runtimeLayers);
  return normalizeLayerSamples(layerSamples)
    .map((sample, index) => ({ sample, index }))
    .sort((left, right) => {
      const leftLayerId = trimString(left.sample?.layerId);
      const rightLayerId = trimString(right.sample?.layerId);
      const leftOrder = orderLookup.has(leftLayerId)
        ? orderLookup.get(leftLayerId)
        : LAYER_FALLBACK_ORDER[leftLayerId] ?? 1000 + left.index;
      const rightOrder = orderLookup.has(rightLayerId)
        ? orderLookup.get(rightLayerId)
        : LAYER_FALLBACK_ORDER[rightLayerId] ?? 1000 + right.index;
      if (leftOrder !== rightOrder) {
        return leftOrder - rightOrder;
      }
      return left.index - right.index;
    })
    .map((entry) => entry.sample);
}

export function buildZonePaneFacts(layerSamples, { zoneCatalog = [], runtimeLayers = [] } = {}) {
  const zoneSample = orderedLayerSamplesForOverview(layerSamples, runtimeLayers).find(
    (sample) => trimString(sample?.layerId) === "zone_mask",
  );
  if (!zoneSample) {
    return [];
  }
  const zoneRgb = zoneRgbFromSample(zoneSample);
  const zoneFact = detailFactByKeys(zoneSample, ["zone"]);
  const zoneName =
    zoneFact?.value || (zoneRgb != null ? trimString(zoneDisplayNameFromCatalog(zoneCatalog, zoneRgb)) : "");
  const rgb = normalizeRgbTriplet(zoneSample?.rgb) || rgbTripletFromZoneRgb(zoneRgb);
  const biteTime = formatBiteTimeRange(zoneCatalogEntryForRgb(zoneCatalog, zoneRgb));
  const rows = [];
  if (zoneName) {
    rows.push({
      key: "zone",
      icon: "hover-zone",
      label: mapText("overview.zone"),
      value: zoneName,
      ...(zoneFact?.statusIcon ? { statusIcon: zoneFact.statusIcon } : {}),
      ...(zoneFact?.statusIconTone ? { statusIconTone: zoneFact.statusIconTone } : {}),
    });
  }
  if (rgb) {
    rows.push({
      key: "rgb",
      icon: "theme-palette",
      label: mapText("overview.rgb"),
      value: formatRgbTriplet(rgb),
      swatchRgb: formatSwatchRgb(rgb),
    });
  }
  if (biteTime) {
    rows.push({
      key: "bite_time",
      icon: "stopwatch",
      label: mapText("overview.bite_time"),
      value: biteTime,
    });
  }
  return rows;
}

function buildResourcesFactRow(layerSamples, runtimeLayers = []) {
  const sample = orderedLayerSamplesForOverview(layerSamples, runtimeLayers).find(
    (candidate) => trimString(candidate?.layerId) === "region_groups",
  );
  const fact = detailFactByKeys(sample, ["resource_group", "resources", "resource_region"]);
  if (!fact?.value) {
    return null;
  }
  return {
    key: "resources",
    icon: "hover-resources",
    label: mapText("overview.resources"),
    value: fact.value,
    ...(fact.statusIcon ? { statusIcon: fact.statusIcon } : {}),
    ...(fact.statusIconTone ? { statusIconTone: fact.statusIconTone } : {}),
  };
}

function buildOriginFactRow(layerSamples, runtimeLayers = []) {
  const sample = orderedLayerSamplesForOverview(layerSamples, runtimeLayers).find(
    (candidate) => trimString(candidate?.layerId) === "regions",
  );
  const fact = detailFactByKeys(sample, ["origin_region", "origin"]);
  if (!fact?.value) {
    return null;
  }
  return {
    key: "origin",
    icon: "trade-origin",
    label: mapText("overview.origin"),
    value: fact.value,
    ...(fact.statusIcon ? { statusIcon: fact.statusIcon } : {}),
    ...(fact.statusIconTone ? { statusIconTone: fact.statusIconTone } : {}),
  };
}

export function buildTerritoryPaneFacts(layerSamples, { runtimeLayers = [] } = {}) {
  const row = buildResourcesFactRow(layerSamples, runtimeLayers);
  return row ? [row] : [];
}

export function buildTradePaneFacts(layerSamples, { runtimeLayers = [] } = {}) {
  const row = buildOriginFactRow(layerSamples, runtimeLayers);
  return row ? [row] : [];
}

export function buildOverviewRowsForLayerSamples(
  layerSamples,
  { zoneCatalog = [], runtimeLayers = [] } = {},
) {
  return [
    ...buildZonePaneFacts(layerSamples, { zoneCatalog, runtimeLayers }).filter(
      (row) => row.key === "zone",
    ),
    ...buildTerritoryPaneFacts(layerSamples, { runtimeLayers }),
    ...buildTradePaneFacts(layerSamples, { runtimeLayers }),
  ];
}

export function preferredOverviewRow(layerSamples, options = {}) {
  const rows = buildOverviewRowsForLayerSamples(layerSamples, options);
  for (const key of PRIMARY_OVERVIEW_ROW_KEYS) {
    const match = rows.find((row) => trimString(row?.key) === key);
    if (match) {
      return match;
    }
  }
  return rows[0] || null;
}

function pointLabelForSample(sample, { zoneCatalog = [] } = {}) {
  const layerId = trimString(sample?.layerId);
  if (!layerId) {
    return "";
  }
  if (layerId === "zone_mask") {
    const zoneFact = detailFactByKeys(sample, ["zone"]);
    if (zoneFact?.value) {
      return zoneFact.value;
    }
    const zoneRgb = zoneRgbFromSample(sample);
    const zoneName =
      zoneRgb != null ? trimString(zoneDisplayNameFromCatalog(zoneCatalog, zoneRgb)) : "";
    if (zoneName) {
      return zoneName;
    }
  }
  const fact = preferredDetailFact(sample);
  if (fact?.value) {
    return fact.value;
  }
  if (Array.isArray(sample?.targets)) {
    const targetLabel = sample.targets
      .map((target) => trimString(target?.label))
      .find(Boolean);
    if (targetLabel) {
      return targetLabel;
    }
  }
  return "";
}

export function preferredPointLabelForLayerSamples(
  layerSamples,
  { zoneCatalog = [], runtimeLayers = [] } = {},
) {
  const orderedSamples = orderedLayerSamplesForOverview(layerSamples, runtimeLayers);
  for (const sample of orderedSamples) {
    const label = pointLabelForSample(sample, { zoneCatalog });
    if (label) {
      return label;
    }
  }
  return "";
}
