function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function normalizeExpandedLayerIds(values) {
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

export function toggleExpandedLayerIds(values, layerId) {
  const normalizedLayerId = String(layerId ?? "").trim();
  if (!normalizedLayerId) {
    return normalizeExpandedLayerIds(values);
  }
  const current = normalizeExpandedLayerIds(values);
  if (current.includes(normalizedLayerId)) {
    return current.filter((candidate) => candidate !== normalizedLayerId);
  }
  return current.concat(normalizedLayerId);
}

export function patchTouchesLayerPanelSignals(patch) {
  if (!isPlainObject(patch)) {
    return false;
  }
  if (patch._map_runtime?.ready != null) {
    return true;
  }
  if (patch._map_runtime?.catalog?.layers != null) {
    return true;
  }
  if (patch._map_runtime?.selection != null) {
    return true;
  }
  if (patch._map_bridged?.filters != null) {
    return true;
  }
  if (patch._map_ui?.layers != null) {
    return true;
  }
  return false;
}

export function buildLayerPanelStateBundle(signals) {
  const runtime = isPlainObject(signals?._map_runtime) ? signals._map_runtime : {};
  const bridged = isPlainObject(signals?._map_bridged?.filters) ? signals._map_bridged.filters : {};
  return {
    state: {
      ready: runtime.ready === true,
      catalog: {
        layers: Array.isArray(runtime.catalog?.layers) ? cloneJson(runtime.catalog.layers) : [],
      },
    },
    inputState: {
      filters: cloneJson(bridged),
    },
  };
}
