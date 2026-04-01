import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const MAP_PAGE_STATE_SOURCE = fs.readFileSync(new URL("./map-page-state.js", import.meta.url), "utf8");

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

function loadPageState() {
  const context = {
    window: {},
    URL,
    JSON,
    Object,
    Array,
    String,
    Number,
    Set,
    Map,
    console,
    globalThis: null,
  };
  context.globalThis = context;
  vm.runInNewContext(MAP_PAGE_STATE_SOURCE, context, { filename: "map-page-state.js" });
  context.__fishystuffMapPageState = context.window.__fishystuffMapPageState;
  return context.window.__fishystuffMapPageState;
}

test("map-page-state loadRestoreState strips query-owned fields", () => {
  const pageState = loadPageState();
  const localStorage = new MemoryStorage({
    "fishystuff.map.window_ui.v1": JSON.stringify({
      search: { query: "eel" },
      bridgedUi: { diagnosticsOpen: true, showPoints: true, showPointIcons: true, viewMode: "2d" },
      bridgedFilters: {
        fishIds: [77],
        fishFilterTerms: [{ type: "fish", fishId: 77 }],
        layerIdsVisible: ["bookmarks"],
        fromPatchId: "oldest",
        toPatchId: "latest",
      },
    }),
  });

  const restoreState = pageState.loadRestoreState({
    localStorage,
    sessionStorage: new MemoryStorage(),
    locationHref:
      "https://fishystuff.fish/map/?search=tuna&diagnostics=1&fish=77&layers=zone_mask&fromPatch=abc&toPatch=def",
  });

  assert.equal(restoreState.uiPatch._map_ui?.search?.query, undefined);
  assert.equal(restoreState.uiPatch._map_bridged?.ui?.diagnosticsOpen, undefined);
  assert.equal(restoreState.uiPatch._map_bridged?.filters?.fishIds, undefined);
  assert.equal(restoreState.uiPatch._map_bridged?.filters?.layerIdsVisible, undefined);
  assert.equal(restoreState.uiPatch._map_bridged?.filters?.fromPatchId, undefined);
  assert.equal(restoreState.uiPatch._map_bridged?.filters?.toPatchId, undefined);
});

test("map-page-state createPersistedState captures durable map branches", () => {
  const pageState = loadPageState();

  const persisted = pageState.createPersistedState({
    _map_ui: {
      windowUi: {
        search: { open: false, collapsed: true, x: 20, y: 30 },
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
    bridgedUi: {
      diagnosticsOpen: true,
      showPoints: false,
      showPointIcons: false,
      viewMode: "3d",
      pointIconScale: 1.5,
    },
    bridgedFilters: {
      fishIds: [],
      zoneRgbs: [],
      semanticFieldIdsByLayer: {},
      fishFilterTerms: [],
      patchId: null,
      fromPatchId: null,
      toPatchId: null,
      layerIdsVisible: ["bookmarks", "zone_mask"],
      layerIdsOrdered: [],
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
