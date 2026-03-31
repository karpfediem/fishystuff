(function () {
  const DEFAULT_ENABLED_LAYER_IDS = Object.freeze([
    "bookmarks",
    "fish_evidence",
    "zone_mask",
    "minimap",
  ]);

  function cloneJson(value) {
    return JSON.parse(JSON.stringify(value));
  }

  function isPlainObject(value) {
    return Boolean(value) && typeof value === "object" && !Array.isArray(value);
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
    const legacyInputUi = isPlainObject(stored?.inputUi) ? stored.inputUi : {};
    const legacyInputFilters = isPlainObject(stored?.inputFilters) ? stored.inputFilters : {};
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
        semanticFieldIdsByLayer: isPlainObject(bridgedFilters.semanticFieldIdsByLayer)
          ? cloneJson(bridgedFilters.semanticFieldIdsByLayer)
          : isPlainObject(legacyInputFilters.semanticFieldIdsByLayer)
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
        layerOpacities: isPlainObject(bridgedFilters.layerOpacities)
          ? cloneJson(bridgedFilters.layerOpacities)
          : isPlainObject(legacyInputFilters.layerOpacities)
            ? cloneJson(legacyInputFilters.layerOpacities)
            : {},
        layerClipMasks: isPlainObject(bridgedFilters.layerClipMasks)
          ? cloneJson(bridgedFilters.layerClipMasks)
          : isPlainObject(legacyInputFilters.layerClipMasks)
            ? cloneJson(legacyInputFilters.layerClipMasks)
            : {},
        layerWaypointConnectionsVisible: isPlainObject(bridgedFilters.layerWaypointConnectionsVisible)
          ? cloneJson(bridgedFilters.layerWaypointConnectionsVisible)
          : isPlainObject(legacyInputFilters.layerWaypointConnectionsVisible)
            ? cloneJson(legacyInputFilters.layerWaypointConnectionsVisible)
            : {},
        layerWaypointLabelsVisible: isPlainObject(bridgedFilters.layerWaypointLabelsVisible)
          ? cloneJson(bridgedFilters.layerWaypointLabelsVisible)
          : isPlainObject(legacyInputFilters.layerWaypointLabelsVisible)
            ? cloneJson(legacyInputFilters.layerWaypointLabelsVisible)
            : {},
        layerPointIconsVisible: isPlainObject(bridgedFilters.layerPointIconsVisible)
          ? cloneJson(bridgedFilters.layerPointIconsVisible)
          : isPlainObject(legacyInputFilters.layerPointIconsVisible)
            ? cloneJson(legacyInputFilters.layerPointIconsVisible)
            : {},
        layerPointIconScales: isPlainObject(bridgedFilters.layerPointIconScales)
          ? cloneJson(bridgedFilters.layerPointIconScales)
          : isPlainObject(legacyInputFilters.layerPointIconScales)
            ? cloneJson(legacyInputFilters.layerPointIconScales)
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
      patch._map_ui.search = { query: String(search.query || "") };
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
    if (isPlainObject(patch._map_bridged?.filters) && !Object.keys(patch._map_bridged.filters).length) {
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

  window.__fishystuffMapPageState = Object.freeze({
    DEFAULT_ENABLED_LAYER_IDS,
    storedUiSignals,
    uiStorageSnapshot,
    restoreUiPatch,
    sessionStorageSnapshot,
    restoreSessionPatch,
    stripEmptyRestorePatchBranches,
    stripQueryOwnedRestoreFields,
  });
})();
