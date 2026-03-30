(function () {
  const MAP_UI_STORAGE_KEY = "fishystuff.map.window_ui.v1";
  const MAP_BOOKMARKS_STORAGE_KEY = "fishystuff.map.bookmarks.v1";
  const MAP_SESSION_STORAGE_KEY = "fishystuff.map.session.v1";
  const LEGACY_MAP_PREFS_STORAGE_KEY = "fishystuff.map.prefs.v1";
  const SHARED_FISH_STORAGE_KEYS = Object.freeze({
    caught: "fishystuff.fishydex.caught.v1",
    favourites: "fishystuff.fishydex.favourites.v1",
  });
  const MAP_PERSIST_SIGNAL_FILTER =
    /^_(?:map_ui\.(?:windowUi|layers(?:\.|$))|map_input\.ui\.(?:diagnosticsOpen|legendOpen|leftPanelOpen|showPoints|showPointIcons|pointIconScale)|map_input\.filters\.(?:fishIds|zoneRgbs|semanticFieldIdsByLayer|fishFilterTerms|searchText|fromPatchId|toPatchId|layerIdsVisible|layerIdsOrdered|layerOpacities|layerClipMasks|layerWaypointConnectionsVisible|layerWaypointLabelsVisible|layerPointIconsVisible|layerPointIconScales)|map_bookmarks\.entries|map_session(?:\.|$))(?:\.|$)/;
  const state = {
    persistedUiJson: "",
    persistedBookmarksJson: "",
    persistedSessionJson: "",
    uiStateRestored: false,
    persistBinding: null,
    restoreResolved: false,
    restorePromise: null,
    resolveRestore: null,
  };
  state.restorePromise = new Promise((resolve) => {
    state.resolveRestore = resolve;
  });

  const signalStore = window.__fishystuffDatastarState.createPageSignalStore();

  function signalObject() {
    return signalStore.signalObject();
  }

  function connect(signals) {
    signalStore.connect(signals);
  }

  function cloneJson(value) {
    return JSON.parse(JSON.stringify(value));
  }

  function currentLocationHref() {
    return globalThis.location?.href || globalThis.window?.location?.href || "";
  }

  function datastarPersistHelper() {
    const helper = window.__fishystuffDatastarPersist;
    return helper && typeof helper.createDebouncedSignalPatchPersistor === "function"
      ? helper
      : null;
  }

  function bindPersistListener() {
    if (state.persistBinding) {
      return;
    }
    const helper = datastarPersistHelper();
    if (!helper) {
      return;
    }
    state.persistBinding = helper.createDebouncedSignalPatchPersistor({
      delayMs: 120,
      isReady() {
        return state.uiStateRestored;
      },
      filter: {
        include: MAP_PERSIST_SIGNAL_FILTER,
      },
      persist,
    });
    state.persistBinding.bind();
  }

  function patchSignals(patch) {
    signalStore.patchSignals(patch);
  }

  function sharedFishStateHelper() {
    const helper = window.__fishystuffSharedFishState;
    return helper && typeof helper.loadState === "function" ? helper : null;
  }

  function normalizeFallbackFishIds(values) {
    if (!Array.isArray(values)) {
      return [];
    }
    const next = [];
    const seen = new Set();
    for (const value of values) {
      const fishId = Number.parseInt(String(value), 10);
      if (!Number.isInteger(fishId) || fishId <= 0 || seen.has(fishId)) {
        continue;
      }
      seen.add(fishId);
      next.push(fishId);
    }
    return next.sort((left, right) => left - right);
  }

  function restoreSharedFishPatch() {
    const helper = sharedFishStateHelper();
    if (helper) {
      const shared = helper.loadState(SHARED_FISH_STORAGE_KEYS, globalThis.localStorage);
      return {
        _shared_fish: {
          caughtIds: cloneJson(shared.caughtIds || []),
          favouriteIds: cloneJson(shared.favouriteIds || []),
        },
      };
    }
    let caughtIds = [];
    let favouriteIds = [];
    try {
      caughtIds = JSON.parse(globalThis.localStorage?.getItem?.(SHARED_FISH_STORAGE_KEYS.caught) || "[]");
    } catch (_error) {
      caughtIds = [];
    }
    try {
      favouriteIds = JSON.parse(globalThis.localStorage?.getItem?.(SHARED_FISH_STORAGE_KEYS.favourites) || "[]");
    } catch (_error) {
      favouriteIds = [];
    }
    return {
      _shared_fish: {
        caughtIds: normalizeFallbackFishIds(caughtIds),
        favouriteIds: normalizeFallbackFishIds(favouriteIds),
      },
    };
  }

  function storedUiSignals(signals) {
    const windowUi = signals?._map_ui?.windowUi;
    const inputUi = signals?._map_input?.ui;
    const inputFilters = signals?._map_input?.filters;
    const bookmarkEntries = Array.isArray(signals?._map_bookmarks?.entries)
      ? cloneJson(signals._map_bookmarks.entries)
      : [];
    // Map-page persistence is intentionally limited to page-owned durable UI state.
    // Ephemeral locals such as `_map_ui.search` and `_map_ui.bookmarks` stay live-only.
    return {
      _map_ui: {
        windowUi:
          windowUi && typeof windowUi === "object" && !Array.isArray(windowUi)
            ? cloneJson(windowUi)
            : {},
        layers: {
          expandedLayerIds: Array.isArray(signals?._map_ui?.layers?.expandedLayerIds)
            ? cloneJson(signals._map_ui.layers.expandedLayerIds)
            : [],
        },
      },
      _map_input: {
        ui: {
          diagnosticsOpen: inputUi?.diagnosticsOpen === true,
          legendOpen: inputUi?.legendOpen === true,
          leftPanelOpen: inputUi?.leftPanelOpen !== false,
          showPoints: inputUi?.showPoints !== false,
          showPointIcons: inputUi?.showPointIcons !== false,
          pointIconScale: Number.isFinite(inputUi?.pointIconScale)
            ? Number(inputUi.pointIconScale)
            : 1,
        },
        filters: {
          fishIds: Array.isArray(inputFilters?.fishIds) ? cloneJson(inputFilters.fishIds) : [],
          zoneRgbs: Array.isArray(inputFilters?.zoneRgbs) ? cloneJson(inputFilters.zoneRgbs) : [],
          semanticFieldIdsByLayer:
            inputFilters?.semanticFieldIdsByLayer &&
            typeof inputFilters.semanticFieldIdsByLayer === "object" &&
            !Array.isArray(inputFilters.semanticFieldIdsByLayer)
              ? cloneJson(inputFilters.semanticFieldIdsByLayer)
              : {},
          fishFilterTerms: Array.isArray(inputFilters?.fishFilterTerms)
            ? cloneJson(inputFilters.fishFilterTerms)
            : [],
          searchText: String(inputFilters?.searchText || ""),
          fromPatchId:
            inputFilters?.fromPatchId == null ? null : String(inputFilters.fromPatchId),
          toPatchId: inputFilters?.toPatchId == null ? null : String(inputFilters.toPatchId),
          layerIdsVisible: Array.isArray(inputFilters?.layerIdsVisible)
            ? cloneJson(inputFilters.layerIdsVisible)
            : [],
          layerIdsOrdered: Array.isArray(inputFilters?.layerIdsOrdered)
            ? cloneJson(inputFilters.layerIdsOrdered)
            : [],
          layerOpacities:
            inputFilters?.layerOpacities &&
            typeof inputFilters.layerOpacities === "object" &&
            !Array.isArray(inputFilters.layerOpacities)
              ? cloneJson(inputFilters.layerOpacities)
              : {},
          layerClipMasks:
            inputFilters?.layerClipMasks &&
            typeof inputFilters.layerClipMasks === "object" &&
            !Array.isArray(inputFilters.layerClipMasks)
              ? cloneJson(inputFilters.layerClipMasks)
              : {},
          layerWaypointConnectionsVisible:
            inputFilters?.layerWaypointConnectionsVisible &&
            typeof inputFilters.layerWaypointConnectionsVisible === "object" &&
            !Array.isArray(inputFilters.layerWaypointConnectionsVisible)
              ? cloneJson(inputFilters.layerWaypointConnectionsVisible)
              : {},
          layerWaypointLabelsVisible:
            inputFilters?.layerWaypointLabelsVisible &&
            typeof inputFilters.layerWaypointLabelsVisible === "object" &&
            !Array.isArray(inputFilters.layerWaypointLabelsVisible)
              ? cloneJson(inputFilters.layerWaypointLabelsVisible)
              : {},
          layerPointIconsVisible:
            inputFilters?.layerPointIconsVisible &&
            typeof inputFilters.layerPointIconsVisible === "object" &&
            !Array.isArray(inputFilters.layerPointIconsVisible)
              ? cloneJson(inputFilters.layerPointIconsVisible)
              : {},
          layerPointIconScales:
            inputFilters?.layerPointIconScales &&
            typeof inputFilters.layerPointIconScales === "object" &&
            !Array.isArray(inputFilters.layerPointIconScales)
              ? cloneJson(inputFilters.layerPointIconScales)
              : {},
        },
      },
      _map_bookmarks: {
        entries: bookmarkEntries,
      },
      _map_session:
        signals?._map_session &&
        typeof signals._map_session === "object" &&
        !Array.isArray(signals._map_session)
          ? cloneJson(signals._map_session)
          : {
              view: { viewMode: "2d", camera: {} },
              selection: {},
            },
    };
  }

  function uiStorageSnapshot(stored) {
    return {
      windowUi:
        stored?._map_ui?.windowUi &&
        typeof stored._map_ui.windowUi === "object" &&
        !Array.isArray(stored._map_ui.windowUi)
          ? cloneJson(stored._map_ui.windowUi)
          : {},
      layers: {
        expandedLayerIds: Array.isArray(stored?._map_ui?.layers?.expandedLayerIds)
          ? cloneJson(stored._map_ui.layers.expandedLayerIds)
          : [],
      },
      inputUi: {
        diagnosticsOpen: stored?._map_input?.ui?.diagnosticsOpen === true,
        legendOpen: stored?._map_input?.ui?.legendOpen === true,
        leftPanelOpen: stored?._map_input?.ui?.leftPanelOpen !== false,
        showPoints: stored?._map_input?.ui?.showPoints !== false,
        showPointIcons: stored?._map_input?.ui?.showPointIcons !== false,
        pointIconScale: Number.isFinite(stored?._map_input?.ui?.pointIconScale)
          ? Number(stored._map_input.ui.pointIconScale)
          : 1,
      },
      inputFilters: {
        fishIds: Array.isArray(stored?._map_input?.filters?.fishIds)
          ? cloneJson(stored._map_input.filters.fishIds)
          : [],
        zoneRgbs: Array.isArray(stored?._map_input?.filters?.zoneRgbs)
          ? cloneJson(stored._map_input.filters.zoneRgbs)
          : [],
        semanticFieldIdsByLayer:
          stored?._map_input?.filters?.semanticFieldIdsByLayer &&
          typeof stored._map_input.filters.semanticFieldIdsByLayer === "object" &&
          !Array.isArray(stored._map_input.filters.semanticFieldIdsByLayer)
            ? cloneJson(stored._map_input.filters.semanticFieldIdsByLayer)
            : {},
        fishFilterTerms: Array.isArray(stored?._map_input?.filters?.fishFilterTerms)
          ? cloneJson(stored._map_input.filters.fishFilterTerms)
          : [],
        searchText: String(stored?._map_input?.filters?.searchText || ""),
        fromPatchId:
          stored?._map_input?.filters?.fromPatchId == null
            ? null
            : String(stored._map_input.filters.fromPatchId),
        toPatchId:
          stored?._map_input?.filters?.toPatchId == null
            ? null
            : String(stored._map_input.filters.toPatchId),
        layerIdsVisible: Array.isArray(stored?._map_input?.filters?.layerIdsVisible)
          ? cloneJson(stored._map_input.filters.layerIdsVisible)
          : [],
        layerIdsOrdered: Array.isArray(stored?._map_input?.filters?.layerIdsOrdered)
          ? cloneJson(stored._map_input.filters.layerIdsOrdered)
          : [],
        layerOpacities:
          stored?._map_input?.filters?.layerOpacities &&
          typeof stored._map_input.filters.layerOpacities === "object" &&
          !Array.isArray(stored._map_input.filters.layerOpacities)
            ? cloneJson(stored._map_input.filters.layerOpacities)
            : {},
        layerClipMasks:
          stored?._map_input?.filters?.layerClipMasks &&
          typeof stored._map_input.filters.layerClipMasks === "object" &&
          !Array.isArray(stored._map_input.filters.layerClipMasks)
            ? cloneJson(stored._map_input.filters.layerClipMasks)
            : {},
        layerWaypointConnectionsVisible:
          stored?._map_input?.filters?.layerWaypointConnectionsVisible &&
          typeof stored._map_input.filters.layerWaypointConnectionsVisible === "object" &&
          !Array.isArray(stored._map_input.filters.layerWaypointConnectionsVisible)
            ? cloneJson(stored._map_input.filters.layerWaypointConnectionsVisible)
            : {},
        layerWaypointLabelsVisible:
          stored?._map_input?.filters?.layerWaypointLabelsVisible &&
          typeof stored._map_input.filters.layerWaypointLabelsVisible === "object" &&
          !Array.isArray(stored._map_input.filters.layerWaypointLabelsVisible)
            ? cloneJson(stored._map_input.filters.layerWaypointLabelsVisible)
            : {},
        layerPointIconsVisible:
          stored?._map_input?.filters?.layerPointIconsVisible &&
          typeof stored._map_input.filters.layerPointIconsVisible === "object" &&
          !Array.isArray(stored._map_input.filters.layerPointIconsVisible)
            ? cloneJson(stored._map_input.filters.layerPointIconsVisible)
            : {},
        layerPointIconScales:
          stored?._map_input?.filters?.layerPointIconScales &&
          typeof stored._map_input.filters.layerPointIconScales === "object" &&
          !Array.isArray(stored._map_input.filters.layerPointIconScales)
            ? cloneJson(stored._map_input.filters.layerPointIconScales)
            : {},
      },
    };
  }

  function restoreUiPatch(parsed) {
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return null;
    }
    const patch = {};
    if (parsed.windowUi && typeof parsed.windowUi === "object" && !Array.isArray(parsed.windowUi)) {
      patch._map_ui = {
        windowUi: cloneJson(parsed.windowUi),
      };
    }
    if (parsed.layers && typeof parsed.layers === "object" && !Array.isArray(parsed.layers)) {
      patch._map_ui = patch._map_ui || {};
      patch._map_ui.layers = {
        expandedLayerIds: Array.isArray(parsed.layers.expandedLayerIds)
          ? cloneJson(parsed.layers.expandedLayerIds)
          : [],
      };
    }
    if (parsed.inputUi && typeof parsed.inputUi === "object" && !Array.isArray(parsed.inputUi)) {
      patch._map_input = {
        ui: {
          diagnosticsOpen: parsed.inputUi.diagnosticsOpen === true,
          legendOpen: parsed.inputUi.legendOpen === true,
          leftPanelOpen: parsed.inputUi.leftPanelOpen !== false,
          showPoints: parsed.inputUi.showPoints !== false,
          showPointIcons: parsed.inputUi.showPointIcons !== false,
          pointIconScale: Number.isFinite(parsed.inputUi.pointIconScale)
            ? Number(parsed.inputUi.pointIconScale)
            : 1,
        },
      };
    }
    if (
      parsed.inputFilters &&
      typeof parsed.inputFilters === "object" &&
      !Array.isArray(parsed.inputFilters)
    ) {
      patch._map_input = patch._map_input || {};
      patch._map_input.filters = {
        fishIds: Array.isArray(parsed.inputFilters.fishIds)
          ? cloneJson(parsed.inputFilters.fishIds)
          : [],
        zoneRgbs: Array.isArray(parsed.inputFilters.zoneRgbs)
          ? cloneJson(parsed.inputFilters.zoneRgbs)
          : [],
        semanticFieldIdsByLayer:
          parsed.inputFilters.semanticFieldIdsByLayer &&
          typeof parsed.inputFilters.semanticFieldIdsByLayer === "object" &&
          !Array.isArray(parsed.inputFilters.semanticFieldIdsByLayer)
            ? cloneJson(parsed.inputFilters.semanticFieldIdsByLayer)
            : {},
        fishFilterTerms: Array.isArray(parsed.inputFilters.fishFilterTerms)
          ? cloneJson(parsed.inputFilters.fishFilterTerms)
          : [],
        searchText: String(parsed.inputFilters.searchText || ""),
        fromPatchId:
          parsed.inputFilters.fromPatchId == null ? null : String(parsed.inputFilters.fromPatchId),
        toPatchId:
          parsed.inputFilters.toPatchId == null ? null : String(parsed.inputFilters.toPatchId),
        layerIdsVisible: Array.isArray(parsed.inputFilters.layerIdsVisible)
          ? cloneJson(parsed.inputFilters.layerIdsVisible)
          : [],
        layerIdsOrdered: Array.isArray(parsed.inputFilters.layerIdsOrdered)
          ? cloneJson(parsed.inputFilters.layerIdsOrdered)
          : [],
        layerOpacities:
          parsed.inputFilters.layerOpacities &&
          typeof parsed.inputFilters.layerOpacities === "object" &&
          !Array.isArray(parsed.inputFilters.layerOpacities)
            ? cloneJson(parsed.inputFilters.layerOpacities)
            : {},
        layerClipMasks:
          parsed.inputFilters.layerClipMasks &&
          typeof parsed.inputFilters.layerClipMasks === "object" &&
          !Array.isArray(parsed.inputFilters.layerClipMasks)
            ? cloneJson(parsed.inputFilters.layerClipMasks)
            : {},
        layerWaypointConnectionsVisible:
          parsed.inputFilters.layerWaypointConnectionsVisible &&
          typeof parsed.inputFilters.layerWaypointConnectionsVisible === "object" &&
          !Array.isArray(parsed.inputFilters.layerWaypointConnectionsVisible)
            ? cloneJson(parsed.inputFilters.layerWaypointConnectionsVisible)
            : {},
        layerWaypointLabelsVisible:
          parsed.inputFilters.layerWaypointLabelsVisible &&
          typeof parsed.inputFilters.layerWaypointLabelsVisible === "object" &&
          !Array.isArray(parsed.inputFilters.layerWaypointLabelsVisible)
            ? cloneJson(parsed.inputFilters.layerWaypointLabelsVisible)
            : {},
        layerPointIconsVisible:
          parsed.inputFilters.layerPointIconsVisible &&
          typeof parsed.inputFilters.layerPointIconsVisible === "object" &&
          !Array.isArray(parsed.inputFilters.layerPointIconsVisible)
            ? cloneJson(parsed.inputFilters.layerPointIconsVisible)
            : {},
        layerPointIconScales:
          parsed.inputFilters.layerPointIconScales &&
          typeof parsed.inputFilters.layerPointIconScales === "object" &&
          !Array.isArray(parsed.inputFilters.layerPointIconScales)
            ? cloneJson(parsed.inputFilters.layerPointIconScales)
            : {},
      };
    }
    return Object.keys(patch).length ? patch : null;
  }

  function sessionStorageSnapshot(stored) {
    return {
      version: 1,
      view:
        stored?._map_session?.view &&
        typeof stored._map_session.view === "object" &&
        !Array.isArray(stored._map_session.view)
          ? cloneJson(stored._map_session.view)
          : { viewMode: "2d", camera: {} },
      selection:
        stored?._map_session?.selection &&
        typeof stored._map_session.selection === "object" &&
        !Array.isArray(stored._map_session.selection)
          ? cloneJson(stored._map_session.selection)
          : {},
      filters: {},
    };
  }

  function restoreSessionPatch(parsed) {
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return null;
    }
    return {
      _map_session: {
        view:
          parsed.view && typeof parsed.view === "object" && !Array.isArray(parsed.view)
            ? cloneJson(parsed.view)
            : { viewMode: "2d", camera: {} },
        selection:
          parsed.selection &&
          typeof parsed.selection === "object" &&
          !Array.isArray(parsed.selection)
            ? cloneJson(parsed.selection)
            : {},
      },
    };
  }

  function stripEmptyRestorePatchBranches(patch) {
    if (!patch || typeof patch !== "object") {
      return null;
    }
    if (patch._map_input?.ui && !Object.keys(patch._map_input.ui).length) {
      delete patch._map_input.ui;
    }
    if (patch._map_input?.filters && !Object.keys(patch._map_input.filters).length) {
      delete patch._map_input.filters;
    }
    if (patch._map_input && !Object.keys(patch._map_input).length) {
      delete patch._map_input;
    }
    if (patch._map_ui?.windowUi && !Object.keys(patch._map_ui.windowUi).length) {
      delete patch._map_ui.windowUi;
    }
    if (patch._map_ui && !Object.keys(patch._map_ui).length) {
      delete patch._map_ui;
    }
    if (patch._map_bookmarks?.entries && !patch._map_bookmarks.entries.length) {
      delete patch._map_bookmarks;
    }
    if (
      patch._map_session &&
      typeof patch._map_session === "object" &&
      !Array.isArray(patch._map_session) &&
      !Object.keys(patch._map_session).length
    ) {
      delete patch._map_session;
    }
    return Object.keys(patch).length ? patch : null;
  }

  function stripQueryOwnedRestoreFields(patch, locationHref = currentLocationHref()) {
    if (!patch || typeof patch !== "object" || !locationHref) {
      return patch;
    }
    let params;
    try {
      params = new URL(locationHref, "https://fishystuff.fish").searchParams;
    } catch (_error) {
      return patch;
    }
    const nextPatch = cloneJson(patch);
    const inputUi = nextPatch._map_input?.ui;
    const inputFilters = nextPatch._map_input?.filters;

    if (inputUi) {
      if (params.has("diagnostics")) {
        delete inputUi.diagnosticsOpen;
      }
      if (params.has("legend")) {
        delete inputUi.legendOpen;
      }
    }

    if (inputFilters) {
      if (params.has("focusFish") || params.has("fish")) {
        delete inputFilters.fishIds;
      }
      if (params.has("fishTerms") || params.has("fishFilterTerms")) {
        delete inputFilters.fishFilterTerms;
      }
      if (params.has("search")) {
        delete inputFilters.searchText;
      }
      if (
        params.has("patch") ||
        params.has("fromPatch") ||
        params.has("patchFrom") ||
        params.has("toPatch") ||
        params.has("untilPatch") ||
        params.has("patchTo")
      ) {
        delete inputFilters.fromPatchId;
        delete inputFilters.toPatchId;
      }
      if (params.has("layers") || params.has("layerSet")) {
        delete inputFilters.layerIdsVisible;
      }
    }

    return stripEmptyRestorePatchBranches(nextPatch);
  }

  function restore(signals) {
    connect(signals);
    bindPersistListener();
    const sharedFishPatch = restoreSharedFishPatch();
    let uiPatch = null;
    let bookmarkPatch = null;
    let sessionPatch = null;
    try {
      globalThis.localStorage?.removeItem?.(LEGACY_MAP_PREFS_STORAGE_KEY);
      const rawUi = globalThis.localStorage?.getItem?.(MAP_UI_STORAGE_KEY);
      if (rawUi) {
        try {
          uiPatch = stripQueryOwnedRestoreFields(restoreUiPatch(JSON.parse(rawUi)));
        } catch (_error) {
          globalThis.localStorage?.removeItem?.(MAP_UI_STORAGE_KEY);
        }
      }
      const rawBookmarks = globalThis.localStorage?.getItem?.(MAP_BOOKMARKS_STORAGE_KEY);
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
          globalThis.localStorage?.removeItem?.(MAP_BOOKMARKS_STORAGE_KEY);
        }
      }
      const rawSession = globalThis.sessionStorage?.getItem?.(MAP_SESSION_STORAGE_KEY);
      if (rawSession) {
        try {
          sessionPatch = restoreSessionPatch(JSON.parse(rawSession));
        } catch (_error) {
          globalThis.sessionStorage?.removeItem?.(MAP_SESSION_STORAGE_KEY);
        }
      }
    } catch (_error) {
      uiPatch = null;
      bookmarkPatch = null;
      sessionPatch = null;
    }
    patchSignals(sharedFishPatch);
    if (uiPatch) {
      patchSignals(uiPatch);
    }
    if (bookmarkPatch) {
      patchSignals(bookmarkPatch);
    }
    if (sessionPatch) {
      patchSignals(sessionPatch);
    }
    const stored = storedUiSignals(signals);
    state.persistedUiJson = JSON.stringify(uiStorageSnapshot(stored));
    state.persistedBookmarksJson = JSON.stringify(stored._map_bookmarks.entries);
    state.persistedSessionJson = JSON.stringify(sessionStorageSnapshot(stored));
    state.uiStateRestored = true;
    if (!state.restoreResolved) {
      state.restoreResolved = true;
      state.resolveRestore?.();
    }
  }

  function persist() {
    const snapshot = signalObject();
    if (!snapshot || !state.uiStateRestored) {
      return;
    }
    try {
      const stored = storedUiSignals(snapshot);
      const uiJson = JSON.stringify(uiStorageSnapshot(stored));
      const bookmarksJson = JSON.stringify(stored._map_bookmarks.entries);
      if (uiJson !== state.persistedUiJson) {
        globalThis.localStorage?.setItem?.(MAP_UI_STORAGE_KEY, uiJson);
        state.persistedUiJson = uiJson;
      }
      if (bookmarksJson !== state.persistedBookmarksJson) {
        globalThis.localStorage?.setItem?.(MAP_BOOKMARKS_STORAGE_KEY, bookmarksJson);
        state.persistedBookmarksJson = bookmarksJson;
      }
      const sessionJson = JSON.stringify(sessionStorageSnapshot(stored));
      if (sessionJson !== state.persistedSessionJson) {
        globalThis.sessionStorage?.setItem?.(MAP_SESSION_STORAGE_KEY, sessionJson);
        state.persistedSessionJson = sessionJson;
      }
    } catch (_error) {
      // Map UI persistence is best-effort only.
    }
  }
  window.__fishystuffMap = Object.freeze({
    connect,
    signalObject,
    patchSignals,
    readSignal(path) {
      return signalStore.readSignal(path);
    },
    restore,
    persist,
    whenRestored() {
      return state.restorePromise;
    },
  });
})();
