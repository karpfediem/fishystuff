import { MAP_SEARCH_LAYER_SUPPORT } from "./map-search-contract.js";

const TERM_KIND_LABELS = Object.freeze({
  fish: "Fish",
  "fish-filter": "Fish filters",
  zone: "Zones",
  semantic: "Semantic fields",
});

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function normalizeStringList(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  const next = [];
  const seen = new Set();
  for (const value of values) {
    const normalized = String(value ?? "").trim();
    if (!normalized || seen.has(normalized)) {
      continue;
    }
    seen.add(normalized);
    next.push(normalized);
  }
  return next;
}

function normalizeIntegerList(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  const next = [];
  const seen = new Set();
  for (const value of values) {
    const normalized = Number.parseInt(value, 10);
    if (!Number.isInteger(normalized) || seen.has(normalized)) {
      continue;
    }
    seen.add(normalized);
    next.push(normalized);
  }
  return next;
}

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function normalizeLayerClipMasks(value) {
  const source = isPlainObject(value) ? value : {};
  const next = {};
  for (const [layerIdRaw, maskLayerIdRaw] of Object.entries(source)) {
    const layerId = String(layerIdRaw ?? "").trim();
    const maskLayerId = String(maskLayerIdRaw ?? "").trim();
    if (!layerId || !maskLayerId || layerId === maskLayerId) {
      continue;
    }
    next[layerId] = maskLayerId;
  }
  return next;
}

export function layerSearchTermKindLabels(layerId) {
  return (MAP_SEARCH_LAYER_SUPPORT[String(layerId ?? "").trim()]?.termKinds || []).map(
    (termKind) => TERM_KIND_LABELS[termKind] || termKind,
  );
}

export function hasActiveZoneSearchFilters(filters) {
  const source = isPlainObject(filters) ? filters : {};
  const semanticFieldIdsByLayer = isPlainObject(source.semanticFieldIdsByLayer)
    ? source.semanticFieldIdsByLayer
    : {};
  return (
    normalizeStringList(source.fishFilterTerms).length > 0 ||
    normalizeIntegerList(source.fishIds).length > 0 ||
    normalizeIntegerList(source.zoneRgbs).length > 0 ||
    normalizeIntegerList(semanticFieldIdsByLayer.zone_mask).length > 0
  );
}

export function buildLayerSearchEffects(filters) {
  const source = isPlainObject(filters) ? filters : {};
  const activeZoneSearch = hasActiveZoneSearchFilters(source);
  const effectiveLayerClipMasks = normalizeLayerClipMasks(source.layerClipMasks);

  return {
    activeZoneSearch,
    effectiveLayerClipMasks: cloneJson(effectiveLayerClipMasks),
  };
}
