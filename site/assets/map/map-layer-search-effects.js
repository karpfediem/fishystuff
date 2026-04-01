import { MAP_SEARCH_LAYER_SUPPORT } from "./map-search-contract.js";

export const DEFAULT_LAYER_SEARCH_CLIPS = Object.freeze({
  fish_evidence: "zone-membership",
});

const CLIP_MODE_LABELS = Object.freeze({
  "zone-membership": "Clip to visible Zone Mask",
  "mask-sample": "Mask to visible Zone Mask",
});

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

export function normalizeLayerSearchClips(value) {
  const source = isPlainObject(value) ? value : {};
  const next = {};
  for (const [layerIdRaw, clipModeRaw] of Object.entries(source)) {
    const layerId = String(layerIdRaw ?? "").trim();
    const clipMode = String(clipModeRaw ?? "").trim();
    const supportedClipModes = MAP_SEARCH_LAYER_SUPPORT[layerId]?.clipModes || [];
    if (!layerId || !clipMode || !supportedClipModes.includes(clipMode)) {
      continue;
    }
    next[layerId] = clipMode;
  }
  return next;
}

export function layerSearchTermKindLabels(layerId) {
  return (MAP_SEARCH_LAYER_SUPPORT[String(layerId ?? "").trim()]?.termKinds || []).map(
    (termKind) => TERM_KIND_LABELS[termKind] || termKind,
  );
}

export function layerSearchClipRows(layerId, layerSearchClips) {
  const normalizedLayerId = String(layerId ?? "").trim();
  const supportedClipModes = MAP_SEARCH_LAYER_SUPPORT[normalizedLayerId]?.clipModes || [];
  const normalizedClips = normalizeLayerSearchClips(layerSearchClips);
  return supportedClipModes.map((clipMode) => ({
    layerId: normalizedLayerId,
    clipMode,
    label: CLIP_MODE_LABELS[clipMode] || clipMode,
    enabled: normalizedClips[normalizedLayerId] === clipMode,
  }));
}

export function buildLayerSearchClipsPatch(layerSearchClips, layerId, clipMode, enabled) {
  const normalized = normalizeLayerSearchClips(layerSearchClips);
  const normalizedLayerId = String(layerId ?? "").trim();
  const normalizedClipMode = String(clipMode ?? "").trim();
  if (!normalizedLayerId || !normalizedClipMode) {
    return normalized;
  }
  if (enabled === false) {
    delete normalized[normalizedLayerId];
    return normalized;
  }
  const supportedClipModes = MAP_SEARCH_LAYER_SUPPORT[normalizedLayerId]?.clipModes || [];
  if (!supportedClipModes.includes(normalizedClipMode)) {
    return normalized;
  }
  normalized[normalizedLayerId] = normalizedClipMode;
  return normalized;
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
  const layerSearchClips = normalizeLayerSearchClips(source.layerSearchClips);
  const manualLayerClipMasks = isPlainObject(source.layerClipMasks)
    ? cloneJson(source.layerClipMasks)
    : {};
  const activeZoneSearch = hasActiveZoneSearchFilters(source);
  const effectiveLayerClipMasks = cloneJson(manualLayerClipMasks);
  const zoneMembershipLayerIds = [];

  if (!activeZoneSearch) {
    return {
      activeZoneSearch,
      layerSearchClips,
      effectiveLayerClipMasks,
      zoneMembershipLayerIds,
    };
  }

  for (const [layerId, clipMode] of Object.entries(layerSearchClips)) {
    if (clipMode === "zone-membership") {
      zoneMembershipLayerIds.push(layerId);
      continue;
    }
    if (clipMode === "mask-sample" && !String(effectiveLayerClipMasks[layerId] || "").trim()) {
      effectiveLayerClipMasks[layerId] = "zone_mask";
    }
  }

  return {
    activeZoneSearch,
    layerSearchClips,
    effectiveLayerClipMasks,
    zoneMembershipLayerIds,
  };
}
