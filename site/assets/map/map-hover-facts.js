import { resolveLayerEntries } from "./map-layer-state.js";
import {
  zoneCatalogEntryForRgb,
  zoneDisplayNameFromCatalog,
} from "./map-zone-catalog.js";

const DEFAULT_FACT_ICON = "information-circle";

const LAYER_HOVER_FACT_DEFINITIONS = Object.freeze({
  zone_mask: Object.freeze([
    Object.freeze({
      key: "zone",
      factKeys: Object.freeze(["zone"]),
      name: "Zone Name",
      label: "Zone",
      icon: "hover-zone",
      defaultVisible: true,
    }),
    Object.freeze({
      key: "rgb",
      factKeys: Object.freeze(["rgb"]),
      name: "RGB",
      label: "RGB",
      defaultVisible: true,
    }),
  ]),
  region_groups: Object.freeze([
    Object.freeze({
      key: "resource_group",
      factKeys: Object.freeze(["resource_group", "resources", "resource_region"]),
      name: "Resources",
      label: "Resources",
      icon: "hover-resources",
      defaultVisible: false,
    }),
  ]),
  regions: Object.freeze([
    Object.freeze({
      key: "origin_region",
      factKeys: Object.freeze(["origin_region", "origin"]),
      name: "Origin",
      label: "Origin",
      icon: "trade-origin",
      defaultVisible: false,
    }),
  ]),
});

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
  return trimString(value).replace(/_/g, " ") || "Details";
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
  return LAYER_HOVER_FACT_DEFINITIONS[trimString(layerId)] || [];
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
      patch._map_ui?.layers?.hoverFactsVisibleByLayer != null ||
      patch._map_bridged?.filters?.layerIdsOrdered != null,
  );
}

export function buildHoverTooltipRows({
  hover = null,
  stateBundle = null,
  visibilityByLayer = {},
  zoneCatalog = [],
} = {}) {
  const layerSamples = Array.isArray(hover?.layerSamples) ? hover.layerSamples : [];
  if (!layerSamples.length) {
    return [];
  }
  const sampleByLayerId = new Map(
    layerSamples
      .map((sample) => [trimString(sample?.layerId), sample])
      .filter(([layerId]) => layerId),
  );
  return orderedHoverLayerIds(hover, stateBundle).flatMap((layerId) => {
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
