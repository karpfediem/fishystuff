import {
  normalizeFishFilterTerms,
  projectSelectedSearchTermsToBridgedFilters,
  resolveSelectedSearchTerms,
} from "./map-search-contract.js";

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
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

function normalizeSemanticFieldIdsByLayer(values) {
  if (!isPlainObject(values)) {
    return {};
  }
  const out = {};
  for (const [layerIdRaw, fieldIdsRaw] of Object.entries(values)) {
    const layerId = String(layerIdRaw ?? "").trim();
    if (!layerId || !Array.isArray(fieldIdsRaw)) {
      continue;
    }
    const fieldIds = normalizeIntegerList(fieldIdsRaw);
    if (fieldIds.length) {
      out[layerId] = fieldIds;
    }
  }
  return out;
}

function normalizeProjectedFilters(value) {
  const source = isPlainObject(value) ? value : {};
  return {
    fishIds: normalizeIntegerList(source.fishIds),
    zoneRgbs: normalizeIntegerList(source.zoneRgbs),
    semanticFieldIdsByLayer: normalizeSemanticFieldIdsByLayer(source.semanticFieldIdsByLayer),
    fishFilterTerms: normalizeFishFilterTerms(source.fishFilterTerms),
  };
}

function projectedFiltersJson(filters) {
  const normalized = normalizeProjectedFilters(filters);
  const semanticFieldIdsByLayer = Object.keys(normalized.semanticFieldIdsByLayer)
    .sort()
    .reduce((out, layerId) => {
      out[layerId] = normalized.semanticFieldIdsByLayer[layerId];
      return out;
    }, {});
  return JSON.stringify({
    fishIds: normalized.fishIds,
    zoneRgbs: normalized.zoneRgbs,
    semanticFieldIdsByLayer,
    fishFilterTerms: normalized.fishFilterTerms,
  });
}

export function resolveSearchProjection(signals) {
  const search = isPlainObject(signals?._map_ui?.search) ? signals._map_ui.search : {};
  const filters = isPlainObject(signals?._map_bridged?.filters) ? signals._map_bridged.filters : {};
  const selectedTerms = resolveSelectedSearchTerms(search.selectedTerms, filters);
  return normalizeProjectedFilters(projectSelectedSearchTermsToBridgedFilters(selectedTerms));
}

export function buildSearchProjectionSignalPatch(signals) {
  const projected = resolveSearchProjection(signals);
  const current = normalizeProjectedFilters(signals?._map_bridged?.filters);
  if (projectedFiltersJson(projected) === projectedFiltersJson(current)) {
    return null;
  }
  return {
    _map_bridged: {
      filters: cloneJson(projected),
    },
  };
}
