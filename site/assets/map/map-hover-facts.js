import { resolveLayerEntries } from "./map-layer-state.js";
import {
  zoneDisplayNameFromCatalog,
} from "./map-zone-catalog.js";
import { mapText } from "./map-i18n.js";

const DEFAULT_FACT_ICON = "information-circle";

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

function factLabelFallback(value) {
  return trimString(value).replace(/_/g, " ") || mapText("hover.details");
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

function rgbDisplayValue(rgb) {
  return Array.isArray(rgb) ? rgb.join(",") : "";
}

function rgbSwatchValue(rgb) {
  return Array.isArray(rgb) ? `${rgb[0]} ${rgb[1]} ${rgb[2]}` : "";
}

function rgbTripletFromU32(value) {
  const rgb = Number.parseInt(value, 10);
  if (!Number.isInteger(rgb) || rgb < 0) {
    return null;
  }
  return [(rgb >> 16) & 0xff, (rgb >> 8) & 0xff, rgb & 0xff];
}

function normalizeIntegerList(value) {
  if (!Array.isArray(value)) {
    return [];
  }
  const seen = new Set();
  const values = [];
  for (const entry of value) {
    const parsed = Number.parseInt(entry, 10);
    if (!Number.isInteger(parsed) || parsed < 0 || seen.has(parsed)) {
      continue;
    }
    seen.add(parsed);
    values.push(parsed);
  }
  return values;
}

function formatSampleDate(tsUtc) {
  const seconds = Number.parseInt(tsUtc, 10);
  if (!Number.isInteger(seconds)) {
    return "";
  }
  const date = new Date(seconds * 1000);
  if (!Number.isFinite(date.getTime())) {
    return "";
  }
  return date.toISOString().slice(0, 10);
}

function fishCatalogLookup(stateBundle) {
  const fish = Array.isArray(stateBundle?.state?.catalog?.fish)
    ? stateBundle.state.catalog.fish
    : [];
  return new Map(
    fish
      .map((entry) => [Number.parseInt(entry?.fishId, 10), entry])
      .filter(([fishId]) => Number.isInteger(fishId)),
  );
}

function fishItemIconUrl(itemId) {
  const parsed = Number.parseInt(itemId, 10);
  if (!Number.isInteger(parsed) || parsed <= 0) {
    return "";
  }
  const resolver = globalThis.window?.__fishystuffResolveFishItemIconUrl;
  if (typeof resolver === "function") {
    return trimString(resolver(parsed));
  }
  return `/images/items/${String(parsed).padStart(8, "0")}.webp`;
}

function zoneSummary(zoneRgb, zoneCatalog) {
  const rgb = rgbTripletFromU32(zoneRgb);
  const name =
    trimString(zoneDisplayNameFromCatalog(zoneCatalog, zoneRgb)) ||
    mapText("search.zone.fallback", { zone: `#${Number(zoneRgb).toString(16).padStart(6, "0")}` });
  return {
    zoneRgb,
    name,
    ...(rgb ? { swatchRgb: rgbSwatchValue(rgb) } : {}),
  };
}

function normalizePointSamples(samples) {
  return (Array.isArray(samples) ? samples : [])
    .flatMap((sample) => {
      if (!isPlainObject(sample)) {
        return [];
      }
      const fishId = Number.parseInt(sample.fishId, 10);
      const sampleCount = Math.max(1, Number.parseInt(sample.sampleCount, 10) || 1);
      const lastTsUtc = Number.parseInt(sample.lastTsUtc, 10);
      if (!Number.isInteger(fishId) || !Number.isInteger(lastTsUtc)) {
        return [];
      }
      return [
        {
          fishId,
          sampleCount,
          lastTsUtc,
          zoneRgbs: normalizeIntegerList(sample.zoneRgbs),
          fullZoneRgbs: normalizeIntegerList(sample.fullZoneRgbs),
        },
      ];
    })
    .sort((left, right) =>
      right.sampleCount - left.sampleCount ||
      right.lastTsUtc - left.lastTsUtc ||
      left.fishId - right.fishId,
    );
}

function currentZoneRgbFromSource(source) {
  const zoneSample = (Array.isArray(source?.layerSamples) ? source.layerSamples : []).find(
    (sample) => trimString(sample?.layerId) === "zone_mask",
  );
  const zoneRgb = Number.parseInt(zoneSample?.rgbU32, 10);
  return Number.isInteger(zoneRgb) && zoneRgb >= 0 ? zoneRgb : null;
}

function visibleSampleZones(zones, currentZoneRgb) {
  if (!Number.isInteger(currentZoneRgb)) {
    return zones;
  }
  if (zones.every((zone) => zone?.zoneRgb === currentZoneRgb)) {
    return [];
  }
  return zones.filter((zone) => zone?.zoneRgb !== currentZoneRgb);
}

export function buildPointSampleRows({ source = null, stateBundle = null, zoneCatalog = [] } = {}) {
  const fishById = fishCatalogLookup(stateBundle);
  const currentZoneRgb = currentZoneRgbFromSource(source);
  return normalizePointSamples(source?.pointSamples).map((sample, index) => {
    const fish = fishById.get(sample.fishId) || {};
    const itemId = Number.parseInt(fish?.itemId, 10);
    const zones = (sample.fullZoneRgbs.length ? sample.fullZoneRgbs : sample.zoneRgbs)
      .map((zoneRgb) => zoneSummary(zoneRgb, zoneCatalog));
    return {
      kind: "point-sample",
      key: `point-sample:${sample.fishId}:${sample.sampleCount}:${sample.lastTsUtc}:${sample.zoneRgbs.join(",")}:${sample.fullZoneRgbs.join(",")}:${index}`,
      fishId: sample.fishId,
      itemId: Number.isInteger(itemId) ? itemId : null,
      fishName: trimString(fish?.name) || mapText("info.fish.unknown"),
      grade: trimString(fish?.grade),
      isPrize: fish?.isPrize === true,
      iconUrl: Number.isInteger(itemId) ? fishItemIconUrl(itemId) : "",
      sampleCount: sample.sampleCount,
      dateText: formatSampleDate(sample.lastTsUtc),
      zoneKind: sample.fullZoneRgbs.length ? "full" : "partial",
      zones: visibleSampleZones(zones, currentZoneRgb),
    };
  });
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
      const label = trimString(fact.label) || sectionTitle || factLabelFallback(key);
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

function definitionsForLayer(layerId) {
  const definitions = {
    zone_mask: [
      {
        key: "zone",
        factKeys: ["zone"],
        name: mapText("hover.zone_name"),
        label: mapText("hover.zone"),
        icon: "hover-zone",
        defaultVisible: true,
      },
      {
        key: "rgb",
        factKeys: ["rgb"],
        name: mapText("hover.rgb"),
        label: mapText("hover.rgb"),
        defaultVisible: true,
      },
    ],
    region_groups: [
      {
        key: "resource_group",
        factKeys: ["resource_group", "resources", "resource_region"],
        name: mapText("hover.resources"),
        label: mapText("hover.resources"),
        icon: "hover-resources",
        defaultVisible: false,
      },
    ],
    regions: [
      {
        key: "origin_region",
        factKeys: ["origin_region", "origin"],
        name: mapText("hover.origin"),
        label: mapText("hover.origin"),
        icon: "trade-origin",
        defaultVisible: false,
      },
    ],
  };
  return definitions[trimString(layerId)] || [];
}

function zoneMaskFactRow(definition, sample, zoneCatalog) {
  if (definition.key === "zone") {
    const zoneRgb = Number.isFinite(Number(sample?.rgbU32)) ? Number(sample.rgbU32) : null;
    const zoneName =
      zoneRgb != null ? trimString(zoneDisplayNameFromCatalog(zoneCatalog, zoneRgb)) : "";
    const detailFact = detailFactByKeys(sample, definition.factKeys);
    const value = zoneName || detailFact?.value || "";
    return {
      key: definition.key,
      name: definition.name,
      label: definition.label,
      value,
      icon: definition.icon,
      defaultVisible: definition.defaultVisible,
      ...(detailFact?.statusIcon ? { statusIcon: detailFact.statusIcon } : {}),
      ...(detailFact?.statusIconTone ? { statusIconTone: detailFact.statusIconTone } : {}),
    };
  }

  if (definition.key === "rgb") {
    const rgb = normalizeRgbTriplet(sample?.rgb);
    return {
      key: definition.key,
      name: definition.name,
      label: definition.label,
      value: rgbDisplayValue(rgb),
      defaultVisible: definition.defaultVisible,
      ...(rgb ? { swatchRgb: rgbSwatchValue(rgb) } : {}),
    };
  }

  return null;
}

function knownFactRowsForSample(layerId, sample, zoneCatalog) {
  return definitionsForLayer(layerId)
    .map((definition) => {
      if (trimString(layerId) === "zone_mask") {
        return zoneMaskFactRow(definition, sample, zoneCatalog);
      }
      const fact = detailFactByKeys(sample, definition.factKeys);
      return {
        key: definition.key,
        name: definition.name,
        label: definition.label,
        value: fact?.value || "",
        icon: definition.icon || fact?.icon || DEFAULT_FACT_ICON,
        defaultVisible: definition.defaultVisible,
        ...(fact?.statusIcon ? { statusIcon: fact.statusIcon } : {}),
        ...(fact?.statusIconTone ? { statusIconTone: fact.statusIconTone } : {}),
      };
    })
    .filter(Boolean);
}

function discoveredFactRowsForSample(layerId, sample) {
  const consumedKeys = new Set(
    definitionsForLayer(layerId)
      .flatMap((definition) => definition.factKeys)
      .map(trimString)
      .filter(Boolean),
  );
  return normalizeDetailFacts(sample)
    .filter((fact) => !consumedKeys.has(fact.key))
    .map((fact) => ({
      key: fact.key,
      name: fact.label || factLabelFallback(fact.key),
      label: fact.label || factLabelFallback(fact.key),
      value: fact.value,
      icon: fact.icon || DEFAULT_FACT_ICON,
      defaultVisible: false,
      ...(fact.statusIcon ? { statusIcon: fact.statusIcon } : {}),
      ...(fact.statusIconTone ? { statusIconTone: fact.statusIconTone } : {}),
    }));
}

function visibilityOverride(visibilityByLayer, layerId, factKey) {
  const layerState = isPlainObject(visibilityByLayer?.[layerId]) ? visibilityByLayer[layerId] : null;
  return layerState && typeof layerState[factKey] === "boolean" ? layerState[factKey] : null;
}

export function normalizeHoverFactVisibilityByLayer(raw) {
  if (!isPlainObject(raw)) {
    return {};
  }
  const next = {};
  for (const [layerIdInput, facts] of Object.entries(raw)) {
    const layerId = trimString(layerIdInput);
    if (!layerId || !isPlainObject(facts)) {
      continue;
    }
    const nextFacts = {};
    for (const [factKeyInput, enabled] of Object.entries(facts)) {
      const factKey = trimString(factKeyInput);
      if (!factKey || typeof enabled !== "boolean") {
        continue;
      }
      nextFacts[factKey] = enabled;
    }
    if (Object.keys(nextFacts).length) {
      next[layerId] = nextFacts;
    }
  }
  return next;
}

export function buildLayerHoverSettingsRows({
  layerId,
  sample = null,
  zoneCatalog = [],
  visibilityByLayer = {},
} = {}) {
  const normalizedLayerId = trimString(layerId);
  const rows = [
    ...knownFactRowsForSample(normalizedLayerId, sample, zoneCatalog),
    ...discoveredFactRowsForSample(normalizedLayerId, sample),
  ];
  const seen = new Set();
  return rows.flatMap((row) => {
    const key = trimString(row?.key);
    if (!key || seen.has(key)) {
      return [];
    }
    seen.add(key);
    const defaultVisible = row.defaultVisible === true;
    const override = visibilityOverride(visibilityByLayer, normalizedLayerId, key);
    return [
      {
        ...row,
        key,
        enabled: override == null ? defaultVisible : override,
      },
    ];
  });
}

function hoverFactRowsForLayer(layerId, sample, visibilityByLayer, zoneCatalog) {
  return buildLayerHoverSettingsRows({
    layerId,
    sample,
    zoneCatalog,
    visibilityByLayer,
  }).filter((row) => row.enabled && trimString(row.value));
}

function orderedHoverLayerIds(hover, stateBundle) {
  const layerSamples = Array.isArray(hover?.layerSamples) ? hover.layerSamples : [];
  const sampleByLayerId = new Map(
    layerSamples
      .map((sample) => [trimString(sample?.layerId), sample])
      .filter(([layerId]) => layerId),
  );
  const orderedLayerIds = resolveLayerEntries(stateBundle || {})
    .map((layer) => trimString(layer?.layerId))
    .filter((layerId) => sampleByLayerId.has(layerId))
    .reverse();
  return orderedLayerIds.length
    ? orderedLayerIds
    : layerSamples.map((sample) => trimString(sample?.layerId)).filter(Boolean);
}

export function patchTouchesHoverTooltipSignals(patch) {
  if (!isPlainObject(patch)) {
    return false;
  }
  return Boolean(
    patch._map_runtime?.catalog?.layers != null ||
      patch._map_runtime?.catalog?.fish != null ||
      patch._map_ui?.layers?.hoverFactsVisibleByLayer != null ||
      patch._map_ui?.layers?.sampleHoverVisibleByLayer != null ||
      patch._map_bridged?.filters?.layerIdsOrdered != null,
  );
}

export function buildHoverTooltipRows({
  hover = null,
  stateBundle = null,
  visibilityByLayer = {},
  pointSamplesEnabled = true,
  zoneCatalog = [],
} = {}) {
  const layerSamples = Array.isArray(hover?.layerSamples) ? hover.layerSamples : [];
  const pointRows = pointSamplesEnabled
    ? buildPointSampleRows({ source: hover, stateBundle, zoneCatalog })
    : [];
  if (!layerSamples.length) {
    return pointRows;
  }
  const sampleByLayerId = new Map(
    layerSamples
      .map((sample) => [trimString(sample?.layerId), sample])
      .filter(([layerId]) => layerId),
  );
  const layerRows = orderedHoverLayerIds(hover, stateBundle).flatMap((layerId) => {
    const sample = sampleByLayerId.get(layerId);
    if (!sample) {
      return [];
    }
    return hoverFactRowsForLayer(layerId, sample, visibilityByLayer, zoneCatalog).map((row) => ({
      layerId,
      key: row.key,
      icon: row.icon || DEFAULT_FACT_ICON,
      label: row.label || row.name,
      value: row.value,
      ...(row.swatchRgb ? { swatchRgb: row.swatchRgb } : {}),
      ...(row.statusIcon ? { statusIcon: row.statusIcon } : {}),
      ...(row.statusIconTone ? { statusIconTone: row.statusIconTone } : {}),
    }));
  });
  return [...pointRows, ...layerRows];
}

function previewSampleForLayer(layerId, sources = []) {
  const normalizedLayerId = trimString(layerId);
  if (!normalizedLayerId) {
    return null;
  }
  for (const source of sources) {
    const sample = (Array.isArray(source?.layerSamples) ? source.layerSamples : []).find(
      (candidate) => trimString(candidate?.layerId) === normalizedLayerId,
    );
    if (sample) {
      return sample;
    }
  }
  return null;
}

export function buildLayerPanelHoverFactPreview({
  layerId,
  hover = null,
  selection = null,
  zoneCatalog = [],
  visibilityByLayer = {},
} = {}) {
  const sample = previewSampleForLayer(layerId, [hover, selection]);
  return buildLayerHoverSettingsRows({
    layerId,
    sample,
    zoneCatalog,
    visibilityByLayer,
  });
}

export function nextHoverFactVisibilityByLayer(
  currentVisibilityByLayer,
  layerIdInput,
  factKeyInput,
  enabled,
) {
  const layerId = trimString(layerIdInput);
  const factKey = trimString(factKeyInput);
  if (!layerId || !factKey) {
    return normalizeHoverFactVisibilityByLayer(currentVisibilityByLayer);
  }
  const next = normalizeHoverFactVisibilityByLayer(cloneJson(currentVisibilityByLayer || {}));
  const layerState = isPlainObject(next[layerId]) ? next[layerId] : {};
  layerState[factKey] = enabled === true;
  next[layerId] = layerState;
  return next;
}
