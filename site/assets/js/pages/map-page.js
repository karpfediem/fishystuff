(function () {
  const MAP_UI_STORAGE_KEY = "fishystuff.map.window_ui.v1";
  const MAP_BOOKMARKS_STORAGE_KEY = "fishystuff.map.bookmarks.v1";
  const MAP_SESSION_STORAGE_KEY = "fishystuff.map.session.v1";
  const LEGACY_MAP_PREFS_STORAGE_KEY = "fishystuff.map.prefs.v1";
  const SHARED_FISH_STORAGE_KEYS = Object.freeze({
    caught: "fishystuff.fishydex.caught.v1",
    favourites: "fishystuff.fishydex.favourites.v1",
  });
  const DEFAULT_ENABLED_LAYER_IDS = Object.freeze([
    "bookmarks",
    "fish_evidence",
    "zone_mask",
    "minimap",
  ]);
  const MAP_PERSIST_SIGNAL_FILTER =
    /^_(?:map_ui\.(?:windowUi|layers(?:\.|$)|search\.query)|map_bridged\.ui\.(?:diagnosticsOpen|showPoints|showPointIcons|viewMode|pointIconScale)|map_bridged\.filters\.(?:fishIds|zoneRgbs|semanticFieldIdsByLayer|fishFilterTerms|patchId|fromPatchId|toPatchId|layerIdsVisible|layerIdsOrdered|layerOpacities|layerClipMasks|layerWaypointConnectionsVisible|layerWaypointLabelsVisible|layerPointIconsVisible|layerPointIconScales)|map_bookmarks\.entries|map_session(?:\.|$))(?:\.|$)/;
  const EXACT_PATCH_PATHS = Object.freeze([
    "_map_ui.layers.expandedLayerIds",
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
  const DATASTAR_SIGNAL_PATCH_EVENT = "datastar-signal-patch";
  const FISHYMAP_SIGNAL_PATCH_EVENT = "fishymap-signals-patch";
  const state = {
    shell: null,
    liveSignals: null,
    persistedUiJson: "",
    persistedBookmarksJson: "",
    persistedSessionJson: "",
    uiStateRestored: false,
    persistTimer: 0,
    signalPatchListenerBound: false,
    restoreResolved: false,
    restorePromise: null,
    resolveRestore: null,
  };
  state.restorePromise = new Promise((resolve) => {
    state.resolveRestore = resolve;
  });

  function cloneJson(value) {
    return JSON.parse(JSON.stringify(value));
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
      if (!current[key] || typeof current[key] !== "object" || Array.isArray(current[key])) {
        current[key] = {};
      }
      current = current[key];
    }
    current[parts[parts.length - 1]] = value;
    return root;
  }

  function applyExactPatchReplacements(signals, patch) {
    if (!signals || typeof signals !== "object" || !patch || typeof patch !== "object") {
      return;
    }
    for (const path of EXACT_PATCH_PATHS) {
      if (!hasObjectPath(patch, path)) {
        continue;
      }
      setObjectPath(signals, path, cloneJson(readObjectPath(patch, path)));
    }
  }

  function signalObject() {
    return state.liveSignals && typeof state.liveSignals === "object" ? state.liveSignals : null;
  }

  function resolveShell() {
    const shell = globalThis.document?.getElementById?.("map-page-shell");
    return shell && typeof shell.dispatchEvent === "function" ? shell : null;
  }

  function connect(signals) {
    state.liveSignals = signals && typeof signals === "object" ? signals : null;
    state.shell = resolveShell();
    return state.liveSignals;
  }

  function currentLocationHref() {
    return globalThis.location?.href || globalThis.window?.location?.href || "";
  }

  function patchMatchesSignalFilter(patch, filter, prefix = "") {
    if (!patch || typeof patch !== "object") {
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
      return value && typeof value === "object" && patchMatchesSignalFilter(value, filter, path);
    });
  }

  function clearPersistTimer() {
    if (!state.persistTimer) {
      return;
    }
    globalThis.clearTimeout?.(state.persistTimer);
    state.persistTimer = 0;
  }

  function schedulePersist() {
    clearPersistTimer();
    state.persistTimer = globalThis.setTimeout?.(() => {
      state.persistTimer = 0;
      persist();
    }, 120);
  }

  function handleSignalPatch(event) {
    if (!state.uiStateRestored) {
      return;
    }
    if (!patchMatchesSignalFilter(event?.detail, { include: MAP_PERSIST_SIGNAL_FILTER })) {
      return;
    }
    schedulePersist();
  }

  function bindSignalPatchListener() {
    if (state.signalPatchListenerBound) {
      return;
    }
    globalThis.document?.addEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);
    state.signalPatchListenerBound = true;
  }

  function applyPatch(signals, patch) {
    const liveSignals = signals && typeof signals === "object" ? signals : state.liveSignals;
    if (!liveSignals || !patch || typeof patch !== "object") {
      return;
    }
    window.__fishystuffDatastarState.mergeObjectPatch(liveSignals, cloneJson(patch));
    applyExactPatchReplacements(liveSignals, patch);
    connect(liveSignals);
    if (state.uiStateRestored && patchMatchesSignalFilter(patch, { include: MAP_PERSIST_SIGNAL_FILTER })) {
      schedulePersist();
    }
  }

  function patchSignals(patch) {
    const shell = state.shell || resolveShell();
    if (
      shell &&
      typeof globalThis.CustomEvent === "function" &&
      patch &&
      typeof patch === "object"
    ) {
      state.shell = shell;
      shell.dispatchEvent(
        new globalThis.CustomEvent(FISHYMAP_SIGNAL_PATCH_EVENT, {
          bubbles: true,
          detail: cloneJson(patch),
        }),
      );
      return;
    }
    applyPatch(state.liveSignals, patch);
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
    const search = signals?._map_ui?.search;
    const bridgedUi = signals?._map_bridged?.ui;
    const bridgedFilters = signals?._map_bridged?.filters;
    const bookmarkEntries = Array.isArray(signals?._map_bookmarks?.entries)
      ? cloneJson(signals._map_bookmarks.entries)
      : [];
    // Map-page persistence is intentionally limited to page-owned durable UI state.
    // Ephemeral locals such as `_map_ui.search.open` and `_map_ui.bookmarks` stay live-only.
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
        search: {
          query: String(search?.query || ""),
        },
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
          zoneRgbs: Array.isArray(bridgedFilters?.zoneRgbs) ? cloneJson(bridgedFilters.zoneRgbs) : [],
          semanticFieldIdsByLayer:
            bridgedFilters?.semanticFieldIdsByLayer &&
            typeof bridgedFilters.semanticFieldIdsByLayer === "object" &&
            !Array.isArray(bridgedFilters.semanticFieldIdsByLayer)
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
          layerOpacities:
            bridgedFilters?.layerOpacities &&
            typeof bridgedFilters.layerOpacities === "object" &&
            !Array.isArray(bridgedFilters.layerOpacities)
              ? cloneJson(bridgedFilters.layerOpacities)
              : {},
          layerClipMasks:
            bridgedFilters?.layerClipMasks &&
            typeof bridgedFilters.layerClipMasks === "object" &&
            !Array.isArray(bridgedFilters.layerClipMasks)
              ? cloneJson(bridgedFilters.layerClipMasks)
              : {},
          layerWaypointConnectionsVisible:
            bridgedFilters?.layerWaypointConnectionsVisible &&
            typeof bridgedFilters.layerWaypointConnectionsVisible === "object" &&
            !Array.isArray(bridgedFilters.layerWaypointConnectionsVisible)
              ? cloneJson(bridgedFilters.layerWaypointConnectionsVisible)
              : {},
          layerWaypointLabelsVisible:
            bridgedFilters?.layerWaypointLabelsVisible &&
            typeof bridgedFilters.layerWaypointLabelsVisible === "object" &&
            !Array.isArray(bridgedFilters.layerWaypointLabelsVisible)
              ? cloneJson(bridgedFilters.layerWaypointLabelsVisible)
              : {},
          layerPointIconsVisible:
            bridgedFilters?.layerPointIconsVisible &&
            typeof bridgedFilters.layerPointIconsVisible === "object" &&
            !Array.isArray(bridgedFilters.layerPointIconsVisible)
              ? cloneJson(bridgedFilters.layerPointIconsVisible)
              : {},
          layerPointIconScales:
            bridgedFilters?.layerPointIconScales &&
            typeof bridgedFilters.layerPointIconScales === "object" &&
            !Array.isArray(bridgedFilters.layerPointIconScales)
              ? cloneJson(bridgedFilters.layerPointIconScales)
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
    const legacyInputUi =
      stored?.inputUi && typeof stored.inputUi === "object" && !Array.isArray(stored.inputUi)
        ? stored.inputUi
        : {};
    const legacyInputFilters =
      stored?.inputFilters &&
      typeof stored.inputFilters === "object" &&
      !Array.isArray(stored.inputFilters)
        ? stored.inputFilters
        : {};
    const bridgedUi =
      stored?._map_bridged?.ui &&
      typeof stored._map_bridged.ui === "object" &&
      !Array.isArray(stored._map_bridged.ui)
        ? stored._map_bridged.ui
        : {};
    const bridgedFilters =
      stored?._map_bridged?.filters &&
      typeof stored._map_bridged.filters === "object" &&
      !Array.isArray(stored._map_bridged.filters)
        ? stored._map_bridged.filters
        : {};
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
      search: {
        query: String(stored?._map_ui?.search?.query || ""),
      },
      bridgedUi: {
        diagnosticsOpen: bridgedUi.diagnosticsOpen === true || legacyInputUi.diagnosticsOpen === true,
        showPoints: bridgedUi.showPoints !== false && legacyInputUi.showPoints !== false,
        showPointIcons:
          bridgedUi.showPointIcons !== false && legacyInputUi.showPointIcons !== false,
        viewMode:
          bridgedUi.viewMode === "3d" || legacyInputUi.viewMode === "3d" ? "3d" : "2d",
        pointIconScale: Number.isFinite(bridgedUi.pointIconScale)
          ? Number(bridgedUi.pointIconScale)
          : Number.isFinite(legacyInputUi.pointIconScale)
            ? Number(legacyInputUi.pointIconScale)
            : 1,
      },
      bridgedFilters: {
        fishIds: Array.isArray(bridgedFilters.fishIds)
          ? cloneJson(bridgedFilters.fishIds)
          : Array.isArray(legacyInputFilters.fishIds)
            ? cloneJson(legacyInputFilters.fishIds)
            : [],
        zoneRgbs: Array.isArray(bridgedFilters.zoneRgbs)
          ? cloneJson(bridgedFilters.zoneRgbs)
          : Array.isArray(legacyInputFilters.zoneRgbs)
            ? cloneJson(legacyInputFilters.zoneRgbs)
            : [],
        semanticFieldIdsByLayer:
          bridgedFilters.semanticFieldIdsByLayer &&
          typeof bridgedFilters.semanticFieldIdsByLayer === "object" &&
          !Array.isArray(bridgedFilters.semanticFieldIdsByLayer)
            ? cloneJson(bridgedFilters.semanticFieldIdsByLayer)
            : legacyInputFilters.semanticFieldIdsByLayer &&
                typeof legacyInputFilters.semanticFieldIdsByLayer === "object" &&
                !Array.isArray(legacyInputFilters.semanticFieldIdsByLayer)
              ? cloneJson(legacyInputFilters.semanticFieldIdsByLayer)
              : {},
        fishFilterTerms: Array.isArray(bridgedFilters.fishFilterTerms)
          ? cloneJson(bridgedFilters.fishFilterTerms)
          : Array.isArray(legacyInputFilters.fishFilterTerms)
            ? cloneJson(legacyInputFilters.fishFilterTerms)
            : [],
        patchId:
          bridgedFilters.patchId == null
            ? legacyInputFilters.patchId == null
              ? null
              : String(legacyInputFilters.patchId)
            : String(bridgedFilters.patchId),
        fromPatchId:
          bridgedFilters.fromPatchId == null
            ? legacyInputFilters.fromPatchId == null
              ? null
              : String(legacyInputFilters.fromPatchId)
            : String(bridgedFilters.fromPatchId),
        toPatchId:
          bridgedFilters.toPatchId == null
            ? legacyInputFilters.toPatchId == null
              ? null
              : String(legacyInputFilters.toPatchId)
            : String(bridgedFilters.toPatchId),
        layerIdsVisible: Array.isArray(bridgedFilters.layerIdsVisible)
          ? cloneJson(bridgedFilters.layerIdsVisible)
          : Array.isArray(legacyInputFilters.layerIdsVisible)
            ? cloneJson(legacyInputFilters.layerIdsVisible)
            : cloneJson(DEFAULT_ENABLED_LAYER_IDS),
        layerIdsOrdered: Array.isArray(bridgedFilters.layerIdsOrdered)
          ? cloneJson(bridgedFilters.layerIdsOrdered)
          : Array.isArray(legacyInputFilters.layerIdsOrdered)
            ? cloneJson(legacyInputFilters.layerIdsOrdered)
            : [],
        layerOpacities:
          bridgedFilters.layerOpacities &&
          typeof bridgedFilters.layerOpacities === "object" &&
          !Array.isArray(bridgedFilters.layerOpacities)
            ? cloneJson(bridgedFilters.layerOpacities)
            : legacyInputFilters.layerOpacities &&
                typeof legacyInputFilters.layerOpacities === "object" &&
                !Array.isArray(legacyInputFilters.layerOpacities)
              ? cloneJson(legacyInputFilters.layerOpacities)
              : {},
        layerClipMasks:
          bridgedFilters.layerClipMasks &&
          typeof bridgedFilters.layerClipMasks === "object" &&
          !Array.isArray(bridgedFilters.layerClipMasks)
            ? cloneJson(bridgedFilters.layerClipMasks)
            : legacyInputFilters.layerClipMasks &&
                typeof legacyInputFilters.layerClipMasks === "object" &&
                !Array.isArray(legacyInputFilters.layerClipMasks)
              ? cloneJson(legacyInputFilters.layerClipMasks)
              : {},
        layerWaypointConnectionsVisible:
          bridgedFilters.layerWaypointConnectionsVisible &&
          typeof bridgedFilters.layerWaypointConnectionsVisible === "object" &&
          !Array.isArray(bridgedFilters.layerWaypointConnectionsVisible)
            ? cloneJson(bridgedFilters.layerWaypointConnectionsVisible)
            : legacyInputFilters.layerWaypointConnectionsVisible &&
                typeof legacyInputFilters.layerWaypointConnectionsVisible === "object" &&
                !Array.isArray(legacyInputFilters.layerWaypointConnectionsVisible)
              ? cloneJson(legacyInputFilters.layerWaypointConnectionsVisible)
              : {},
        layerWaypointLabelsVisible:
          bridgedFilters.layerWaypointLabelsVisible &&
          typeof bridgedFilters.layerWaypointLabelsVisible === "object" &&
          !Array.isArray(bridgedFilters.layerWaypointLabelsVisible)
            ? cloneJson(bridgedFilters.layerWaypointLabelsVisible)
            : legacyInputFilters.layerWaypointLabelsVisible &&
                typeof legacyInputFilters.layerWaypointLabelsVisible === "object" &&
                !Array.isArray(legacyInputFilters.layerWaypointLabelsVisible)
              ? cloneJson(legacyInputFilters.layerWaypointLabelsVisible)
              : {},
        layerPointIconsVisible:
          bridgedFilters.layerPointIconsVisible &&
          typeof bridgedFilters.layerPointIconsVisible === "object" &&
          !Array.isArray(bridgedFilters.layerPointIconsVisible)
            ? cloneJson(bridgedFilters.layerPointIconsVisible)
            : legacyInputFilters.layerPointIconsVisible &&
                typeof legacyInputFilters.layerPointIconsVisible === "object" &&
                !Array.isArray(legacyInputFilters.layerPointIconsVisible)
              ? cloneJson(legacyInputFilters.layerPointIconsVisible)
              : {},
        layerPointIconScales:
          bridgedFilters.layerPointIconScales &&
          typeof bridgedFilters.layerPointIconScales === "object" &&
          !Array.isArray(bridgedFilters.layerPointIconScales)
            ? cloneJson(bridgedFilters.layerPointIconScales)
            : legacyInputFilters.layerPointIconScales &&
                typeof legacyInputFilters.layerPointIconScales === "object" &&
                !Array.isArray(legacyInputFilters.layerPointIconScales)
              ? cloneJson(legacyInputFilters.layerPointIconScales)
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
    const search = parsed.search && typeof parsed.search === "object" && !Array.isArray(parsed.search)
      ? parsed.search
      : parsed.inputFilters &&
          typeof parsed.inputFilters === "object" &&
          !Array.isArray(parsed.inputFilters)
        ? { query: parsed.inputFilters.searchText }
        : {};
    const bridgedUi =
      parsed.bridgedUi && typeof parsed.bridgedUi === "object" && !Array.isArray(parsed.bridgedUi)
        ? parsed.bridgedUi
        : parsed.inputUi && typeof parsed.inputUi === "object" && !Array.isArray(parsed.inputUi)
          ? parsed.inputUi
          : null;
    const bridgedFilters =
      parsed.bridgedFilters &&
      typeof parsed.bridgedFilters === "object" &&
      !Array.isArray(parsed.bridgedFilters)
        ? parsed.bridgedFilters
        : parsed.inputFilters &&
            typeof parsed.inputFilters === "object" &&
            !Array.isArray(parsed.inputFilters)
          ? parsed.inputFilters
          : null;

    if (Object.keys(search).length) {
      patch._map_ui = patch._map_ui || {};
      patch._map_ui.search = {
        query: String(search.query || ""),
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
        semanticFieldIdsByLayer:
          bridgedFilters.semanticFieldIdsByLayer &&
          typeof bridgedFilters.semanticFieldIdsByLayer === "object" &&
          !Array.isArray(bridgedFilters.semanticFieldIdsByLayer)
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
        layerOpacities:
          bridgedFilters.layerOpacities &&
          typeof bridgedFilters.layerOpacities === "object" &&
          !Array.isArray(bridgedFilters.layerOpacities)
            ? cloneJson(bridgedFilters.layerOpacities)
            : {},
        layerClipMasks:
          bridgedFilters.layerClipMasks &&
          typeof bridgedFilters.layerClipMasks === "object" &&
          !Array.isArray(bridgedFilters.layerClipMasks)
            ? cloneJson(bridgedFilters.layerClipMasks)
            : {},
        layerWaypointConnectionsVisible:
          bridgedFilters.layerWaypointConnectionsVisible &&
          typeof bridgedFilters.layerWaypointConnectionsVisible === "object" &&
          !Array.isArray(bridgedFilters.layerWaypointConnectionsVisible)
            ? cloneJson(bridgedFilters.layerWaypointConnectionsVisible)
            : {},
        layerWaypointLabelsVisible:
          bridgedFilters.layerWaypointLabelsVisible &&
          typeof bridgedFilters.layerWaypointLabelsVisible === "object" &&
          !Array.isArray(bridgedFilters.layerWaypointLabelsVisible)
            ? cloneJson(bridgedFilters.layerWaypointLabelsVisible)
            : {},
        layerPointIconsVisible:
          bridgedFilters.layerPointIconsVisible &&
          typeof bridgedFilters.layerPointIconsVisible === "object" &&
          !Array.isArray(bridgedFilters.layerPointIconsVisible)
            ? cloneJson(bridgedFilters.layerPointIconsVisible)
            : {},
        layerPointIconScales:
          bridgedFilters.layerPointIconScales &&
          typeof bridgedFilters.layerPointIconScales === "object" &&
          !Array.isArray(bridgedFilters.layerPointIconScales)
            ? cloneJson(bridgedFilters.layerPointIconScales)
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
    if (patch._map_bridged?.ui && !Object.keys(patch._map_bridged.ui).length) {
      delete patch._map_bridged.ui;
    }
    if (patch._map_bridged?.filters && !Object.keys(patch._map_bridged.filters).length) {
      delete patch._map_bridged.filters;
    }
    if (patch._map_bridged && !Object.keys(patch._map_bridged).length) {
      delete patch._map_bridged;
    }
    if (patch._map_ui?.windowUi && !Object.keys(patch._map_ui.windowUi).length) {
      delete patch._map_ui.windowUi;
    }
    if (patch._map_ui?.layers && !Object.keys(patch._map_ui.layers).length) {
      delete patch._map_ui.layers;
    }
    if (patch._map_ui?.search && !Object.keys(patch._map_ui.search).length) {
      delete patch._map_ui.search;
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
      (
        params.has("patch") ||
        params.has("fromPatch") ||
        params.has("patchFrom") ||
        params.has("toPatch") ||
        params.has("untilPatch") ||
        params.has("patchTo")
      )
    ) {
      delete bridgedFilters.fromPatchId;
      delete bridgedFilters.toPatchId;
    }

    return stripEmptyRestorePatchBranches(nextPatch);
  }

  function restore(signals) {
    connect(signals);
    bindSignalPatchListener();
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
    signalObject,
    patchSignals,
    applyPatch,
    restore,
    whenRestored() {
      return state.restorePromise;
    },
  });
})();
