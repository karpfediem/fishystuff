export const FISH_FILTER_TERM_ORDER = Object.freeze([
  "favourite",
  "missing",
  "red",
  "yellow",
  "blue",
  "green",
  "white",
]);

export const MAP_SEARCH_LAYER_SUPPORT = Object.freeze({
  bookmarks: Object.freeze({
    termKinds: Object.freeze([]),
    attachmentClipModes: Object.freeze([]),
  }),
  fish_evidence: Object.freeze({
    termKinds: Object.freeze(["fish", "fish-filter", "zone"]),
    attachmentClipModes: Object.freeze(["zone-membership"]),
  }),
  minimap: Object.freeze({
    termKinds: Object.freeze([]),
    attachmentClipModes: Object.freeze(["mask-sample"]),
  }),
  node_waypoints: Object.freeze({
    termKinds: Object.freeze([]),
    attachmentClipModes: Object.freeze([]),
  }),
  region_groups: Object.freeze({
    termKinds: Object.freeze(["semantic"]),
    attachmentClipModes: Object.freeze(["mask-sample"]),
  }),
  regions: Object.freeze({
    termKinds: Object.freeze(["semantic"]),
    attachmentClipModes: Object.freeze(["mask-sample"]),
  }),
  zone_mask: Object.freeze({
    termKinds: Object.freeze(["fish", "fish-filter", "zone"]),
    attachmentClipModes: Object.freeze([]),
  }),
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

export function normalizeFishFilterTerm(value) {
  const normalized = String(value ?? "").trim().toLowerCase();
  if (
    normalized === "favourite" ||
    normalized === "favourites" ||
    normalized === "favorite" ||
    normalized === "favorites"
  ) {
    return "favourite";
  }
  if (
    normalized === "missing" ||
    normalized === "uncaught" ||
    normalized === "not caught" ||
    normalized === "not yet caught"
  ) {
    return "missing";
  }
  if (normalized === "red" || normalized === "prize") {
    return "red";
  }
  if (normalized === "yellow" || normalized === "rare") {
    return "yellow";
  }
  if (
    normalized === "blue" ||
    normalized === "highquality" ||
    normalized === "high_quality" ||
    normalized === "high-quality"
  ) {
    return "blue";
  }
  if (normalized === "green" || normalized === "general") {
    return "green";
  }
  if (normalized === "white" || normalized === "trash") {
    return "white";
  }
  return "";
}

export function normalizeFishFilterTerms(values) {
  const selected = new Set();
  for (const value of Array.isArray(values) ? values : []) {
    const normalized = normalizeFishFilterTerm(value);
    if (normalized) {
      selected.add(normalized);
    }
  }
  return FISH_FILTER_TERM_ORDER.filter((term) => selected.has(term));
}

export function normalizeSearchTerm(raw) {
  if (!isPlainObject(raw)) {
    return null;
  }
  const kind = String(raw.kind ?? "").trim().toLowerCase();
  if (kind === "fish-filter") {
    const term = normalizeFishFilterTerm(raw.term ?? raw.fishFilterTerm);
    return term ? { kind: "fish-filter", term } : null;
  }
  if (kind === "fish") {
    const fishId = Number.parseInt(raw.fishId ?? raw.itemId, 10);
    return Number.isInteger(fishId) && fishId > 0 ? { kind: "fish", fishId } : null;
  }
  if (kind === "zone") {
    const zoneRgb = Number.parseInt(raw.zoneRgb ?? raw.fieldId, 10);
    return Number.isInteger(zoneRgb) && zoneRgb > 0 ? { kind: "zone", zoneRgb } : null;
  }
  if (kind === "semantic") {
    const layerId = String(raw.layerId ?? "").trim();
    const fieldId = Number.parseInt(raw.fieldId, 10);
    if (layerId === "zone_mask") {
      return Number.isInteger(fieldId) && fieldId > 0 ? { kind: "zone", zoneRgb: fieldId } : null;
    }
    if (!layerId || !Number.isInteger(fieldId) || fieldId <= 0) {
      return null;
    }
    return { kind: "semantic", layerId, fieldId };
  }
  return null;
}

export function searchTermKey(term) {
  if (!term || typeof term !== "object") {
    return "";
  }
  if (term.kind === "fish-filter") {
    return `fish-filter:${normalizeFishFilterTerm(term.term)}`;
  }
  if (term.kind === "fish") {
    const fishId = Number.parseInt(term.fishId, 10);
    return Number.isInteger(fishId) ? `fish:${fishId}` : "";
  }
  if (term.kind === "zone") {
    const zoneRgb = Number.parseInt(term.zoneRgb, 10);
    return Number.isInteger(zoneRgb) ? `zone:${zoneRgb}` : "";
  }
  if (term.kind === "semantic") {
    const layerId = String(term.layerId ?? "").trim();
    const fieldId = Number.parseInt(term.fieldId, 10);
    return layerId && Number.isInteger(fieldId) ? `semantic:${layerId}:${fieldId}` : "";
  }
  return "";
}

export function normalizeSelectedSearchTerms(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  const next = [];
  const seen = new Set();
  for (const value of values) {
    const normalized = normalizeSearchTerm(value);
    if (!normalized) {
      continue;
    }
    const key = searchTermKey(normalized);
    if (!key || seen.has(key)) {
      continue;
    }
    seen.add(key);
    next.push(normalized);
  }
  return next;
}

export function selectedSearchTermsFromLegacyFilters(filters) {
  const source = isPlainObject(filters) ? filters : {};
  const terms = [];
  for (const term of normalizeFishFilterTerms(source.fishFilterTerms)) {
    terms.push({ kind: "fish-filter", term });
  }
  for (const fishId of normalizeIntegerList(source.fishIds)) {
    terms.push({ kind: "fish", fishId });
  }
  for (const zoneRgb of normalizeIntegerList(source.zoneRgbs)) {
    terms.push({ kind: "zone", zoneRgb });
  }
  const byLayer = isPlainObject(source.semanticFieldIdsByLayer)
    ? source.semanticFieldIdsByLayer
    : {};
  for (const zoneRgb of normalizeIntegerList(byLayer.zone_mask)) {
    terms.push({ kind: "zone", zoneRgb });
  }
  for (const [layerIdRaw, fieldIdsRaw] of Object.entries(byLayer)) {
    const layerId = String(layerIdRaw ?? "").trim();
    if (!layerId || layerId === "zone_mask") {
      continue;
    }
    for (const fieldId of normalizeIntegerList(fieldIdsRaw)) {
      terms.push({ kind: "semantic", layerId, fieldId });
    }
  }
  return normalizeSelectedSearchTerms(terms);
}

export function resolveSelectedSearchTerms(value, legacyFilters = null) {
  const selectedTerms = normalizeSelectedSearchTerms(value);
  if (selectedTerms.length || Array.isArray(value)) {
    return selectedTerms;
  }
  return selectedSearchTermsFromLegacyFilters(legacyFilters);
}

export function projectSelectedSearchTermsToBridgedFilters(terms) {
  const selectedTerms = normalizeSelectedSearchTerms(terms);
  const fishIds = [];
  const zoneRgbs = [];
  const fishFilterTerms = [];
  const semanticFieldIdsByLayer = {};

  for (const term of selectedTerms) {
    if (term.kind === "fish") {
      fishIds.push(term.fishId);
      continue;
    }
    if (term.kind === "zone") {
      zoneRgbs.push(term.zoneRgb);
      continue;
    }
    if (term.kind === "fish-filter") {
      fishFilterTerms.push(term.term);
      continue;
    }
    if (term.kind === "semantic") {
      const fieldIds = semanticFieldIdsByLayer[term.layerId] || [];
      fieldIds.push(term.fieldId);
      semanticFieldIdsByLayer[term.layerId] = fieldIds;
    }
  }

  if (zoneRgbs.length) {
    semanticFieldIdsByLayer.zone_mask = zoneRgbs.slice();
  }

  return {
    fishIds,
    zoneRgbs,
    semanticFieldIdsByLayer,
    fishFilterTerms,
  };
}

export function addSelectedSearchTerm(selectedTerms, term) {
  return normalizeSelectedSearchTerms(
    normalizeSelectedSearchTerms(selectedTerms).concat(term),
  );
}

export function removeSelectedSearchTerm(selectedTerms, target) {
  const normalizedTarget = normalizeSearchTerm(target);
  if (!normalizedTarget) {
    return normalizeSelectedSearchTerms(selectedTerms);
  }
  const targetKey = searchTermKey(normalizedTarget);
  return normalizeSelectedSearchTerms(selectedTerms).filter(
    (term) => searchTermKey(term) !== targetKey,
  );
}

export function buildSearchSelectionStatePatch(selectedTerms, searchPatch = null) {
  const normalizedTerms = normalizeSelectedSearchTerms(selectedTerms);
  const projection = projectSelectedSearchTermsToBridgedFilters(normalizedTerms);
  const patch = {
    _map_ui: {
      search: {
        selectedTerms: normalizedTerms,
      },
    },
    _map_bridged: {
      filters: {
        fishIds: projection.fishIds,
        zoneRgbs: projection.zoneRgbs,
        semanticFieldIdsByLayer: projection.semanticFieldIdsByLayer,
        fishFilterTerms: projection.fishFilterTerms,
      },
    },
  };
  if (isPlainObject(searchPatch)) {
    patch._map_ui.search = {
      ...patch._map_ui.search,
      ...searchPatch,
    };
  }
  return patch;
}

export function layerSupportsSearchTerm(layerId, termKind) {
  const normalizedLayerId = String(layerId ?? "").trim();
  const normalizedTermKind = String(termKind ?? "").trim();
  const layerSupport = MAP_SEARCH_LAYER_SUPPORT[normalizedLayerId];
  return !!layerSupport?.termKinds?.includes(normalizedTermKind);
}

export function layerSupportsAttachmentClipMode(layerId, clipMode) {
  const normalizedLayerId = String(layerId ?? "").trim();
  const normalizedClipMode = String(clipMode ?? "").trim();
  const layerSupport = MAP_SEARCH_LAYER_SUPPORT[normalizedLayerId];
  return !!layerSupport?.attachmentClipModes?.includes(normalizedClipMode);
}
