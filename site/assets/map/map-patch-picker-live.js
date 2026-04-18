import {
  projectSelectedSearchTermsToBridgedFilters,
  resolveSearchExpression,
  resolveSelectedSearchTerms,
} from "./map-search-contract.js";

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function normalizePatchId(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function normalizePatchLabel(patchId, patchName) {
  const normalizedName = String(patchName ?? "").trim();
  return normalizedName || patchId;
}

function normalizePatchStartTs(value) {
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric : 0;
}

function normalizePatchSummary(patch) {
  const patchId = normalizePatchId(patch?.patchId ?? patch?.patch_id);
  if (!patchId) {
    return null;
  }
  return {
    patchId,
    label: normalizePatchLabel(patchId, patch?.patchName ?? patch?.patch_name),
    startTsUtc: normalizePatchStartTs(patch?.startTsUtc ?? patch?.start_ts_utc),
  };
}

function comparePatchSummariesDescending(left, right) {
  if (right.startTsUtc !== left.startTsUtc) {
    return right.startTsUtc - left.startTsUtc;
  }
  return right.patchId.localeCompare(left.patchId);
}

export function normalizePatchCatalog(patches) {
  return (Array.isArray(patches) ? patches : [])
    .map(normalizePatchSummary)
    .filter(Boolean)
    .sort(comparePatchSummariesDescending);
}

export function buildPatchPickerStateBundle(signals) {
  const runtime = isPlainObject(signals?._map_runtime) ? signals._map_runtime : {};
  const search = isPlainObject(signals?._map_ui?.search) ? signals._map_ui.search : {};
  const searchExpression = resolveSearchExpression(search.expression, search.selectedTerms);
  const searchProjection = projectSelectedSearchTermsToBridgedFilters(
    resolveSelectedSearchTerms(search.selectedTerms, searchExpression),
  );
  return {
    state: {
      ready: runtime.ready === true,
      catalog: {
        patches: normalizePatchCatalog(runtime.catalog?.patches),
      },
    },
    inputState: {
      filters: {
        patchId: normalizePatchId(searchProjection.patchId) || null,
        fromPatchId: normalizePatchId(searchProjection.fromPatchId) || null,
        toPatchId: normalizePatchId(searchProjection.toPatchId) || null,
      },
    },
  };
}

export function patchTouchesPatchPickerSignals(patch) {
  if (!isPlainObject(patch)) {
    return false;
  }
  if (patch._map_runtime?.ready != null) {
    return true;
  }
  if (patch._map_runtime?.catalog?.patches != null) {
    return true;
  }
  const search = patch._map_ui?.search;
  return Boolean(
    search
      && (
        search.expression != null
        || search.selectedTerms != null
      ),
  );
}
