(function () {
  const MAP_UI_STORAGE_KEY = "fishystuff.map.window_ui.v1";
  const MAP_BOOKMARKS_STORAGE_KEY = "fishystuff.map.bookmarks.v1";
  const MAP_PERSIST_SIGNAL_FILTER = /^_(?:map_ui\.windowUi|map_bookmarks\.entries)(?:\.|$)/;
  const state = {
    persistedUiJson: "",
    persistedBookmarksJson: "",
    uiStateRestored: false,
    persistBinding: null,
  };

  function datastarStateHelper() {
    const helper = window.__fishystuffDatastarState;
    return helper && typeof helper.createSignalStore === "function" ? helper : null;
  }

  function createFallbackSignalStore() {
    let signals = null;
    function isPlainObject(value) {
      return value && typeof value === "object" && !Array.isArray(value);
    }
    function mergeObjectPatch(root, patch) {
      if (!isPlainObject(root) || !isPlainObject(patch)) {
        return patch;
      }
      for (const [key, value] of Object.entries(patch)) {
        if (isPlainObject(value) && isPlainObject(root[key])) {
          mergeObjectPatch(root[key], value);
          continue;
        }
        root[key] = value;
      }
      return root;
    }
    return {
      connect(nextSignals) {
        signals = nextSignals && typeof nextSignals === "object" ? nextSignals : null;
        return signals;
      },
      signalObject() {
        return signals && typeof signals === "object" ? signals : null;
      },
      patchSignals(patch) {
        const currentSignals = this.signalObject();
        if (!currentSignals || !patch || typeof patch !== "object") {
          return;
        }
        mergeObjectPatch(currentSignals, patch);
      },
      readSignal(path) {
        return String(path ?? "")
          .split(".")
          .filter(Boolean)
          .reduce((current, key) => {
            if (current && typeof current === "object" && key in current) {
              return current[key];
            }
            return null;
          }, this.signalObject());
      },
    };
  }

  const signalStore = datastarStateHelper()?.createSignalStore() || createFallbackSignalStore();

  function signalObject() {
    return signalStore.signalObject();
  }

  function connect(signals) {
    signalStore.connect(signals);
  }

  function cloneJson(value) {
    return JSON.parse(JSON.stringify(value));
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

  function storedUiSignals(signals) {
    const windowUi = signals?._map_ui?.windowUi;
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
            uiPatch = {
              _map_ui: parsed,
            };
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
  window.__fishystuffMap = Object.freeze({
    connect,
    signalObject,
    patchSignals,
    readSignal(path) {
      return signalStore.readSignal(path);
    },
    restore,
    persist,
  });
})();
