export const MAP_PAGE_PERSIST_SIGNAL_FILTER =
  /^_(?:map_ui\.(?:windowUi|layers(?:\.|$)|search\.(?:query|selectedTerms|expression))|map_bridged\.ui\.(?:diagnosticsOpen|showPoints|showPointIcons|viewMode|pointIconScale)|map_bridged\.filters\.(?:fishIds|zoneRgbs|semanticFieldIdsByLayer|fishFilterTerms|patchId|fromPatchId|toPatchId|layerIdsVisible|layerIdsOrdered|layerOpacities|layerClipMasks|layerWaypointConnectionsVisible|layerWaypointLabelsVisible|layerPointIconsVisible|layerPointIconScales)|map_bookmarks\.entries|map_session(?:\.|$))(?:\.|$)/;

export const MAP_PAGE_EXACT_PATCH_PATHS = Object.freeze([
  "_map_ui.layers.expandedLayerIds",
  "_map_ui.layers.hoverFactsVisibleByLayer",
  "_map_ui.search.expression",
  "_map_bridged.filters.semanticFieldIdsByLayer",
  "_map_bridged.filters.layerOpacities",
  "_map_bridged.filters.layerClipMasks",
  "_map_bridged.filters.layerWaypointConnectionsVisible",
  "_map_bridged.filters.layerWaypointLabelsVisible",
  "_map_bridged.filters.layerPointIconsVisible",
  "_map_bridged.filters.layerPointIconScales",
  "_map_runtime.theme",
  "_map_runtime.view",
  "_map_runtime.selection",
  "_map_runtime.catalog",
  "_map_runtime.statuses",
  "_map_session.view",
  "_map_session.selection",
]);

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function readObjectPath(root, path) {
  return String(path ?? "")
    .split(".")
    .filter(Boolean)
    .reduce((current, key) => {
      if (current && typeof current === "object" && key in current) {
        return current[key];
      }
      return undefined;
    }, root);
}

function hasObjectPath(root, path) {
  if (!root || typeof root !== "object") {
    return false;
  }
  const parts = String(path ?? "").split(".").filter(Boolean);
  if (!parts.length) {
    return false;
  }
  let current = root;
  for (const key of parts) {
    if (!current || typeof current !== "object" || !(key in current)) {
      return false;
    }
    current = current[key];
  }
  return true;
}

function setObjectPath(root, path, value) {
  if (!root || typeof root !== "object") {
    return root;
  }
  const parts = String(path ?? "").split(".").filter(Boolean);
  if (!parts.length) {
    return root;
  }
  let current = root;
  for (const key of parts.slice(0, -1)) {
    if (!isPlainObject(current[key])) {
      current[key] = {};
    }
    current = current[key];
  }
  current[parts[parts.length - 1]] = value;
  return root;
}

function mergeObjectPatch(target, patch) {
  if (!isPlainObject(target) || !isPlainObject(patch)) {
    return target;
  }
  for (const [key, value] of Object.entries(patch)) {
    if (Array.isArray(value)) {
      target[key] = cloneJson(value);
      continue;
    }
    if (isPlainObject(value)) {
      const nextTarget = isPlainObject(target[key]) ? target[key] : {};
      target[key] = nextTarget;
      mergeObjectPatch(nextTarget, value);
      continue;
    }
    target[key] = value;
  }
  return target;
}

function applyExactPatchReplacements(signals, patch, exactPatchPaths = MAP_PAGE_EXACT_PATCH_PATHS) {
  if (!isPlainObject(signals) || !isPlainObject(patch)) {
    return;
  }
  for (const path of exactPatchPaths) {
    if (!hasObjectPath(patch, path)) {
      continue;
    }
    setObjectPath(signals, path, cloneJson(readObjectPath(patch, path)));
  }
}

export function applyMapPageSignalsPatch(signals, patch, options = {}) {
  if (!isPlainObject(signals) || !isPlainObject(patch)) {
    return;
  }
  mergeObjectPatch(signals, cloneJson(patch));
  applyExactPatchReplacements(signals, patch, options.exactPatchPaths);
}

export function patchMatchesSignalFilter(patch, filter, prefix = "") {
  if (!isPlainObject(patch)) {
    return false;
  }
  const include = filter?.include && typeof filter.include.test === "function"
    ? filter.include
    : null;
  const exclude = filter?.exclude && typeof filter.exclude.test === "function"
    ? filter.exclude
    : null;
  return Object.entries(patch).some(([key, value]) => {
    const path = prefix ? `${prefix}.${key}` : key;
    if (include) {
      if (include.test(path)) {
        return true;
      }
    } else if (exclude) {
      if (!exclude.test(path)) {
        return true;
      }
    }
    return isPlainObject(value) && patchMatchesSignalFilter(value, filter, path);
  });
}

export function patchMatchesMapPagePersistFilter(patch) {
  return patchMatchesSignalFilter(patch, { include: MAP_PAGE_PERSIST_SIGNAL_FILTER });
}
