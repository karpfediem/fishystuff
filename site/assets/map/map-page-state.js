(function () {
  const DEFAULT_ENABLED_LAYER_IDS = Object.freeze([
    "bookmarks",
    "fish_evidence",
    "zone_mask",
    "minimap",
  ]);
  const MAP_UI_STORAGE_KEY = "fishystuff.map.window_ui.v1";
  const MAP_BOOKMARKS_STORAGE_KEY = "fishystuff.map.bookmarks.v1";
  const MAP_SESSION_STORAGE_KEY = "fishystuff.map.session.v1";
  const SHARED_FISH_STORAGE_KEYS = Object.freeze({
    caught: "fishystuff.fishydex.caught.v1",
    favourites: "fishystuff.fishydex.favourites.v1",
  });

  function cloneJson(value) {
    return JSON.parse(JSON.stringify(value));
  }

  function isPlainObject(value) {
    return Boolean(value) && typeof value === "object" && !Array.isArray(value);
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
    if (bridgedFilters) {
      if (params.has("focusFish") || params.has("fish")) {
        delete bridgedFilters.fishIds;
      }
      if (params.has("fishTerms") || params.has("fishFilterTerms")) {
        delete bridgedFilters.fishFilterTerms;
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
        },
        search: {
          query: String(search?.query || ""),
          selectedTerms: Array.isArray(search?.selectedTerms)
            ? cloneJson(search.selectedTerms)
            : [],
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
            : 1,
        },
        filters: {
          fishIds: Array.isArray(bridgedFilters?.fishIds) ? cloneJson(bridgedFilters.fishIds) : [],
          zoneRgbs: Array.isArray(bridgedFilters?.zoneRgbs)
            ? cloneJson(bridgedFilters.zoneRgbs)
            : [],
          semanticFieldIdsByLayer: isPlainObject(bridgedFilters?.semanticFieldIdsByLayer)
            ? cloneJson(bridgedFilters.semanticFieldIdsByLayer)
            : {},
          fishFilterTerms: Array.isArray(bridgedFilters?.fishFilterTerms)
            ? cloneJson(bridgedFilters.fishFilterTerms)
            : [],
          patchId: bridgedFilters?.patchId ?? null,
          fromPatchId: bridgedFilters?.fromPatchId ?? null,
          toPatchId: bridgedFilters?.toPatchId ?? null,
          layerIdsVisible: Array.isArray(bridgedFilters?.layerIdsVisible)
            ? cloneJson(bridgedFilters.layerIdsVisible)
            : cloneJson(DEFAULT_ENABLED_LAYER_IDS),
          layerIdsOrdered: Array.isArray(bridgedFilters?.layerIdsOrdered)
            ? cloneJson(bridgedFilters.layerIdsOrdered)
            : [],
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
    return {
      windowUi: isPlainObject(stored?._map_ui?.windowUi) ? cloneJson(stored._map_ui.windowUi) : {},
      layers: {
        expandedLayerIds: Array.isArray(stored?._map_ui?.layers?.expandedLayerIds)
          ? cloneJson(stored._map_ui.layers.expandedLayerIds)
          : [],
        hoverFactsVisibleByLayer: isPlainObject(stored?._map_ui?.layers?.hoverFactsVisibleByLayer)
          ? cloneJson(stored._map_ui.layers.hoverFactsVisibleByLayer)
          : {},
      },
      search: {
        query: String(stored?._map_ui?.search?.query || ""),
        selectedTerms: Array.isArray(stored?._map_ui?.search?.selectedTerms)
          ? cloneJson(stored._map_ui.search.selectedTerms)
          : [],
      },
      bridgedUi: {
        diagnosticsOpen: bridgedUi.diagnosticsOpen === true,
        showPoints: bridgedUi.showPoints !== false,
        showPointIcons: bridgedUi.showPointIcons !== false,
        viewMode: bridgedUi.viewMode === "3d" ? "3d" : "2d",
        pointIconScale: Number.isFinite(bridgedUi.pointIconScale)
          ? Number(bridgedUi.pointIconScale)
          : 1,
      },
      bridgedFilters: {
        fishIds: Array.isArray(bridgedFilters.fishIds)
          ? cloneJson(bridgedFilters.fishIds)
          : [],
        zoneRgbs: Array.isArray(bridgedFilters.zoneRgbs)
          ? cloneJson(bridgedFilters.zoneRgbs)
          : [],
        semanticFieldIdsByLayer: isPlainObject(bridgedFilters.semanticFieldIdsByLayer)
          ? cloneJson(bridgedFilters.semanticFieldIdsByLayer)
          : {},
        fishFilterTerms: Array.isArray(bridgedFilters.fishFilterTerms)
          ? cloneJson(bridgedFilters.fishFilterTerms)
          : [],
        patchId: bridgedFilters.patchId == null ? null : String(bridgedFilters.patchId),
        fromPatchId: bridgedFilters.fromPatchId == null ? null : String(bridgedFilters.fromPatchId),
        toPatchId: bridgedFilters.toPatchId == null ? null : String(bridgedFilters.toPatchId),
        layerIdsVisible: Array.isArray(bridgedFilters.layerIdsVisible)
          ? cloneJson(bridgedFilters.layerIdsVisible)
          : cloneJson(DEFAULT_ENABLED_LAYER_IDS),
        layerIdsOrdered: Array.isArray(bridgedFilters.layerIdsOrdered)
          ? cloneJson(bridgedFilters.layerIdsOrdered)
          : [],
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
        hoverFactsVisibleByLayer: isPlainObject(parsed.layers.hoverFactsVisibleByLayer)
          ? cloneJson(parsed.layers.hoverFactsVisibleByLayer)
          : {},
      };
    }
    const search = isPlainObject(parsed.search)
      ? parsed.search
      : isPlainObject(parsed.inputFilters)
        ? { query: parsed.inputFilters.searchText }
        : {};
    const bridgedUi = isPlainObject(parsed.bridgedUi)
      ? parsed.bridgedUi
      : isPlainObject(parsed.inputUi)
        ? parsed.inputUi
        : null;
    const bridgedFilters = isPlainObject(parsed.bridgedFilters)
      ? parsed.bridgedFilters
      : isPlainObject(parsed.inputFilters)
        ? parsed.inputFilters
        : null;

    if (Object.keys(search).length) {
      patch._map_ui = patch._map_ui || {};
      patch._map_ui.search = {
        query: String(search.query || ""),
        ...(Array.isArray(search.selectedTerms)
          ? { selectedTerms: cloneJson(search.selectedTerms) }
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
          : 1,
      };
    }
    if (bridgedFilters) {
      patch._map_bridged = patch._map_bridged || {};
      patch._map_bridged.filters = {
        fishIds: Array.isArray(bridgedFilters.fishIds) ? cloneJson(bridgedFilters.fishIds) : [],
        zoneRgbs: Array.isArray(bridgedFilters.zoneRgbs) ? cloneJson(bridgedFilters.zoneRgbs) : [],
        semanticFieldIdsByLayer: isPlainObject(bridgedFilters.semanticFieldIdsByLayer)
          ? cloneJson(bridgedFilters.semanticFieldIdsByLayer)
          : {},
        fishFilterTerms: Array.isArray(bridgedFilters.fishFilterTerms)
          ? cloneJson(bridgedFilters.fishFilterTerms)
          : [],
        patchId: bridgedFilters.patchId == null ? null : String(bridgedFilters.patchId),
        fromPatchId: bridgedFilters.fromPatchId == null ? null : String(bridgedFilters.fromPatchId),
        toPatchId: bridgedFilters.toPatchId == null ? null : String(bridgedFilters.toPatchId),
        layerIdsVisible: Array.isArray(bridgedFilters.layerIdsVisible)
          ? cloneJson(bridgedFilters.layerIdsVisible)
          : cloneJson(DEFAULT_ENABLED_LAYER_IDS),
        layerIdsOrdered: Array.isArray(bridgedFilters.layerIdsOrdered)
          ? cloneJson(bridgedFilters.layerIdsOrdered)
          : [],
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
          },
          search: { query: "", selectedTerms: [] },
          bridgedUi: {
            diagnosticsOpen: false,
            showPoints: true,
            showPointIcons: true,
            viewMode: "2d",
            pointIconScale: 1,
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

  function loadRestoreState({
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
      sharedFishPatch: buildSharedFishRestorePatch(localStorage),
      uiPatch,
      bookmarkPatch,
      sessionPatch,
    };
  }

  function createPersistedState(signals) {
    const stored = ensureStoredSignals(storedUiSignals(signals));
    return {
      uiJson: JSON.stringify(ensureUiSnapshot(uiStorageSnapshot(stored))),
      bookmarksJson: JSON.stringify(stored._map_bookmarks.entries),
      sessionJson: JSON.stringify(ensureSessionSnapshot(sessionStorageSnapshot(stored))),
    };
  }

  window.__fishystuffMapPageState = Object.freeze({
    MAP_UI_STORAGE_KEY,
    MAP_BOOKMARKS_STORAGE_KEY,
    MAP_SESSION_STORAGE_KEY,
    createPersistedState,
    loadRestoreState,
  });
})();
