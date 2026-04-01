(function () {
  const PAGE_STATE = globalThis.__fishystuffMapPageState;
  const MAP_UI_STORAGE_KEY = PAGE_STATE?.MAP_UI_STORAGE_KEY || "fishystuff.map.window_ui.v1";
  const MAP_BOOKMARKS_STORAGE_KEY =
    PAGE_STATE?.MAP_BOOKMARKS_STORAGE_KEY || "fishystuff.map.bookmarks.v1";
  const MAP_SESSION_STORAGE_KEY =
    PAGE_STATE?.MAP_SESSION_STORAGE_KEY || "fishystuff.map.session.v1";
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
  const FISHYMAP_LIVE_BOOTSTRAP_REQUEST_EVENT = "fishymap-live-bootstrap-request";
  const FISHYMAP_LIVE_READY_EVENT = "fishymap-live-ready";
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
    bootstrapRequestListenerBound: false,
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
    const api = Object.freeze({
      patchSignals,
      signalObject,
      whenRestored() {
        return state.restorePromise;
      },
    });
    state.shell = shell;
    shell.dispatchEvent(new globalThis.CustomEvent(FISHYMAP_LIVE_READY_EVENT, {
      detail: api,
    }));
    return api;
  }

  function connect(signals) {
    state.liveSignals = signals && typeof signals === "object" ? signals : null;
    state.shell = resolveShell() || state.shell;
    ensureShellApi();
    return state.liveSignals;
  }

  function currentLocationHref() {
    return globalThis.location?.href || globalThis.window?.location?.href || "";
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

  function handleBootstrapRequest() {
    ensureShellApi();
  }

  function bindBootstrapRequestListener() {
    const shell = state.shell || resolveShell();
    if (!shell || state.bootstrapRequestListenerBound) {
      return;
    }
    shell.addEventListener(FISHYMAP_LIVE_BOOTSTRAP_REQUEST_EVENT, handleBootstrapRequest);
    state.shell = shell;
    state.bootstrapRequestListenerBound = true;
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

  function restore(signals) {
    connect(signals);
    bindSignalPatchListener();
    const restoreState =
      typeof PAGE_STATE?.loadRestoreState === "function"
        ? PAGE_STATE.loadRestoreState({
            localStorage: globalThis.localStorage,
            sessionStorage: globalThis.sessionStorage,
            locationHref: currentLocationHref(),
          })
        : {
            sharedFishPatch: null,
            uiPatch: null,
            bookmarkPatch: null,
            sessionPatch: null,
          };
    patchSignals(restoreState.sharedFishPatch);
    if (restoreState.uiPatch) {
      patchSignals(restoreState.uiPatch);
    }
    if (restoreState.bookmarkPatch) {
      patchSignals(restoreState.bookmarkPatch);
    }
    if (restoreState.sessionPatch) {
      patchSignals(restoreState.sessionPatch);
    }
    const persistedState =
      typeof PAGE_STATE?.createPersistedState === "function"
        ? PAGE_STATE.createPersistedState(signals)
        : {
            uiJson: "",
            bookmarksJson: "",
            sessionJson: "",
          };
    state.persistedUiJson = persistedState.uiJson;
    state.persistedBookmarksJson = persistedState.bookmarksJson;
    state.persistedSessionJson = persistedState.sessionJson;
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
      const persistedState =
        typeof PAGE_STATE?.createPersistedState === "function"
          ? PAGE_STATE.createPersistedState(snapshot)
          : null;
      const uiJson = persistedState?.uiJson || state.persistedUiJson;
      const bookmarksJson = persistedState?.bookmarksJson || state.persistedBookmarksJson;
      const sessionJson = persistedState?.sessionJson || state.persistedSessionJson;
      if (uiJson !== state.persistedUiJson) {
        globalThis.localStorage?.setItem?.(MAP_UI_STORAGE_KEY, uiJson);
        state.persistedUiJson = uiJson;
      }
      if (bookmarksJson !== state.persistedBookmarksJson) {
        globalThis.localStorage?.setItem?.(MAP_BOOKMARKS_STORAGE_KEY, bookmarksJson);
        state.persistedBookmarksJson = bookmarksJson;
      }
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
  bindBootstrapRequestListener();
})();
