(function () {
  const MAP_UI_STORAGE_KEY = "fishystuff.map.window_ui.v1";
  const MAP_BOOKMARKS_STORAGE_KEY = "fishystuff.map.bookmarks.v1";
  const MAP_SESSION_STORAGE_KEY = "fishystuff.map.session.v1";
  const LEGACY_MAP_PREFS_STORAGE_KEY = "fishystuff.map.prefs.v1";
  const SHARED_FISH_STORAGE_KEYS = Object.freeze({
    caught: "fishystuff.fishydex.caught.v1",
    favourites: "fishystuff.fishydex.favourites.v1",
  });
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

  function mapPageSignalsHelper() {
    return window.__fishystuffMapPageSignals || null;
  }

  function patchMatchesPersistFilter(patch) {
    const helper = mapPageSignalsHelper();
    return helper ? helper.patchMatchesPersistFilter(patch) : false;
  }

  function applySignalsPatch(signals, patch) {
    const helper = mapPageSignalsHelper();
    if (!helper) {
      return;
    }
    helper.applyPatchToSignals(signals, patch);
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
          layers: { expandedLayerIds: [] },
          search: { query: "" },
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
