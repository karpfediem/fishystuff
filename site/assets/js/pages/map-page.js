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

  function mapPageStateHelper() {
    return window.__fishystuffMapPageState || null;
  }

  function storedUiSignals(signals) {
    const helper = mapPageStateHelper();
    return helper ? helper.storedUiSignals(signals) : null;
  }

  function uiStorageSnapshot(stored) {
    const helper = mapPageStateHelper();
    return helper ? helper.uiStorageSnapshot(stored) : null;
  }

  function restoreUiPatch(parsed) {
    const helper = mapPageStateHelper();
    return helper ? helper.restoreUiPatch(parsed) : null;
  }

  function sessionStorageSnapshot(stored) {
    const helper = mapPageStateHelper();
    return helper ? helper.sessionStorageSnapshot(stored) : null;
  }

  function restoreSessionPatch(parsed) {
    const helper = mapPageStateHelper();
    return helper ? helper.restoreSessionPatch(parsed) : null;
  }

  function stripQueryOwnedRestoreFields(patch, locationHref = currentLocationHref()) {
    const helper = mapPageStateHelper();
    return helper ? helper.stripQueryOwnedRestoreFields(patch, locationHref) : patch;
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
