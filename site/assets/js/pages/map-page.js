(function () {
  const MAP_UI_STORAGE_KEY = "fishystuff.map.window_ui.v1";
  const MAP_PERSIST_SIGNAL_FILTER = /^_map_ui(?:\.|$)/;
  const state = {
    signals: null,
    persistedUiJson: "",
    uiStateRestored: false,
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

  function patchSignals(patch) {
    const signals = signalObject();
    if (!signals || !patch || typeof patch !== "object") {
      return;
    }
    Object.assign(signals, patch);
  }

  function storedUiSignals(signals) {
    const windowUi = signals?._map_ui?.windowUi;
    return {
      _map_ui: {
        windowUi:
          windowUi && typeof windowUi === "object" && !Array.isArray(windowUi)
            ? cloneJson(windowUi)
            : {},
      },
    };
  }

  function restore(signals) {
    connect(signals);
    let patch = null;
    try {
      const raw = globalThis.localStorage?.getItem?.(MAP_UI_STORAGE_KEY);
      if (raw) {
        try {
          const parsed = JSON.parse(raw);
          if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
            patch = parsed;
          }
        } catch (_error) {
          globalThis.localStorage?.removeItem?.(MAP_UI_STORAGE_KEY);
        }
      }
    } catch (_error) {
      patch = null;
    }
    if (patch) {
      Object.assign(signals, patch);
    }
    state.persistedUiJson = JSON.stringify(storedUiSignals(signals));
    state.uiStateRestored = true;
  }

  function persist(signals) {
    const snapshot = signals && typeof signals === "object" ? signals : signalObject();
    if (!snapshot || !state.uiStateRestored) {
      return;
    }
    try {
      const json = JSON.stringify(storedUiSignals(snapshot));
      if (json === state.persistedUiJson) {
        return;
      }
      globalThis.localStorage?.setItem?.(MAP_UI_STORAGE_KEY, json);
      state.persistedUiJson = json;
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
