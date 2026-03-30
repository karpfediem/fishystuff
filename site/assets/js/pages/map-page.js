(function () {
  const MAP_UI_STORAGE_KEY = "fishystuff.map.window_ui.v1";
  const MAP_BOOKMARKS_STORAGE_KEY = "fishystuff.map.bookmarks.v1";
  const MAP_PERSIST_SIGNAL_FILTER =
    /^_(?:map_ui\.windowUi|map_input\.ui\.(?:diagnosticsOpen|legendOpen|leftPanelOpen|showPoints|showPointIcons|pointIconScale)|map_input\.filters\.(?:fishIds|zoneRgbs|semanticFieldIdsByLayer|fishFilterTerms|searchText|fromPatchId|toPatchId|layerIdsVisible|layerIdsOrdered|layerOpacities|layerClipMasks|layerWaypointConnectionsVisible|layerWaypointLabelsVisible|layerPointIconsVisible|layerPointIconScales)|map_bookmarks\.entries)(?:\.|$)/;
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

  function restore(signals) {
    connect(signals);
    bindPersistListener();
    let uiPatch = null;
    let bookmarkPatch = null;
    try {
      const rawUi = globalThis.localStorage?.getItem?.(MAP_UI_STORAGE_KEY);
      if (rawUi) {
        try {
          uiPatch = restoreUiPatch(JSON.parse(rawUi));
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
      patchSignals(uiPatch);
    }
    if (bookmarkPatch) {
      patchSignals(bookmarkPatch);
    }
    const stored = storedUiSignals(signals);
    state.persistedUiJson = JSON.stringify(uiStorageSnapshot(stored));
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
