(function () {
  const DATASTAR_SIGNAL_PATCH_EVENT = "datastar-signal-patch";
  const MAP_UI_STORAGE_KEY = "fishystuff.map.window_ui.v1";
  const MAP_BOOKMARKS_STORAGE_KEY = "fishystuff.map.bookmarks.v1";
  const MAP_PERSIST_SIGNAL_FILTER = /^_(?:map_ui|map_bookmarks)(?:\.|$)/;
  const state = {
    signals: null,
    persistedUiJson: "",
    persistedBookmarksJson: "",
    uiStateRestored: false,
    persistTimer: 0,
    persistListenerBound: false,
  };

  function signalObject() {
    return state.signals && typeof state.signals === "object" ? state.signals : null;
  }

  function connect(signals) {
    state.signals = signals && typeof signals === "object" ? signals : null;
  }

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
        return null;
      }, root);
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
    state.persistTimer = globalThis.setTimeout(() => {
      state.persistTimer = 0;
      persist();
    }, 120);
  }

  function patchIncludesPersistedSignals(patch, prefix = "") {
    if (!patch || typeof patch !== "object") {
      return false;
    }
    return Object.entries(patch).some(([key, value]) => {
      const path = prefix ? `${prefix}.${key}` : key;
      if (MAP_PERSIST_SIGNAL_FILTER.test(path)) {
        return true;
      }
      return value && typeof value === "object" && patchIncludesPersistedSignals(value, path);
    });
  }

  function handleSignalPatch(event) {
    if (!state.uiStateRestored) {
      return;
    }
    const patch = event?.detail;
    if (!patchIncludesPersistedSignals(patch)) {
      return;
    }
    schedulePersist();
  }

  function bindPersistListener() {
    if (state.persistListenerBound) {
      return;
    }
    document.addEventListener(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);
    state.persistListenerBound = true;
  }

  function patchSignals(patch) {
    const signals = signalObject();
    if (!signals || !patch || typeof patch !== "object") {
      return;
    }
    Object.assign(signals, patch);
  }

  function storedUiSignals(signals) {
    const windowUi = signals?._map_ui?.windowUi;
    const bookmarkEntries = Array.isArray(signals?._map_bookmarks?.entries)
      ? cloneJson(signals._map_bookmarks.entries)
      : [];
    return {
      _map_ui: {
        windowUi:
          windowUi && typeof windowUi === "object" && !Array.isArray(windowUi)
            ? cloneJson(windowUi)
            : {},
      },
      _map_bookmarks: {
        entries: bookmarkEntries,
      },
    };
  }

  function restore(signals) {
    connect(signals);
    bindPersistListener();
    let uiPatch = null;
    let bookmarkPatch = null;
    try {
      const rawUi = globalThis.localStorage?.getItem?.(MAP_UI_STORAGE_KEY);
      if (rawUi) {
        try {
          const parsed = JSON.parse(rawUi);
          if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
            uiPatch = parsed;
          }
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
    } catch (_error) {
      uiPatch = null;
      bookmarkPatch = null;
    }
    if (uiPatch) {
      Object.assign(signals, uiPatch);
    }
    if (bookmarkPatch) {
      Object.assign(signals, bookmarkPatch);
    }
    const stored = storedUiSignals(signals);
    state.persistedUiJson = JSON.stringify(stored._map_ui);
    state.persistedBookmarksJson = JSON.stringify(stored._map_bookmarks.entries);
    state.uiStateRestored = true;
  }

  function persist() {
    const snapshot = signalObject();
    if (!snapshot || !state.uiStateRestored) {
      return;
    }
    try {
      const stored = storedUiSignals(snapshot);
      const uiJson = JSON.stringify(stored._map_ui);
      const bookmarksJson = JSON.stringify(stored._map_bookmarks.entries);
      if (uiJson !== state.persistedUiJson) {
        globalThis.localStorage?.setItem?.(MAP_UI_STORAGE_KEY, uiJson);
        state.persistedUiJson = uiJson;
      }
      if (bookmarksJson !== state.persistedBookmarksJson) {
        globalThis.localStorage?.setItem?.(MAP_BOOKMARKS_STORAGE_KEY, bookmarksJson);
        state.persistedBookmarksJson = bookmarksJson;
      }
    } catch (_error) {
      // Map UI persistence is best-effort only.
    }
  }

  function persistSignalPatchFilter() {
    return {
      include: MAP_PERSIST_SIGNAL_FILTER,
    };
  }

  function toggleWindow(windowId) {
    const signals = signalObject();
    if (!signals) {
      return;
    }
    const mapUi =
      signals._map_ui && typeof signals._map_ui === "object" && !Array.isArray(signals._map_ui)
        ? cloneJson(signals._map_ui)
        : {};
    const windowUi =
      mapUi.windowUi && typeof mapUi.windowUi === "object" && !Array.isArray(mapUi.windowUi)
        ? mapUi.windowUi
        : {};
    const currentEntry =
      windowUi[windowId] && typeof windowUi[windowId] === "object" && !Array.isArray(windowUi[windowId])
        ? windowUi[windowId]
        : {};
    patchSignals({
      _map_ui: {
        ...mapUi,
        windowUi: {
          ...windowUi,
          [windowId]: {
            ...currentEntry,
            open: currentEntry.open === false,
          },
        },
      },
    });
  }

  window.__fishystuffMap = Object.freeze({
    connect,
    signalObject,
    patchSignals,
    readSignal(path) {
      return readObjectPath(signalObject(), path);
    },
    restore,
    persist,
    persistSignalPatchFilter,
    toggleWindow,
  });
})();
