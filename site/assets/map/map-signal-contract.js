import {
  FISHYMAP_CONTRACT_VERSION,
  FISHYMAP_POINT_ICON_SCALE_DEFAULT,
  FISHYMAP_POINT_ICON_SCALE_MIN,
} from "./map-host.js";
import {
  EMPTY_SEARCH_EXPRESSION,
  resolveSearchExpression,
  resolveSelectedSearchTerms,
} from "./map-search-contract.js";

export const DEFAULT_ZONE_INFO_TAB = "";
export const DEFAULT_AUTO_ADJUST_VIEW = true;
export const DEFAULT_NORMALIZE_RATES = true;
export const DEFAULT_ENABLED_LAYER_IDS = Object.freeze([
  "bookmarks",
  "fish_evidence",
  "zone_mask",
  "minimap",
]);

export const DEFAULT_WINDOW_UI_STATE = Object.freeze({
  search: Object.freeze({ open: true, collapsed: false, x: null, y: null }),
  settings: Object.freeze({
    open: false,
    collapsed: false,
    x: null,
    y: null,
    autoAdjustView: DEFAULT_AUTO_ADJUST_VIEW,
    normalizeRates: DEFAULT_NORMALIZE_RATES,
  }),
  zoneInfo: Object.freeze({
    open: true,
    collapsed: false,
    x: null,
    y: null,
    tab: DEFAULT_ZONE_INFO_TAB,
  }),
  layers: Object.freeze({ open: true, collapsed: false, x: null, y: null }),
  bookmarks: Object.freeze({ open: false, collapsed: false, x: null, y: null }),
});

export const DEFAULT_MAP_UI_SIGNAL_STATE = Object.freeze({
  windowUi: DEFAULT_WINDOW_UI_STATE,
  search: Object.freeze({
    open: false,
    query: "",
    expression: EMPTY_SEARCH_EXPRESSION,
    selectedTerms: [],
  }),
  bookmarks: Object.freeze({ placing: false, selectedIds: [] }),
  layers: Object.freeze({
    expandedLayerIds: [],
    hoverFactsVisibleByLayer: {},
  }),
});

export const MAP_BRIDGE_SHARED_SIGNAL_WHITELIST = Object.freeze({
  bridged: Object.freeze({
    branch: "_map_bridged",
    filters: Object.freeze([
      "fishIds",
      "zoneRgbs",
      "semanticFieldIdsByLayer",
      "fishFilterTerms",
      "searchExpression",
      "patchId",
      "fromPatchId",
      "toPatchId",
      "layerIdsVisible",
      "layerIdsOrdered",
      "layerFilterBindingIdsDisabledByLayer",
      "layerOpacities",
      "layerClipMasks",
      "layerWaypointConnectionsVisible",
      "layerWaypointLabelsVisible",
      "layerPointIconsVisible",
      "layerPointIconScales",
    ]),
    ui: Object.freeze([
      "diagnosticsOpen",
      "showPoints",
      "showPointIcons",
      "viewMode",
      "pointIconScale",
      "bookmarkSelectedIds",
      "bookmarks",
    ]),
  }),
  session: Object.freeze({
    branch: "_map_session",
    fields: Object.freeze(["view", "selection"]),
  }),
  bookmarks: Object.freeze({
    branch: "_map_bookmarks",
    fields: Object.freeze(["entries"]),
  }),
  sharedFish: Object.freeze({
    branch: "_shared_fish",
    fields: Object.freeze(["caughtIds", "favouriteIds"]),
  }),
  actions: Object.freeze({
    branch: "_map_actions",
    fields: Object.freeze([
      "resetViewToken",
      "resetUiToken",
      "saveMapPresetToken",
      "discardMapPresetToken",
      "focusWorldPointToken",
      "focusWorldPoint",
    ]),
  }),
});

export const MAP_BRIDGE_SHARED_SIGNAL_BRANCHES = Object.freeze({
  bridged: MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.bridged.branch,
  session: MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.session.branch,
  bookmarks: MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.bookmarks.branch,
  sharedFish: MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.sharedFish.branch,
  actions: MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.actions.branch,
});

// Transitional branch retained while page-owned UI state is migrated out of `_map_controls`.
export const DEFAULT_MAP_CONTROL_SIGNAL_STATE = Object.freeze({
  filters: Object.freeze({
    searchText: "",
    patchId: null,
  }),
  ui: Object.freeze({
    legendOpen: false,
    leftPanelOpen: true,
  }),
});

export const DEFAULT_MAP_BRIDGED_SIGNAL_STATE = Object.freeze({
  filters: Object.freeze({
    fishIds: [],
    zoneRgbs: [],
    semanticFieldIdsByLayer: {},
    fishFilterTerms: [],
    searchExpression: EMPTY_SEARCH_EXPRESSION,
    patchId: null,
    fromPatchId: null,
    toPatchId: null,
    layerIdsVisible: DEFAULT_ENABLED_LAYER_IDS,
    layerIdsOrdered: [],
    layerFilterBindingIdsDisabledByLayer: {},
    layerOpacities: {},
    layerClipMasks: Object.freeze({
      fish_evidence: "zone_mask",
    }),
    layerWaypointConnectionsVisible: {},
    layerWaypointLabelsVisible: {},
    layerPointIconsVisible: {},
    layerPointIconScales: {},
  }),
  ui: Object.freeze({
    diagnosticsOpen: false,
    showPoints: true,
    showPointIcons: true,
    viewMode: null,
    pointIconScale: FISHYMAP_POINT_ICON_SCALE_DEFAULT,
    bookmarkSelectedIds: [],
    bookmarks: [],
  }),
});

export const DEFAULT_MAP_BOOKMARKS_SIGNAL_STATE = Object.freeze({
  entries: [],
});

export const DEFAULT_MAP_SESSION_SIGNAL_STATE = Object.freeze({
  view: Object.freeze({
    viewMode: "2d",
    camera: {},
  }),
  selection: Object.freeze({}),
});

export const DEFAULT_MAP_ACTION_SIGNAL_STATE = Object.freeze({
  resetViewToken: 0,
  resetUiToken: 0,
  saveMapPresetToken: 0,
  discardMapPresetToken: 0,
  focusWorldPointToken: 0,
  focusWorldPoint: null,
});

function cloneJsonValue(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function hasOwnKey(object, key) {
  return Object.prototype.hasOwnProperty.call(object, key);
}

function mergeDefaults(defaults, raw) {
  if (!isPlainObject(defaults)) {
    return raw === undefined ? defaults : raw;
  }
  const source = isPlainObject(raw) ? raw : {};
  const result = {};
  for (const [key, defaultValue] of Object.entries(defaults)) {
    if (!hasOwnKey(source, key)) {
      result[key] = cloneJsonValue(defaultValue);
      continue;
    }
    const rawValue = source[key];
    if (isPlainObject(defaultValue) && isPlainObject(rawValue)) {
      result[key] = mergeDefaults(defaultValue, rawValue);
      continue;
    }
    result[key] = cloneJsonValue(rawValue);
  }
  for (const [key, rawValue] of Object.entries(source)) {
    if (hasOwnKey(result, key)) {
      continue;
    }
    result[key] = cloneJsonValue(rawValue);
  }
  return result;
}

function normalizeBridgeBookmarkEntries(bookmarks) {
  if (!Array.isArray(bookmarks)) {
    return [];
  }
  return bookmarks.flatMap((bookmark) => {
    if (!isPlainObject(bookmark)) {
      return [];
    }
    const id = String(bookmark.id ?? "").trim();
    const worldX = Number(bookmark.worldX);
    const worldZ = Number(bookmark.worldZ);
    if (!id || !Number.isFinite(worldX) || !Number.isFinite(worldZ)) {
      return [];
    }
    const normalized = { id, worldX, worldZ };
    if (typeof bookmark.label === "string" && bookmark.label.trim()) {
      normalized.label = bookmark.label.trim();
    }
    return [normalized];
  });
}

export function normalizeZoneInfoTab(value) {
  return String(value || "").trim();
}

export function normalizeWindowCoordinate(value) {
  if (value == null || value === "") {
    return null;
  }
  const number = Number(value);
  return Number.isFinite(number) ? Math.round(number) : null;
}

export function normalizeNullableString(value) {
  if (value == null) {
    return null;
  }
  const normalized = String(value).trim();
  return normalized || null;
}

export function normalizeExpandedLayerIds(values) {
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

export function normalizeLayerStringListMap(values) {
  if (!isPlainObject(values)) {
    return {};
  }
  const next = {};
  for (const [layerIdRaw, bindingIdsRaw] of Object.entries(values)) {
    const layerId = String(layerIdRaw ?? "").trim();
    if (!layerId) {
      continue;
    }
    const bindingIds = normalizeExpandedLayerIds(bindingIdsRaw);
    if (!bindingIds.length) {
      continue;
    }
    next[layerId] = bindingIds;
  }
  return next;
}

function normalizeWindowUiEntry(rawEntry, fallbackEntry) {
  const baseEntry = isPlainObject(rawEntry) ? rawEntry : {};
  return {
    open: hasOwnKey(baseEntry, "open") ? baseEntry.open !== false : fallbackEntry.open !== false,
    collapsed: hasOwnKey(baseEntry, "collapsed")
      ? Boolean(baseEntry.collapsed)
      : Boolean(fallbackEntry.collapsed),
    x: hasOwnKey(baseEntry, "x") ? normalizeWindowCoordinate(baseEntry.x) : fallbackEntry.x,
    y: hasOwnKey(baseEntry, "y") ? normalizeWindowCoordinate(baseEntry.y) : fallbackEntry.y,
  };
}

function normalizeZoneInfoWindowUiEntry(rawEntry, fallbackEntry) {
  const baseEntry = isPlainObject(rawEntry) ? rawEntry : {};
  return {
    ...normalizeWindowUiEntry(baseEntry, fallbackEntry),
    tab: hasOwnKey(baseEntry, "tab")
      ? normalizeZoneInfoTab(baseEntry.tab)
      : normalizeZoneInfoTab(fallbackEntry?.tab),
  };
}

function normalizeSettingsWindowUiEntry(rawEntry, fallbackEntry) {
  const baseEntry = isPlainObject(rawEntry) ? rawEntry : {};
  return {
    ...normalizeWindowUiEntry(baseEntry, fallbackEntry),
    autoAdjustView: hasOwnKey(baseEntry, "autoAdjustView")
      ? baseEntry.autoAdjustView !== false
      : fallbackEntry?.autoAdjustView !== false,
    normalizeRates: hasOwnKey(baseEntry, "normalizeRates")
      ? baseEntry.normalizeRates !== false
      : fallbackEntry?.normalizeRates !== false,
  };
}

export function normalizeWindowUiState(rawState) {
  const source = isPlainObject(rawState) ? rawState : {};
  return {
    search: {
      ...normalizeWindowUiEntry(source.search, DEFAULT_WINDOW_UI_STATE.search),
      collapsed: false,
    },
    settings: normalizeSettingsWindowUiEntry(source.settings, DEFAULT_WINDOW_UI_STATE.settings),
    zoneInfo: normalizeZoneInfoWindowUiEntry(source.zoneInfo, DEFAULT_WINDOW_UI_STATE.zoneInfo),
    layers: normalizeWindowUiEntry(source.layers, DEFAULT_WINDOW_UI_STATE.layers),
    bookmarks: normalizeWindowUiEntry(source.bookmarks, DEFAULT_WINDOW_UI_STATE.bookmarks),
  };
}

export function normalizeMapUiSignalState(raw) {
  const current = mergeDefaults(DEFAULT_MAP_UI_SIGNAL_STATE, raw);
  const rawSearch = isPlainObject(raw?.search) ? raw.search : {};
  const normalizedBookmarks = current?.bookmarks && typeof current.bookmarks === "object"
    ? current.bookmarks
    : {};
  const normalizedLayers = current?.layers && typeof current.layers === "object"
    ? current.layers
    : {};
  const searchExpression = resolveSearchExpression(
    hasOwnKey(rawSearch, "expression") ? rawSearch.expression : undefined,
    current?.search?.selectedTerms,
  );
  return {
    windowUi: normalizeWindowUiState(current?.windowUi),
    search: {
      open: current?.search?.open === true,
      query: String(current?.search?.query || ""),
      expression: searchExpression,
      selectedTerms: resolveSelectedSearchTerms(current?.search?.selectedTerms, searchExpression),
    },
    bookmarks: {
      placing: normalizedBookmarks.placing === true,
      selectedIds: Array.isArray(normalizedBookmarks.selectedIds)
        ? normalizedBookmarks.selectedIds
            .map((value) => String(value ?? "").trim())
            .filter(Boolean)
        : [],
    },
    layers: {
      expandedLayerIds: normalizeExpandedLayerIds(normalizedLayers.expandedLayerIds),
      hoverFactsVisibleByLayer: isPlainObject(normalizedLayers.hoverFactsVisibleByLayer)
        ? cloneJsonValue(normalizedLayers.hoverFactsVisibleByLayer)
        : {},
    },
  };
}

export function normalizeMapControlSignalState(raw) {
  const current = mergeDefaults(DEFAULT_MAP_CONTROL_SIGNAL_STATE, raw);
  return {
    filters: {
      fishIds: cloneJsonValue(current.filters?.fishIds || []),
      zoneRgbs: cloneJsonValue(current.filters?.zoneRgbs || []),
      semanticFieldIdsByLayer: cloneJsonValue(current.filters?.semanticFieldIdsByLayer || {}),
      fishFilterTerms: cloneJsonValue(current.filters?.fishFilterTerms || []),
      searchText: String(current.filters?.searchText || ""),
      patchId: normalizeNullableString(current.filters?.patchId),
    },
    ui: {
      legendOpen: current.ui?.legendOpen === true,
      leftPanelOpen: current.ui?.leftPanelOpen !== false,
    },
  };
}

export function normalizeMapBridgedSignalState(raw) {
  const current = mergeDefaults(DEFAULT_MAP_BRIDGED_SIGNAL_STATE, raw);
  const rawFilters = isPlainObject(raw?.filters) ? raw.filters : null;
  const fromPatchId = normalizeNullableString(current.filters?.fromPatchId);
  const toPatchId = normalizeNullableString(current.filters?.toPatchId);
  const pointIconScale = Number(current.ui?.pointIconScale);
  return {
    version: FISHYMAP_CONTRACT_VERSION,
    filters: {
      fishIds: cloneJsonValue(current.filters?.fishIds || []),
      zoneRgbs: cloneJsonValue(current.filters?.zoneRgbs || []),
      semanticFieldIdsByLayer: cloneJsonValue(current.filters?.semanticFieldIdsByLayer || {}),
      fishFilterTerms: cloneJsonValue(current.filters?.fishFilterTerms || []),
      searchExpression: resolveSearchExpression(
        rawFilters && hasOwnKey(rawFilters, "searchExpression")
          ? rawFilters.searchExpression
          : undefined,
      ),
      patchId: fromPatchId || toPatchId ? null : normalizeNullableString(current.filters?.patchId),
      fromPatchId,
      toPatchId,
      layerIdsVisible: normalizeExpandedLayerIds(
        current.filters?.layerIdsVisible || DEFAULT_ENABLED_LAYER_IDS,
      ),
      layerIdsOrdered: normalizeExpandedLayerIds(current.filters?.layerIdsOrdered || []),
      layerFilterBindingIdsDisabledByLayer: normalizeLayerStringListMap(
        current.filters?.layerFilterBindingIdsDisabledByLayer || {},
      ),
      layerOpacities: cloneJsonValue(current.filters?.layerOpacities || {}),
      layerClipMasks:
        rawFilters && hasOwnKey(rawFilters, "layerClipMasks")
          ? cloneJsonValue(rawFilters.layerClipMasks || {})
          : cloneJsonValue(current.filters?.layerClipMasks || {}),
      layerWaypointConnectionsVisible: cloneJsonValue(
        current.filters?.layerWaypointConnectionsVisible || {},
      ),
      layerWaypointLabelsVisible: cloneJsonValue(
        current.filters?.layerWaypointLabelsVisible || {},
      ),
      layerPointIconsVisible: cloneJsonValue(current.filters?.layerPointIconsVisible || {}),
      layerPointIconScales: cloneJsonValue(current.filters?.layerPointIconScales || {}),
    },
    ui: {
      diagnosticsOpen: current.ui?.diagnosticsOpen === true,
      showPoints: current.ui?.showPoints !== false,
      showPointIcons: current.ui?.showPointIcons !== false,
      viewMode: current.ui?.viewMode === "3d" ? "3d" : normalizeNullableString(current.ui?.viewMode),
      pointIconScale: Number.isFinite(pointIconScale)
        ? pointIconScale
        : FISHYMAP_POINT_ICON_SCALE_DEFAULT,
      bookmarkSelectedIds: normalizeExpandedLayerIds(current.ui?.bookmarkSelectedIds || []),
      bookmarks: normalizeBridgeBookmarkEntries(current.ui?.bookmarks || []),
    },
  };
}
