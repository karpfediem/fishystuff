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
  const MAP_PERSIST_SIGNAL_FILTER =
    /^_(?:map_ui\.(?:windowUi|layers(?:\.|$)|search\.(?:query|selectedTerms))|map_bridged\.ui\.(?:diagnosticsOpen|showPoints|showPointIcons|viewMode|pointIconScale)|map_bridged\.filters\.(?:fishIds|zoneRgbs|semanticFieldIdsByLayer|fishFilterTerms|patchId|fromPatchId|toPatchId|layerIdsVisible|layerIdsOrdered|layerOpacities|layerClipMasks|layerWaypointConnectionsVisible|layerWaypointLabelsVisible|layerPointIconsVisible|layerPointIconScales)|map_bookmarks\.entries|map_session(?:\.|$))(?:\.|$)/;
  const EXACT_PATCH_PATHS = Object.freeze([
    "_map_ui.layers.expandedLayerIds",
    "_map_ui.layers.hoverFactsVisibleByLayer",
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
  const FISHYMAP_LIVE_INIT_EVENT = "fishymap-live-init";
  const SHELL_SIGNAL_API_KEY = "__fishystuffMapPage";
  const state = {
    shell: null,
    liveSignals: null,
    persistedUiJson: "",
    persistedBookmarksJson: "",
    persistedSessionJson: "",
    uiStateRestored: false,
    persistTimer: 0,
    signalPatchListenerBound: false,
    initListenerBound: false,
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

  function applyExactPatchReplacements(signals, patch) {
    if (!isPlainObject(signals) || !isPlainObject(patch)) {
      return;
    }
    for (const path of EXACT_PATCH_PATHS) {
      if (!hasObjectPath(patch, path)) {
        continue;
      }
      setObjectPath(signals, path, cloneJson(readObjectPath(patch, path)));
    }
  }

  function applySignalsPatch(signals, patch) {
    if (!isPlainObject(signals) || !isPlainObject(patch)) {
      return;
    }
    mergeObjectPatch(signals, cloneJson(patch));
    applyExactPatchReplacements(signals, patch);
  }

  function patchMatchesSignalFilter(patch, filter, prefix = "") {
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

  function patchMatchesPersistFilter(patch) {
    return patchMatchesSignalFilter(patch, { include: MAP_PERSIST_SIGNAL_FILTER });
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
      _map_bookmarks: {
        entries: bookmarkEntries,
      },
      _map_session: isPlainObject(signals?._map_session)
        ? cloneJson(signals._map_session)
        : {
            view: { viewMode: "2d", camera: {} },
            selection: {},
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

  function signalObject() {
    return state.liveSignals && typeof state.liveSignals === "object" ? state.liveSignals : null;
  }

  function resolveShell() {
    const shell = globalThis.document?.getElementById?.("map-page-shell");
    return shell && typeof shell.dispatchEvent === "function" ? shell : null;
  }

  function ensureShellApi() {
    const shell = state.shell || resolveShell();
    if (!shell) {
      return null;
    }
    shell[SHELL_SIGNAL_API_KEY] = Object.freeze({
      patchSignals,
      signalObject,
      whenRestored() {
        return state.restorePromise;
      },
    });
    state.shell = shell;
    return shell;
  }

  function connect(signals) {
    state.liveSignals = signals && typeof signals === "object" ? signals : null;
    state.shell = ensureShellApi();
    return state.liveSignals;
  }

  function currentLocationHref() {
    return globalThis.location?.href || globalThis.window?.location?.href || "";
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
    if (!patchMatchesPersistFilter(event?.detail)) {
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

  function handleLiveInit(event) {
    const signals = event?.detail;
    if (!signals || typeof signals !== "object") {
      return;
    }
    restore(signals);
  }

  function bindInitListener() {
    const shell = state.shell || resolveShell();
    if (!shell || state.initListenerBound) {
      return;
    }
    shell.addEventListener(FISHYMAP_LIVE_INIT_EVENT, handleLiveInit);
    state.shell = shell;
    state.initListenerBound = true;
  }

  function applyPatch(signals, patch) {
    const liveSignals = signals && typeof signals === "object" ? signals : state.liveSignals;
    if (!liveSignals || !patch || typeof patch !== "object") {
      return;
    }
    applySignalsPatch(liveSignals, patch);
    connect(liveSignals);
    if (state.uiStateRestored && patchMatchesPersistFilter(patch)) {
      schedulePersist();
    }
  }

  function patchSignals(patch) {
    applyPatch(state.liveSignals, patch);
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

  function restoreSharedFishPatch() {
    let caughtIds = [];
    let favouriteIds = [];
    try {
      caughtIds = normalizeFallbackFishIds(JSON.parse(
        globalThis.localStorage?.getItem?.(SHARED_FISH_STORAGE_KEYS.caught) || "[]",
      ));
    } catch (_error) {
      caughtIds = [];
      globalThis.localStorage?.removeItem?.(SHARED_FISH_STORAGE_KEYS.caught);
    }
    try {
      favouriteIds = normalizeFallbackFishIds(JSON.parse(
        globalThis.localStorage?.getItem?.(SHARED_FISH_STORAGE_KEYS.favourites) || "[]",
      ));
    } catch (_error) {
      favouriteIds = [];
      globalThis.localStorage?.removeItem?.(SHARED_FISH_STORAGE_KEYS.favourites);
    }
    return {
      _shared_fish: {
        caughtIds,
        favouriteIds,
      },
    };
  }

  function restore(signals) {
    connect(signals);
    bindSignalPatchListener();
    const sharedFishPatch = restoreSharedFishPatch();
    let uiPatch = null;
    let bookmarkPatch = null;
    let sessionPatch = null;
    try {
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
    const stored = ensureStoredSignals(storedUiSignals(signals));
    state.persistedUiJson = JSON.stringify(ensureUiSnapshot(uiStorageSnapshot(stored)));
    state.persistedBookmarksJson = JSON.stringify(stored._map_bookmarks.entries);
    state.persistedSessionJson = JSON.stringify(ensureSessionSnapshot(sessionStorageSnapshot(stored)));
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
      const stored = ensureStoredSignals(storedUiSignals(snapshot));
      const uiJson = JSON.stringify(ensureUiSnapshot(uiStorageSnapshot(stored)));
      const bookmarksJson = JSON.stringify(stored._map_bookmarks.entries);
      if (uiJson !== state.persistedUiJson) {
        globalThis.localStorage?.setItem?.(MAP_UI_STORAGE_KEY, uiJson);
        state.persistedUiJson = uiJson;
      }
      if (bookmarksJson !== state.persistedBookmarksJson) {
        globalThis.localStorage?.setItem?.(MAP_BOOKMARKS_STORAGE_KEY, bookmarksJson);
        state.persistedBookmarksJson = bookmarksJson;
      }
      const sessionJson = JSON.stringify(ensureSessionSnapshot(sessionStorageSnapshot(stored)));
      if (sessionJson !== state.persistedSessionJson) {
        globalThis.sessionStorage?.setItem?.(MAP_SESSION_STORAGE_KEY, sessionJson);
        state.persistedSessionJson = sessionJson;
      }
    } catch (_error) {
      // Map UI persistence is best-effort only.
    }
  }

  state.shell = resolveShell();
  bindInitListener();
})();
