import { test } from "bun:test";
import assert from "node:assert/strict";

import {
  createMapPresetPayload,
  createPersistedState,
  defaultMapPresetPayload,
  loadRestoreState,
  mapPresetRestorePatch,
  normalizeMapPresetPayload,
} from "./map-page-state.js";

class MemoryStorage {
  constructor(initial = {}) {
    this.map = new Map(Object.entries(initial));
  }

  getItem(key) {
    return this.map.has(key) ? this.map.get(key) : null;
  }

  setItem(key, value) {
    this.map.set(key, String(value));
  }

  removeItem(key) {
    this.map.delete(key);
  }
}

test("map-page-state loadRestoreState strips query-owned fields", () => {
  const localStorage = new MemoryStorage({
    "fishystuff.map.window_ui.v1": JSON.stringify({
      search: { query: "eel" },
      bridgedUi: { diagnosticsOpen: true, showPoints: true, showPointIcons: true, viewMode: "2d" },
      bridgedFilters: {
        layerIdsVisible: ["bookmarks"],
      },
    }),
  });

  const restoreState = loadRestoreState({
    localStorage,
    sessionStorage: new MemoryStorage(),
    locationHref:
      "https://fishystuff.fish/map/?search=tuna&diagnostics=1&fish=77&layers=zone_mask&fromPatch=abc&toPatch=def",
  });

  assert.equal(restoreState.uiPatch._map_ui?.search?.query, undefined);
  assert.equal(restoreState.uiPatch._map_ui?.search?.expression, undefined);
  assert.equal(restoreState.uiPatch._map_ui?.search?.selectedTerms, undefined);
  assert.equal(restoreState.uiPatch._map_bridged?.ui?.diagnosticsOpen, undefined);
  assert.equal(restoreState.uiPatch._map_bridged?.filters?.fishIds, undefined);
  assert.equal(restoreState.uiPatch._map_bridged?.filters?.layerIdsVisible, undefined);
});

test("map-page-state loadRestoreState ignores legacy bridged search filters", () => {
  const localStorage = new MemoryStorage({
    "fishystuff.map.window_ui.v1": JSON.stringify({
      bridgedFilters: {
        fishIds: [77],
        fromPatchId: "2026-02-26",
        toPatchId: "2026-03-12",
      },
    }),
  });

  const restoreState = loadRestoreState({
    localStorage,
    sessionStorage: new MemoryStorage(),
    locationHref: "https://fishystuff.fish/map/",
  });

  assert.equal(restoreState.uiPatch, null);
});

test("map-page-state loadRestoreState restores persisted normalize rates setting", () => {
  const localStorage = new MemoryStorage({
    "fishystuff.map.window_ui.v1": JSON.stringify({
      windowUi: {
        settings: { normalizeRates: false },
      },
    }),
  });

  const restoreState = loadRestoreState({
    localStorage,
    sessionStorage: new MemoryStorage(),
    locationHref: "https://fishystuff.fish/map/",
  });

  assert.equal(restoreState.uiPatch._map_ui.windowUi.settings.normalizeRates, false);
});

test("map-page-state createPersistedState captures durable map branches", () => {
  const persisted = createPersistedState({
    _map_ui: {
      windowUi: {
        search: { open: false, collapsed: true, x: 20, y: 30 },
        settings: { normalizeRates: false },
      },
      layers: {
        expandedLayerIds: ["zone_mask"],
        hoverFactsVisibleByLayer: {
          regions: { origin_region: true },
        },
      },
      search: {
        query: "eel",
        selectedTerms: [{ type: "fish", fishId: 77 }],
      },
    },
    _map_bridged: {
      ui: {
        diagnosticsOpen: true,
        showPoints: false,
        showPointIcons: false,
        viewMode: "3d",
        pointIconScale: 1.5,
      },
      filters: {
        layerIdsVisible: ["bookmarks", "zone_mask"],
        layerFilterBindingIdsDisabledByLayer: {
          fish_evidence: ["zone_mask"],
        },
      },
    },
    _map_bookmarks: {
      entries: [{ id: "bookmark:1", label: "Alpha", worldX: 1, worldZ: 2 }],
    },
    _map_session: {
      view: { viewMode: "2d", camera: { zoom: 2 } },
      selection: { zoneRgb: "1,2,3" },
    },
  });

  assert.deepEqual(JSON.parse(persisted.uiJson), {
    windowUi: {
      search: { open: false, collapsed: true, x: 20, y: 30 },
      settings: { normalizeRates: false },
    },
    layers: {
      expandedLayerIds: ["zone_mask"],
      hoverFactsVisibleByLayer: {
        regions: { origin_region: true },
      },
    },
    search: {
      query: "eel",
      expression: {
        type: "group",
        operator: "or",
        children: [{ type: "term", term: { kind: "fish", fishId: 77 } }],
      },
      selectedTerms: [{ kind: "fish", fishId: 77 }],
    },
    bridgedUi: {
      diagnosticsOpen: true,
      showPoints: false,
      showPointIcons: false,
      viewMode: "3d",
      pointIconScale: 1.5,
    },
    bridgedFilters: {
      layerIdsVisible: ["bookmarks", "zone_mask"],
      layerIdsOrdered: [],
      layerFilterBindingIdsDisabledByLayer: {
        fish_evidence: ["zone_mask"],
      },
      layerOpacities: {},
      layerClipMasks: {},
      layerWaypointConnectionsVisible: {},
      layerWaypointLabelsVisible: {},
      layerPointIconsVisible: {},
      layerPointIconScales: {},
    },
  });
  assert.deepEqual(JSON.parse(persisted.bookmarksJson), [
    { id: "bookmark:1", label: "Alpha", worldX: 1, worldZ: 2 },
  ]);
  assert.deepEqual(JSON.parse(persisted.sessionJson), {
    version: 1,
    view: { viewMode: "2d", camera: { zoom: 2 } },
    selection: { zoneRgb: "1,2,3" },
    filters: {},
  });
});

test("map-page-state map preset payload excludes bookmarks and runtime catalog data", () => {
  const payload = createMapPresetPayload({
    _map_ui: {
      windowUi: {
        search: { open: false, collapsed: true, x: 20, y: 30 },
        settings: { open: true, collapsed: true, x: 40, y: 50, normalizeRates: false },
      },
      layers: {
        expandedLayerIds: ["zone_mask"],
        hoverFactsVisibleByLayer: {
          regions: { origin_region: true },
        },
      },
      search: {
        query: "eel",
        selectedTerms: [{ type: "fish", fishId: 77 }],
      },
    },
    _map_bridged: {
      ui: {
        showPoints: false,
        viewMode: "3d",
        pointIconScale: 1.5,
      },
      filters: {
        layerIdsVisible: ["bookmarks", "zone_mask"],
        layerOpacities: { zone_mask: 0.5 },
      },
    },
    _map_bookmarks: {
      entries: [{ id: "bookmark:1", label: "Alpha", worldX: 1, worldZ: 2 }],
    },
    _map_session: {
      view: { viewMode: "3d", camera: { zoom: 2 } },
      selection: { zoneRgb: "1,2,3" },
    },
    _map_runtime: {
      catalog: { layers: [{ layerId: "zone_mask" }] },
    },
  });

  assert.equal("bookmarks" in payload, false);
  assert.equal("_map_runtime" in payload, false);
  assert.deepEqual(payload.windowUi.search, { open: false, collapsed: true, x: 20, y: 30 });
  assert.equal("open" in payload.windowUi.settings, false);
  assert.equal("collapsed" in payload.windowUi.settings, false);
  assert.equal(payload.windowUi.settings.x, 40);
  assert.equal(payload.windowUi.settings.y, 50);
  assert.equal(payload.windowUi.settings.normalizeRates, false);
  assert.equal(payload.search.query, "eel");
  assert.deepEqual(payload.search.selectedTerms, [{ kind: "fish", fishId: 77 }]);
  assert.deepEqual(payload.bridgedFilters.layerIdsVisible, ["bookmarks", "zone_mask"]);
  assert.deepEqual(payload.bridgedFilters.layerOpacities, { zone_mask: 0.5 });
  assert.deepEqual(payload.view, { viewMode: "3d", camera: {} });
  assert.deepEqual(
    createMapPresetPayload({
      _map_session: {
        view: { viewMode: "3d", camera: { zoom: 2, distance: null } },
      },
    }, { includeCamera: true }).view,
    { viewMode: "3d", camera: { zoom: 2 } },
  );
});

test("map-page-state camera-less map preset view mode follows bridge UI state", () => {
  const payload = createMapPresetPayload({
    _map_bridged: {
      ui: { viewMode: "2d" },
    },
    _map_session: {
      view: { viewMode: "3d", camera: { zoom: 2 } },
    },
  });

  assert.deepEqual(payload.view, { viewMode: "2d", camera: {} });
  assert.deepEqual(
    createMapPresetPayload({
      _map_bridged: {
        ui: { viewMode: "2d" },
      },
      _map_session: {
        view: { viewMode: "3d", camera: { zoom: 2 } },
      },
    }, { includeCamera: true }).view,
    { viewMode: "3d", camera: { zoom: 2 } },
  );
});

test("map-page-state map preset restore patch applies UI and view without bookmarks", () => {
  const payload = normalizeMapPresetPayload({
    ...defaultMapPresetPayload(),
    windowUi: {
      settings: { open: true, collapsed: true, x: 80, y: 90, normalizeRates: false },
    },
    search: {
      query: "tuna",
      selectedTerms: [{ kind: "fish", fishId: 912 }],
    },
    bridgedUi: {
      showPoints: false,
      showPointIcons: true,
      viewMode: "3d",
      pointIconScale: 1.25,
    },
    bridgedFilters: {
      layerIdsVisible: ["zone_mask"],
      layerIdsOrdered: ["zone_mask", "bookmarks"],
      layerClipMasks: { fish_evidence: "zone_mask" },
    },
    view: { viewMode: "3d", camera: { zoom: 4 } },
  });

  const patch = mapPresetRestorePatch(payload);

  assert.equal(patch._map_bookmarks, undefined);
  assert.equal(patch._map_ui.windowUi.settings.normalizeRates, false);
  assert.equal("open" in patch._map_ui.windowUi.settings, false);
  assert.equal("collapsed" in patch._map_ui.windowUi.settings, false);
  assert.equal(patch._map_ui.windowUi.settings.x, 80);
  assert.equal(patch._map_ui.windowUi.settings.y, 90);
  assert.equal(patch._map_ui.search.query, "tuna");
  assert.deepEqual(patch._map_ui.search.selectedTerms, [{ kind: "fish", fishId: 912 }]);
  assert.equal(patch._map_bridged.ui.showPoints, false);
  assert.deepEqual(patch._map_bridged.filters.layerIdsVisible, ["zone_mask"]);
  assert.deepEqual(patch._map_session.view, { viewMode: "3d", camera: { zoom: 4 } });
});

test("map-page-state map default preset restore does not force an empty camera view", () => {
  const patch = mapPresetRestorePatch(defaultMapPresetPayload());

  assert.equal(patch._map_session, undefined);
});
