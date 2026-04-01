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

function selectedPatchId(filters, bound) {
  const patchId = normalizePatchId(filters?.patchId);
  if (bound === "from") {
    return normalizePatchId(filters?.fromPatchId) || patchId;
  }
  if (bound === "to") {
    return normalizePatchId(filters?.toPatchId) || patchId;
  }
  return patchId;
}

export function normalizePatchCatalog(patches) {
  return (Array.isArray(patches) ? patches : [])
    .map(normalizePatchSummary)
    .filter(Boolean)
    .sort(comparePatchSummariesDescending);
}

export function buildPatchPickerStateBundle(signals) {
  const runtime = isPlainObject(signals?._map_runtime) ? signals._map_runtime : {};
  const bridgedFilters = isPlainObject(signals?._map_bridged?.filters) ? signals._map_bridged.filters : {};
  return {
    state: {
      ready: runtime.ready === true,
      catalog: {
        patches: normalizePatchCatalog(runtime.catalog?.patches),
      },
    },
    inputState: {
      filters: {
        patchId: normalizePatchId(bridgedFilters.patchId) || null,
        fromPatchId: normalizePatchId(bridgedFilters.fromPatchId) || null,
        toPatchId: normalizePatchId(bridgedFilters.toPatchId) || null,
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
  const filters = patch._map_bridged?.filters;
  return Boolean(
    filters
      && (
        filters.patchId != null
        || filters.fromPatchId != null
        || filters.toPatchId != null
      ),
  );
}

export function buildPatchPickerDefaultSignalPatch(signals) {
  const bundle = buildPatchPickerStateBundle(signals);
  if (!bundle.state.ready || !bundle.state.catalog.patches.length) {
    return null;
  }
  const fromPatchId = selectedPatchId(bundle.inputState.filters, "from");
  if (fromPatchId) {
    return null;
  }
  const oldestPatchId = bundle.state.catalog.patches.at(-1)?.patchId;
  if (!oldestPatchId) {
    return null;
  }
  return {
    _map_bridged: {
      filters: {
        fromPatchId: oldestPatchId,
      },
    },
  };
}
