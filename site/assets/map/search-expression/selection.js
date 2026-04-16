import { buildSearchExpressionFromSelectedTerms, coerceSearchExpression, normalizeSearchExpression, selectedSearchTermsFromExpression } from "./core.js";
import { isPlainObject, normalizeIntegerList } from "./shared.js";
import {
  normalizeFishFilterTerms,
  normalizePatchId,
  normalizeSearchTerm,
  normalizeSelectedSearchTerms,
  searchTermKey,
} from "./terms.js";

export function selectedSearchTermsFromLegacyFilters(filters) {
  const source = isPlainObject(filters) ? filters : {};
  const searchExpression = normalizeSearchExpression(source.searchExpression);
  if (searchExpression) {
    return selectedSearchTermsFromExpression(searchExpression);
  }
  const terms = [];
  const exactPatchId = normalizePatchId(source.patchId);
  const fromPatchId = normalizePatchId(source.fromPatchId) || exactPatchId;
  const toPatchId = normalizePatchId(source.toPatchId) || exactPatchId;
  if (fromPatchId) {
    terms.push({ kind: "patch-bound", bound: "from", patchId: fromPatchId });
  }
  if (toPatchId) {
    terms.push({ kind: "patch-bound", bound: "to", patchId: toPatchId });
  }
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

export function resolveSearchExpression(expression, selectedTerms = undefined, legacyFilters = null) {
  if (expression !== undefined) {
    return coerceSearchExpression(expression);
  }
  const legacyExpression = normalizeSearchExpression(legacyFilters?.searchExpression);
  if (legacyExpression) {
    return legacyExpression;
  }
  return buildSearchExpressionFromSelectedTerms(
    resolveSelectedSearchTerms(selectedTerms, legacyFilters),
  );
}

export function resolveSelectedSearchTerms(value, legacyFilters = null, expression = undefined) {
  if (expression !== undefined) {
    return selectedSearchTermsFromExpression(resolveSearchExpression(expression));
  }
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
  let fromPatchId = null;
  let toPatchId = null;

  for (const term of selectedTerms) {
    if (term.kind === "patch-bound") {
      const patchId = normalizePatchId(term.patchId);
      if (!patchId) {
        continue;
      }
      if (term.bound === "from") {
        fromPatchId = patchId;
        continue;
      }
      if (term.bound === "to") {
        toPatchId = patchId;
      }
      continue;
    }
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
    patchId: fromPatchId && toPatchId && fromPatchId === toPatchId ? fromPatchId : null,
    fromPatchId,
    toPatchId,
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
  return buildSearchExpressionStatePatch(
    buildSearchExpressionFromSelectedTerms(selectedTerms),
    searchPatch,
  );
}

export function buildSearchExpressionStatePatch(expression, searchPatch = null) {
  const normalizedExpression = resolveSearchExpression(expression);
  const normalizedTerms = selectedSearchTermsFromExpression(normalizedExpression);
  const projection = projectSelectedSearchTermsToBridgedFilters(normalizedTerms);
  const patch = {
    _map_ui: {
      search: {
        expression: normalizedExpression,
        selectedTerms: normalizedTerms,
      },
    },
    _map_bridged: {
      filters: {
        fishIds: projection.fishIds,
        zoneRgbs: projection.zoneRgbs,
        semanticFieldIdsByLayer: projection.semanticFieldIdsByLayer,
        fishFilterTerms: projection.fishFilterTerms,
        patchId: projection.patchId,
        fromPatchId: projection.fromPatchId,
        toPatchId: projection.toPatchId,
        searchExpression: normalizedExpression,
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
