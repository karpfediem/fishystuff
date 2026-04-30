import {
  EMPTY_SEARCH_EXPRESSION,
  resolveSearchExpression,
  resolveSelectedSearchTerms,
} from "./map-search-contract.js";
import { FISHYMAP_POINT_ICON_SCALE_DEFAULT } from "./map-host.js";

export const DEFAULT_ENABLED_LAYER_IDS = Object.freeze([
  "bookmarks",
  "fish_evidence",
  "zone_mask",
  "minimap",
]);

export const MAP_UI_STORAGE_KEY = "fishystuff.map.window_ui.v1";
export const MAP_BOOKMARKS_STORAGE_KEY = "fishystuff.map.bookmarks.v1";
export const MAP_SESSION_STORAGE_KEY = "fishystuff.map.session.v1";
export const SHARED_FISH_STORAGE_KEYS = Object.freeze({
  caught: "fishystuff.fishydex.caught.v1",
  favourites: "fishystuff.fishydex.favourites.v1",
});

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function hasOwnKey(object, key) {
  return Object.prototype.hasOwnProperty.call(object, key);
}

function stripEmptyRestorePatchBranches(patch) {
  if (!isPlainObject(patch)) {
    return null;
  }
  if (isPlainObject(patch._map_bridged?.ui) && !Object.keys(patch._map_bridged.ui).length) {
    delete patch._map_bridged.ui;
  }
  if (
    isPlainObject(patch._map_bridged?.filters) &&
    !Object.keys(patch._map_bridged.filters).length
  ) {
    delete patch._map_bridged.filters;
  }
  if (isPlainObject(patch._map_bridged) && !Object.keys(patch._map_bridged).length) {
    delete patch._map_bridged;
  }
  if (isPlainObject(patch._map_ui?.windowUi) && !Object.keys(patch._map_ui.windowUi).length) {
    delete patch._map_ui.windowUi;
  }
  if (isPlainObject(patch._map_ui?.layers) && !Object.keys(patch._map_ui.layers).length) {
    delete patch._map_ui.layers;
  }
  if (isPlainObject(patch._map_ui?.search) && !Object.keys(patch._map_ui.search).length) {
    delete patch._map_ui.search;
  }
  if (isPlainObject(patch._map_ui) && !Object.keys(patch._map_ui).length) {
    delete patch._map_ui;
  }
  if (Array.isArray(patch._map_bookmarks?.entries) && !patch._map_bookmarks.entries.length) {
    delete patch._map_bookmarks;
  }
  if (isPlainObject(patch._map_session) && !Object.keys(patch._map_session).length) {
    delete patch._map_session;
  }
  return Object.keys(patch).length ? patch : null;
}

function normalizeLayerBooleanRecord(value) {
  if (!isPlainObject(value)) {
    return {};
  }
  const normalized = {};
  for (const [key, enabled] of Object.entries(value)) {
    const normalizedKey = String(key || "").trim();
    if (!normalizedKey) {
      continue;
    }
    if (typeof enabled === "boolean") {
      normalized[normalizedKey] = enabled;
      continue;
    }
    if (isPlainObject(enabled)) {
      const nested = normalizeLayerBooleanRecord(enabled);
      if (Object.keys(nested).length) {
        normalized[normalizedKey] = nested;
      }
    }
  }
  return normalized;
}

function hasMapPresetCamera(view) {
  if (!isPlainObject(view)) {
    return false;
  }
  const camera = isPlainObject(view.camera) ? view.camera : {};
  return Object.keys(camera).length > 0;
}

function mapPresetViewMode(value) {
  return value === "3d" ? "3d" : "2d";
}

function normalizeMapPresetView(view) {
  const source = isPlainObject(view) ? view : {};
  const camera = {};
  const sourceCamera = isPlainObject(source.camera) ? source.camera : {};
  for (const key of [
    "centerWorldX",
    "centerWorldZ",
    "zoom",
    "pivotWorldX",
    "pivotWorldY",
    "pivotWorldZ",
    "yaw",
    "pitch",
    "distance",
  ]) {
    const rawValue = sourceCamera[key];
    if (rawValue == null || rawValue === "") {
      continue;
    }
    const number = Number(rawValue);
    if (Number.isFinite(number)) {
      camera[key] = number;
    }
  }
  return {
    viewMode: mapPresetViewMode(source.viewMode),
    camera,
  };
}

function stripQueryOwnedRestoreFields(patch, locationHref) {
  if (!isPlainObject(patch) || !locationHref) {
    return patch;
  }
  let params;
  try {
    params = new URL(locationHref, "https://fishystuff.fish").searchParams;
  } catch (_error) {
    return patch;
  }
  const nextPatch = cloneJson(patch);
  const search = nextPatch._map_ui?.search;
  const bridgedUi = nextPatch._map_bridged?.ui;
  const bridgedFilters = nextPatch._map_bridged?.filters;

  if (bridgedUi && params.has("diagnostics")) {
    delete bridgedUi.diagnosticsOpen;
  }
  if (search && params.has("search")) {
    delete search.query;
  }
  if (
    search &&
    (params.has("focusFish") ||
      params.has("fish") ||
      params.has("fishTerms") ||
      params.has("fishFilterTerms") ||
      params.has("patch") ||
      params.has("fromPatch") ||
      params.has("patchFrom") ||
      params.has("toPatch") ||
      params.has("untilPatch") ||
      params.has("patchTo"))
  ) {
    delete search.expression;
    delete search.selectedTerms;
  }
  if (bridgedFilters) {
    if (params.has("focusFish") || params.has("fish")) {
      delete bridgedFilters.fishIds;
    }
    if (params.has("fishTerms") || params.has("fishFilterTerms")) {
      delete bridgedFilters.fishFilterTerms;
    }
    if (
      params.has("focusFish") ||
      params.has("fish") ||
      params.has("fishTerms") ||
      params.has("fishFilterTerms") ||
      params.has("patch") ||
      params.has("fromPatch") ||
      params.has("patchFrom") ||
      params.has("toPatch") ||
      params.has("untilPatch") ||
      params.has("patchTo")
    ) {
      delete bridgedFilters.searchExpression;
    }
    if (params.has("layers") || params.has("layerSet")) {
      delete bridgedFilters.layerIdsVisible;
    }
  }
  if (
    bridgedFilters &&
    (params.has("patch") ||
      params.has("fromPatch") ||
      params.has("patchFrom") ||
      params.has("toPatch") ||
      params.has("untilPatch") ||
      params.has("patchTo"))
  ) {
    delete bridgedFilters.patchId;
    delete bridgedFilters.fromPatchId;
    delete bridgedFilters.toPatchId;
  }

  return stripEmptyRestorePatchBranches(nextPatch);
}

function storedUiSignals(signals) {
  const windowUi = signals?._map_ui?.windowUi;
  const search = signals?._map_ui?.search;
  const bridgedUi = signals?._map_bridged?.ui;
  const bridgedFilters = signals?._map_bridged?.filters;
  const searchExpression = resolveSearchExpression(
    search?.expression,
    search?.selectedTerms,
  );
  const selectedTerms = resolveSelectedSearchTerms(
    search?.selectedTerms,
    searchExpression,
  );
  const bookmarkEntries = Array.isArray(signals?._map_bookmarks?.entries)
    ? cloneJson(signals._map_bookmarks.entries)
    : [];
  return {
    _map_ui: {
      windowUi: isPlainObject(windowUi) ? cloneJson(windowUi) : {},
      layers: {
        expandedLayerIds: Array.isArray(signals?._map_ui?.layers?.expandedLayerIds)
          ? cloneJson(signals._map_ui.layers.expandedLayerIds)
          : [],
        hoverFactsVisibleByLayer: isPlainObject(signals?._map_ui?.layers?.hoverFactsVisibleByLayer)
          ? cloneJson(signals._map_ui.layers.hoverFactsVisibleByLayer)
          : {},
        sampleHoverVisibleByLayer: isPlainObject(signals?._map_ui?.layers?.sampleHoverVisibleByLayer)
          ? cloneJson(signals._map_ui.layers.sampleHoverVisibleByLayer)
          : {},
      },
      search: {
        query: String(search?.query || ""),
        expression: cloneJson(searchExpression),
        selectedTerms: cloneJson(selectedTerms),
      },
    },
    _map_bookmarks: {
      entries: bookmarkEntries,
    },
    _map_session: isPlainObject(signals?._map_session)
      ? cloneJson(signals._map_session)
      : {
          view: { viewMode: "2d", camera: {} },
          selection: {},
        },
    _map_bridged: {
      ui: {
        diagnosticsOpen: bridgedUi?.diagnosticsOpen === true,
        showPoints: bridgedUi?.showPoints !== false,
        showPointIcons: bridgedUi?.showPointIcons !== false,
        viewMode: bridgedUi?.viewMode === "3d" ? "3d" : "2d",
        pointIconScale: Number.isFinite(bridgedUi?.pointIconScale)
          ? Number(bridgedUi.pointIconScale)
          : FISHYMAP_POINT_ICON_SCALE_DEFAULT,
      },
      filters: {
        layerIdsVisible: Array.isArray(bridgedFilters?.layerIdsVisible)
          ? cloneJson(bridgedFilters.layerIdsVisible)
          : cloneJson(DEFAULT_ENABLED_LAYER_IDS),
        layerIdsOrdered: Array.isArray(bridgedFilters?.layerIdsOrdered)
          ? cloneJson(bridgedFilters.layerIdsOrdered)
          : [],
        layerFilterBindingIdsDisabledByLayer: isPlainObject(
          bridgedFilters?.layerFilterBindingIdsDisabledByLayer,
        )
          ? cloneJson(bridgedFilters.layerFilterBindingIdsDisabledByLayer)
          : {},
        layerOpacities: isPlainObject(bridgedFilters?.layerOpacities)
          ? cloneJson(bridgedFilters.layerOpacities)
          : {},
        layerClipMasks: isPlainObject(bridgedFilters?.layerClipMasks)
          ? cloneJson(bridgedFilters.layerClipMasks)
          : {},
        layerWaypointConnectionsVisible: isPlainObject(
          bridgedFilters?.layerWaypointConnectionsVisible,
        )
          ? cloneJson(bridgedFilters.layerWaypointConnectionsVisible)
          : {},
        layerWaypointLabelsVisible: isPlainObject(bridgedFilters?.layerWaypointLabelsVisible)
          ? cloneJson(bridgedFilters.layerWaypointLabelsVisible)
          : {},
        layerPointIconsVisible: isPlainObject(bridgedFilters?.layerPointIconsVisible)
          ? cloneJson(bridgedFilters.layerPointIconsVisible)
          : {},
        layerPointIconScales: isPlainObject(bridgedFilters?.layerPointIconScales)
          ? cloneJson(bridgedFilters.layerPointIconScales)
          : {},
      },
    },
  };
}

function uiStorageSnapshot(stored) {
  const bridgedUi = isPlainObject(stored?._map_bridged?.ui) ? stored._map_bridged.ui : {};
  const bridgedFilters = isPlainObject(stored?._map_bridged?.filters)
    ? stored._map_bridged.filters
    : {};
  const searchExpression = resolveSearchExpression(
    stored?._map_ui?.search?.expression,
    stored?._map_ui?.search?.selectedTerms,
  );
  const selectedTerms = resolveSelectedSearchTerms(
    stored?._map_ui?.search?.selectedTerms,
    searchExpression,
  );
  return {
    windowUi: isPlainObject(stored?._map_ui?.windowUi) ? cloneJson(stored._map_ui.windowUi) : {},
    layers: {
      expandedLayerIds: Array.isArray(stored?._map_ui?.layers?.expandedLayerIds)
        ? cloneJson(stored._map_ui.layers.expandedLayerIds)
        : [],
      hoverFactsVisibleByLayer: normalizeLayerBooleanRecord(
        stored?._map_ui?.layers?.hoverFactsVisibleByLayer,
      ),
      sampleHoverVisibleByLayer: normalizeLayerBooleanRecord(
        stored?._map_ui?.layers?.sampleHoverVisibleByLayer,
      ),
    },
    search: {
      query: String(stored?._map_ui?.search?.query || ""),
      expression: cloneJson(searchExpression),
      selectedTerms: cloneJson(selectedTerms),
    },
    bridgedUi: {
      diagnosticsOpen: bridgedUi.diagnosticsOpen === true,
      showPoints: bridgedUi.showPoints !== false,
      showPointIcons: bridgedUi.showPointIcons !== false,
      viewMode: bridgedUi.viewMode === "3d" ? "3d" : "2d",
      pointIconScale: Number.isFinite(bridgedUi.pointIconScale)
        ? Number(bridgedUi.pointIconScale)
        : FISHYMAP_POINT_ICON_SCALE_DEFAULT,
    },
    bridgedFilters: {
      layerIdsVisible: Array.isArray(bridgedFilters.layerIdsVisible)
        ? cloneJson(bridgedFilters.layerIdsVisible)
        : cloneJson(DEFAULT_ENABLED_LAYER_IDS),
      layerIdsOrdered: Array.isArray(bridgedFilters.layerIdsOrdered)
        ? cloneJson(bridgedFilters.layerIdsOrdered)
        : [],
      layerFilterBindingIdsDisabledByLayer: isPlainObject(
        bridgedFilters.layerFilterBindingIdsDisabledByLayer,
      )
        ? cloneJson(bridgedFilters.layerFilterBindingIdsDisabledByLayer)
        : {},
      layerOpacities: isPlainObject(bridgedFilters.layerOpacities)
        ? cloneJson(bridgedFilters.layerOpacities)
        : {},
      layerClipMasks: isPlainObject(bridgedFilters.layerClipMasks)
        ? cloneJson(bridgedFilters.layerClipMasks)
        : {},
      layerWaypointConnectionsVisible: isPlainObject(bridgedFilters.layerWaypointConnectionsVisible)
        ? cloneJson(bridgedFilters.layerWaypointConnectionsVisible)
        : {},
      layerWaypointLabelsVisible: isPlainObject(bridgedFilters.layerWaypointLabelsVisible)
        ? cloneJson(bridgedFilters.layerWaypointLabelsVisible)
        : {},
      layerPointIconsVisible: isPlainObject(bridgedFilters.layerPointIconsVisible)
        ? cloneJson(bridgedFilters.layerPointIconsVisible)
        : {},
      layerPointIconScales: isPlainObject(bridgedFilters.layerPointIconScales)
        ? cloneJson(bridgedFilters.layerPointIconScales)
        : {},
    },
  };
}

function restoreUiPatch(parsed) {
  if (!isPlainObject(parsed)) {
    return null;
  }
  const patch = {};
  if (isPlainObject(parsed.windowUi)) {
    patch._map_ui = { windowUi: cloneJson(parsed.windowUi) };
  }
  if (isPlainObject(parsed.layers)) {
    patch._map_ui = patch._map_ui || {};
    patch._map_ui.layers = {
      expandedLayerIds: Array.isArray(parsed.layers.expandedLayerIds)
        ? cloneJson(parsed.layers.expandedLayerIds)
        : [],
      hoverFactsVisibleByLayer: normalizeLayerBooleanRecord(parsed.layers.hoverFactsVisibleByLayer),
      sampleHoverVisibleByLayer: normalizeLayerBooleanRecord(parsed.layers.sampleHoverVisibleByLayer),
    };
  }
  const search = isPlainObject(parsed.search)
    ? parsed.search
    : {};
  const bridgedUi = isPlainObject(parsed.bridgedUi) ? parsed.bridgedUi : null;
  const bridgedFilters = isPlainObject(parsed.bridgedFilters) ? parsed.bridgedFilters : null;

  const hasStoredSearchSelection =
    Object.prototype.hasOwnProperty.call(search, "expression") ||
    Array.isArray(search.selectedTerms);
  const searchExpression = hasStoredSearchSelection
    ? resolveSearchExpression(search.expression, search.selectedTerms)
    : null;
  const selectedTerms = searchExpression
    ? resolveSelectedSearchTerms(search.selectedTerms, searchExpression)
    : [];

  if (Object.keys(search).length || searchExpression) {
    patch._map_ui = patch._map_ui || {};
    patch._map_ui.search = {
      query: String(search.query || ""),
      ...(searchExpression
        ? {
            expression: cloneJson(searchExpression),
            selectedTerms: cloneJson(selectedTerms),
          }
        : {}),
    };
  }
  if (bridgedUi) {
    patch._map_bridged = patch._map_bridged || {};
    patch._map_bridged.ui = {
      diagnosticsOpen: bridgedUi.diagnosticsOpen === true,
      showPoints: bridgedUi.showPoints !== false,
      showPointIcons: bridgedUi.showPointIcons !== false,
      viewMode: bridgedUi.viewMode === "3d" ? "3d" : "2d",
      pointIconScale: Number.isFinite(bridgedUi.pointIconScale)
        ? Number(bridgedUi.pointIconScale)
        : FISHYMAP_POINT_ICON_SCALE_DEFAULT,
    };
  }
  const hasPersistedLayerFilterState =
    bridgedFilters
    && [
      "layerIdsVisible",
      "layerIdsOrdered",
      "layerFilterBindingIdsDisabledByLayer",
      "layerOpacities",
      "layerClipMasks",
      "layerWaypointConnectionsVisible",
      "layerWaypointLabelsVisible",
      "layerPointIconsVisible",
      "layerPointIconScales",
    ].some((key) => hasOwnKey(bridgedFilters, key));

  if (hasPersistedLayerFilterState) {
    patch._map_bridged = patch._map_bridged || {};
    patch._map_bridged.filters = {
      layerIdsVisible: Array.isArray(bridgedFilters.layerIdsVisible)
        ? cloneJson(bridgedFilters.layerIdsVisible)
        : cloneJson(DEFAULT_ENABLED_LAYER_IDS),
      layerIdsOrdered: Array.isArray(bridgedFilters.layerIdsOrdered)
        ? cloneJson(bridgedFilters.layerIdsOrdered)
        : [],
      layerFilterBindingIdsDisabledByLayer: isPlainObject(
        bridgedFilters.layerFilterBindingIdsDisabledByLayer,
      )
        ? cloneJson(bridgedFilters.layerFilterBindingIdsDisabledByLayer)
        : {},
      layerOpacities: isPlainObject(bridgedFilters.layerOpacities)
        ? cloneJson(bridgedFilters.layerOpacities)
        : {},
      layerClipMasks: isPlainObject(bridgedFilters.layerClipMasks)
        ? cloneJson(bridgedFilters.layerClipMasks)
        : {},
      layerWaypointConnectionsVisible: isPlainObject(bridgedFilters.layerWaypointConnectionsVisible)
        ? cloneJson(bridgedFilters.layerWaypointConnectionsVisible)
        : {},
      layerWaypointLabelsVisible: isPlainObject(bridgedFilters.layerWaypointLabelsVisible)
        ? cloneJson(bridgedFilters.layerWaypointLabelsVisible)
        : {},
      layerPointIconsVisible: isPlainObject(bridgedFilters.layerPointIconsVisible)
        ? cloneJson(bridgedFilters.layerPointIconsVisible)
        : {},
      layerPointIconScales: isPlainObject(bridgedFilters.layerPointIconScales)
        ? cloneJson(bridgedFilters.layerPointIconScales)
        : {},
    };
  }
  return Object.keys(patch).length ? patch : null;
}

function sessionStorageSnapshot(stored) {
  return {
    version: 1,
    view: isPlainObject(stored?._map_session?.view)
      ? cloneJson(stored._map_session.view)
      : { viewMode: "2d", camera: {} },
    selection: isPlainObject(stored?._map_session?.selection)
      ? cloneJson(stored._map_session.selection)
      : {},
    filters: {},
  };
}

function restoreSessionPatch(parsed) {
  if (!isPlainObject(parsed)) {
    return null;
  }
  return {
    _map_session: {
      view: isPlainObject(parsed.view) ? cloneJson(parsed.view) : { viewMode: "2d", camera: {} },
      selection: isPlainObject(parsed.selection) ? cloneJson(parsed.selection) : {},
    },
  };
}

function ensureStoredSignals(stored) {
  return stored && typeof stored === "object"
    ? stored
    : {
        _map_ui: {},
        _map_bridged: {},
        _map_bookmarks: { entries: [] },
        _map_session: { view: { viewMode: "2d", camera: {} }, selection: {} },
      };
}

function ensureUiSnapshot(stored) {
  return stored && typeof stored === "object"
    ? stored
    : {
        windowUi: {},
        layers: {
          expandedLayerIds: [],
          hoverFactsVisibleByLayer: {},
          sampleHoverVisibleByLayer: {},
        },
        search: { query: "", expression: cloneJson(EMPTY_SEARCH_EXPRESSION), selectedTerms: [] },
        bridgedUi: {
          diagnosticsOpen: false,
          showPoints: true,
          showPointIcons: true,
          viewMode: "2d",
          pointIconScale: FISHYMAP_POINT_ICON_SCALE_DEFAULT,
        },
        bridgedFilters: {},
      };
}

function ensureSessionSnapshot(stored) {
  return stored && typeof stored === "object"
    ? stored
    : {
        version: 1,
        view: { viewMode: "2d", camera: {} },
        selection: {},
        filters: {},
      };
}

function defaultMapPresetSnapshot() {
  return {
    windowUi: {
      search: { open: true, collapsed: false, x: null, y: null },
      settings: {
        x: null,
        y: null,
        autoAdjustView: true,
        normalizeRates: true,
      },
      zoneInfo: { open: true, collapsed: false, x: null, y: null, tab: "" },
      layers: { open: true, collapsed: false, x: null, y: null },
      bookmarks: { open: false, collapsed: false, x: null, y: null },
    },
    layers: {
      expandedLayerIds: [],
      hoverFactsVisibleByLayer: {},
      sampleHoverVisibleByLayer: {},
    },
    search: {
      query: "",
      expression: cloneJson(EMPTY_SEARCH_EXPRESSION),
      selectedTerms: [],
    },
    bridgedUi: {
      diagnosticsOpen: false,
      showPoints: true,
      showPointIcons: true,
      viewMode: "2d",
      pointIconScale: FISHYMAP_POINT_ICON_SCALE_DEFAULT,
    },
    bridgedFilters: {
      layerIdsVisible: cloneJson(DEFAULT_ENABLED_LAYER_IDS),
      layerIdsOrdered: [],
      layerFilterBindingIdsDisabledByLayer: {},
      layerOpacities: {},
      layerClipMasks: { fish_evidence: "zone_mask" },
      layerWaypointConnectionsVisible: {},
      layerWaypointLabelsVisible: {},
      layerPointIconsVisible: {},
      layerPointIconScales: {},
    },
    view: { viewMode: "2d", camera: {} },
  };
}

function mergeMapPresetBranch(defaultBranch, sourceBranch) {
  const merged = cloneJson(defaultBranch);
  if (!isPlainObject(sourceBranch)) {
    return merged;
  }
  for (const [key, value] of Object.entries(sourceBranch)) {
    if (value === undefined) {
      continue;
    }
    merged[key] = isPlainObject(value) && isPlainObject(merged[key])
      ? { ...merged[key], ...cloneJson(value) }
      : cloneJson(value);
  }
  return merged;
}

function stripMapPresetWindowState(windowUi) {
  const next = isPlainObject(windowUi) ? cloneJson(windowUi) : {};
  if (isPlainObject(next.settings)) {
    delete next.settings.open;
    delete next.settings.collapsed;
  }
  return next;
}

export function normalizeMapPresetPayload(value) {
  const source = isPlainObject(value) ? value : {};
  const defaults = defaultMapPresetSnapshot();
  const sourceWindowUi = isPlainObject(source.windowUi)
    ? source.windowUi
    : source._map_ui?.windowUi;
  const sourceLayers = isPlainObject(source.layers)
    ? source.layers
    : source._map_ui?.layers;
  const sourceSearch = isPlainObject(source.search)
    ? source.search
    : source._map_ui?.search;
  const sourceBridgedUi = isPlainObject(source.bridgedUi)
    ? source.bridgedUi
    : source._map_bridged?.ui;
  const sourceBridgedFilters = isPlainObject(source.bridgedFilters)
    ? source.bridgedFilters
    : source._map_bridged?.filters;
  const windowUi = stripMapPresetWindowState(
    mergeMapPresetBranch(defaults.windowUi, sourceWindowUi),
  );
  const mergedSearch = mergeMapPresetBranch(defaults.search, sourceSearch);
  if (
    isPlainObject(sourceSearch) &&
    sourceSearch.expression === undefined &&
    Array.isArray(sourceSearch.selectedTerms)
  ) {
    delete mergedSearch.expression;
  }
  const uiSnapshot = uiStorageSnapshot({
    _map_ui: {
      windowUi,
      layers: mergeMapPresetBranch(defaults.layers, sourceLayers),
      search: mergedSearch,
    },
    _map_bridged: {
      ui: mergeMapPresetBranch(defaults.bridgedUi, sourceBridgedUi),
      filters: mergeMapPresetBranch(defaults.bridgedFilters, sourceBridgedFilters),
    },
  });
  const sourceView = isPlainObject(source.view)
    ? source.view
    : isPlainObject(source.session?.view)
      ? source.session.view
      : isPlainObject(source._map_session?.view)
        ? source._map_session.view
        : null;
  const sessionSnapshot = sessionStorageSnapshot({
    _map_session: {
      view: sourceView || defaults.view,
    },
  });
  const view = normalizeMapPresetView(sessionSnapshot.view);
  if (!Object.keys(view.camera).length) {
    view.viewMode = mapPresetViewMode(uiSnapshot.bridgedUi.viewMode);
  }
  return {
    version: 1,
    windowUi: cloneJson(uiSnapshot.windowUi),
    layers: cloneJson(uiSnapshot.layers),
    search: cloneJson(uiSnapshot.search),
    bridgedUi: cloneJson(uiSnapshot.bridgedUi),
    bridgedFilters: cloneJson(uiSnapshot.bridgedFilters),
    view,
  };
}

export function defaultMapPresetPayload() {
  return normalizeMapPresetPayload(defaultMapPresetSnapshot());
}

export function createMapPresetPayload(signals, { includeCamera = false } = {}) {
  const stored = ensureStoredSignals(storedUiSignals(signals));
  const uiSnapshot = uiStorageSnapshot(stored);
  const sessionSnapshot = sessionStorageSnapshot(stored);
  const view = cloneJson(sessionSnapshot.view);
  if (!includeCamera) {
    view.camera = {};
    view.viewMode = mapPresetViewMode(uiSnapshot.bridgedUi.viewMode);
  }
  return normalizeMapPresetPayload({
    ...uiSnapshot,
    view,
  });
}

export function mapPresetRestorePatch(payload) {
  const normalized = normalizeMapPresetPayload(payload);
  const patch = restoreUiPatch(normalized) || {};
  if (hasMapPresetCamera(normalized.view)) {
    patch._map_session = {
      ...(isPlainObject(patch._map_session) ? patch._map_session : {}),
      view: cloneJson(normalized.view),
    };
  }
  return patch;
}

function normalizeFallbackFishIds(values) {
  let ids = [];
  if (Array.isArray(values)) {
    ids = values;
  } else if (values && typeof values === "object") {
    ids = Object.entries(values)
      .filter((entry) => entry[1])
      .map((entry) => entry[0]);
  } else {
    return [];
  }
  const next = [];
  const seen = new Set();
  for (const value of ids) {
    const fishId = Number.parseInt(String(value), 10);
    if (!Number.isInteger(fishId) || fishId <= 0 || seen.has(fishId)) {
      continue;
    }
    seen.add(fishId);
    next.push(fishId);
  }
  return next.sort((left, right) => left - right);
}

function buildSharedFishRestorePatch(localStorage) {
  let caughtIds = [];
  let favouriteIds = [];
  try {
    caughtIds = normalizeFallbackFishIds(JSON.parse(
      localStorage?.getItem?.(SHARED_FISH_STORAGE_KEYS.caught) || "[]",
    ));
  } catch (_error) {
    caughtIds = [];
    localStorage?.removeItem?.(SHARED_FISH_STORAGE_KEYS.caught);
  }
  try {
    favouriteIds = normalizeFallbackFishIds(JSON.parse(
      localStorage?.getItem?.(SHARED_FISH_STORAGE_KEYS.favourites) || "[]",
    ));
  } catch (_error) {
    favouriteIds = [];
    localStorage?.removeItem?.(SHARED_FISH_STORAGE_KEYS.favourites);
  }
  return {
    _shared_fish: {
      caughtIds,
      favouriteIds,
    },
  };
}

export function loadSharedFishRestoreState({ localStorage } = {}) {
  return buildSharedFishRestorePatch(localStorage);
}

export function loadRestoreState({
  localStorage,
  sessionStorage,
  locationHref,
} = {}) {
  let uiPatch = null;
  let bookmarkPatch = null;
  let sessionPatch = null;
  try {
    const rawUi = localStorage?.getItem?.(MAP_UI_STORAGE_KEY);
    if (rawUi) {
      try {
        uiPatch = stripQueryOwnedRestoreFields(restoreUiPatch(JSON.parse(rawUi)), locationHref);
      } catch (_error) {
        localStorage?.removeItem?.(MAP_UI_STORAGE_KEY);
      }
    }
    const rawBookmarks = localStorage?.getItem?.(MAP_BOOKMARKS_STORAGE_KEY);
    if (rawBookmarks) {
      try {
        const parsed = JSON.parse(rawBookmarks);
        if (Array.isArray(parsed)) {
          bookmarkPatch = {
            _map_bookmarks: {
              entries: parsed,
            },
          };
        }
      } catch (_error) {
        localStorage?.removeItem?.(MAP_BOOKMARKS_STORAGE_KEY);
      }
    }
    const rawSession = sessionStorage?.getItem?.(MAP_SESSION_STORAGE_KEY);
    if (rawSession) {
      try {
        sessionPatch = restoreSessionPatch(JSON.parse(rawSession));
      } catch (_error) {
        sessionStorage?.removeItem?.(MAP_SESSION_STORAGE_KEY);
      }
    }
  } catch (_error) {
    uiPatch = null;
    bookmarkPatch = null;
    sessionPatch = null;
  }
  return {
    sharedFishPatch: loadSharedFishRestoreState({ localStorage }),
    uiPatch,
    bookmarkPatch,
    sessionPatch,
  };
}

export function createPersistedState(signals) {
  const stored = ensureStoredSignals(storedUiSignals(signals));
  return {
    uiJson: JSON.stringify(ensureUiSnapshot(uiStorageSnapshot(stored))),
    bookmarksJson: JSON.stringify(stored._map_bookmarks.entries),
    sessionJson: JSON.stringify(ensureSessionSnapshot(sessionStorageSnapshot(stored))),
  };
}
