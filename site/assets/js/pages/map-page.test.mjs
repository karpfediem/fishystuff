import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const DATASTAR_STATE_SOURCE = fs.readFileSync(
  new URL("../datastar-state.js", import.meta.url),
  "utf8",
);
const DATASTAR_PERSIST_SOURCE = fs.readFileSync(
  new URL("../datastar-persist.js", import.meta.url),
  "utf8",
);
const MAP_PAGE_STATE_SOURCE = fs.readFileSync(
  new URL("../../map/map-page-state.js", import.meta.url),
  "utf8",
);
const MAP_PAGE_SIGNALS_SOURCE = fs.readFileSync(
  new URL("../../map/map-page-signals.js", import.meta.url),
  "utf8",
);
const MAP_PAGE_SOURCE = fs.readFileSync(new URL("./map-page.js", import.meta.url), "utf8");
const DEFAULT_ENABLED_LAYER_IDS = Object.freeze([
  "bookmarks",
  "fish_evidence",
  "zone_mask",
  "minimap",
]);

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

function createDocumentStub() {
  const listeners = new Map();
  return {
    addEventListener(type, listener) {
      if (!listeners.has(type)) {
        listeners.set(type, []);
      }
      listeners.get(type).push(listener);
    },
    dispatchEvent(event) {
      for (const listener of listeners.get(event.type) || []) {
        listener(event);
      }
    },
  };
}

function createContext(localStorageInitial = {}, options = {}) {
  const document = createDocumentStub();
  const window = {};
  const localStorage = new MemoryStorage(localStorageInitial);
  const sessionStorage = new MemoryStorage(options.sessionStorageInitial || {});
  const location = {
    href: options.locationHref || "https://fishystuff.fish/map/",
  };
  const timers = new Map();
  let nextTimerId = 1;
  const context = {
    window,
    document,
    location,
    localStorage,
    sessionStorage,
    JSON,
    Object,
    Array,
    String,
    URL,
    URLSearchParams,
    RegExp,
    Error,
    Map,
    Set,
    console,
    globalThis: null,
    setTimeout(callback) {
      const id = nextTimerId;
      nextTimerId += 1;
      timers.set(id, callback);
      return id;
    },
    clearTimeout(id) {
      timers.delete(id);
    },
  };
  context.globalThis = context;
  vm.runInNewContext(DATASTAR_STATE_SOURCE, context, { filename: "datastar-state.js" });
  window.location = location;
  window.sessionStorage = sessionStorage;
  window.localStorage = localStorage;
  vm.runInNewContext(DATASTAR_PERSIST_SOURCE, context, { filename: "datastar-persist.js" });
  vm.runInNewContext(MAP_PAGE_STATE_SOURCE, context, { filename: "map-page-state.js" });
  vm.runInNewContext(MAP_PAGE_SIGNALS_SOURCE, context, { filename: "map-page-signals.js" });
  vm.runInNewContext(MAP_PAGE_SOURCE, context, { filename: "map-page.js" });
  return {
    window,
    document,
    location,
    localStorage,
    sessionStorage,
    flushTimers() {
      const pending = Array.from(timers.values());
      timers.clear();
      for (const callback of pending) {
        callback();
      }
    },
  };
}

function defaultSignals() {
  return {
    _map_ui: {
      windowUi: {
        search: { open: true, collapsed: false, x: null, y: null },
        settings: { open: true, collapsed: false, x: null, y: null, autoAdjustView: true },
        zoneInfo: { open: true, collapsed: false, x: null, y: null, tab: "" },
        layers: { open: true, collapsed: false, x: null, y: null },
        bookmarks: { open: false, collapsed: false, x: null, y: null },
      },
      search: { open: false, query: "" },
      bookmarks: { placing: false, selectedIds: [] },
      layers: { expandedLayerIds: [] },
    },
    _map_bookmarks: {
      entries: [],
    },
    _map_bridged: {
      filters: {
        fishIds: [],
        zoneRgbs: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: [],
        patchId: null,
        fromPatchId: null,
        toPatchId: null,
        layerIdsVisible: [...DEFAULT_ENABLED_LAYER_IDS],
        layerIdsOrdered: [],
        layerOpacities: {},
        layerClipMasks: {},
        layerWaypointConnectionsVisible: {},
        layerWaypointLabelsVisible: {},
        layerPointIconsVisible: {},
        layerPointIconScales: {},
      },
      ui: {
        diagnosticsOpen: false,
        showPoints: true,
        showPointIcons: true,
        viewMode: null,
        pointIconScale: 1,
        bookmarkSelectedIds: [],
        bookmarks: [],
      },
    },
    _map_session: {
      view: {
        viewMode: "2d",
        camera: {},
      },
      selection: {},
    },
    _shared_fish: {
      caughtIds: [],
      favouriteIds: [],
    },
  };
}

function defaultStoredUiSnapshot() {
  return {
    windowUi: {
      search: { open: true, collapsed: false, x: null, y: null },
      settings: { open: true, collapsed: false, x: null, y: null, autoAdjustView: true },
      zoneInfo: { open: true, collapsed: false, x: null, y: null, tab: "" },
      layers: { open: true, collapsed: false, x: null, y: null },
      bookmarks: { open: false, collapsed: false, x: null, y: null },
    },
    layers: {
      expandedLayerIds: [],
    },
    search: {
      query: "",
    },
    bridgedUi: {
      diagnosticsOpen: false,
      showPoints: true,
      showPointIcons: true,
      viewMode: "2d",
      pointIconScale: 1,
    },
    bridgedFilters: {
      fishIds: [],
      zoneRgbs: [],
      semanticFieldIdsByLayer: {},
      fishFilterTerms: [],
        patchId: null,
        fromPatchId: null,
        toPatchId: null,
        layerIdsVisible: [...DEFAULT_ENABLED_LAYER_IDS],
        layerIdsOrdered: [],
        layerOpacities: {},
        layerClipMasks: {},
        layerWaypointConnectionsVisible: {},
        layerWaypointLabelsVisible: {},
        layerPointIconsVisible: {},
        layerPointIconScales: {},
    },
  };
}

test("map-page restore loads persisted bookmark entries into Datastar signals", () => {
  const persistedBookmarks = [
    { id: "bookmark-1", label: "Persisted", worldX: 10, worldZ: 20, layerSamples: [] },
  ];
  const env = createContext({
    "fishystuff.map.bookmarks.v1": JSON.stringify(persistedBookmarks),
  });
  const signals = defaultSignals();

  env.window.__fishystuffMap.restore(signals);

  assert.deepEqual(signals._map_bookmarks.entries, persistedBookmarks);
});

test("map-page restore loads shared fish state into Datastar signals", () => {
  const env = createContext({
    "fishystuff.fishydex.caught.v1": JSON.stringify([77, 912]),
    "fishystuff.fishydex.favourites.v1": JSON.stringify([912]),
  });
  const signals = defaultSignals();

  env.window.__fishystuffMap.restore(signals);

  assert.deepEqual(JSON.parse(JSON.stringify(signals._shared_fish)), {
    caughtIds: [77, 912],
    favouriteIds: [912],
  });
});

test("map-page restore loads persisted session into _map_session", () => {
  const env = createContext(
    {},
    {
      sessionStorageInitial: {
        "fishystuff.map.session.v1": JSON.stringify({
          version: 1,
          view: {
            viewMode: "3d",
            camera: {
              centerWorldX: 100,
              centerWorldZ: 200,
              distance: 9000,
            },
          },
          selection: {
            fishId: 820986,
            worldX: 123.5,
            worldZ: -45.25,
            pointKind: "bookmark",
            pointLabel: "Pink Dolphin",
          },
        }),
      },
    },
  );
  const signals = defaultSignals();

  env.window.__fishystuffMap.restore(signals);

  assert.deepEqual(signals._map_session, {
    view: {
      viewMode: "3d",
      camera: {
        centerWorldX: 100,
        centerWorldZ: 200,
        distance: 9000,
      },
    },
    selection: {
      fishId: 820986,
      worldX: 123.5,
      worldZ: -45.25,
      pointKind: "bookmark",
      pointLabel: "Pink Dolphin",
    },
  });
});

test("map-page restore loads persisted window ui into _map_ui", () => {
  const env = createContext({
    "fishystuff.map.window_ui.v1": JSON.stringify({
      windowUi: {
        search: { open: false, collapsed: false, x: null, y: null },
        settings: { open: true, collapsed: false, x: null, y: null, autoAdjustView: true },
        zoneInfo: { open: true, collapsed: false, x: null, y: null, tab: "zone_info" },
        layers: { open: true, collapsed: false, x: null, y: null },
        bookmarks: { open: false, collapsed: false, x: null, y: null },
      },
      layers: {
        expandedLayerIds: ["terrain"],
      },
      bridgedUi: {
        diagnosticsOpen: true,
        showPoints: false,
        showPointIcons: false,
        viewMode: "2d",
        pointIconScale: 1.5,
      },
      bridgedFilters: {
        fishIds: [77, 91],
        zoneRgbs: [12615551, 3972668],
        semanticFieldIdsByLayer: {
          region_groups: [295],
        },
        fishFilterTerms: ["favourite", "missing"],
        patchId: null,
        fromPatchId: "2026-02-26",
        toPatchId: "2026-03-12",
        layerIdsVisible: ["zones", "terrain"],
        layerIdsOrdered: ["zones", "terrain", "minimap"],
        layerOpacities: { terrain: 0.35 },
        layerClipMasks: { terrain: "zones" },
        layerWaypointConnectionsVisible: { terrain: true },
        layerWaypointLabelsVisible: { terrain: false },
        layerPointIconsVisible: { terrain: true },
        layerPointIconScales: { terrain: 1.5 },
      },
      search: {
        query: "velia",
      },
    }),
  });
  const signals = defaultSignals();

  env.window.__fishystuffMap.restore(signals);

  assert.equal(signals._map_ui.windowUi.search.open, false);
  assert.equal(signals._map_ui.windowUi.zoneInfo.tab, "zone_info");
  assert.deepEqual(signals._map_ui.layers.expandedLayerIds, ["terrain"]);
  assert.equal(signals._map_ui.search.query, "velia");
  assert.equal(signals._map_bridged.ui.diagnosticsOpen, true);
  assert.equal(signals._map_bridged.ui.showPoints, false);
  assert.equal(signals._map_bridged.ui.showPointIcons, false);
  assert.equal(signals._map_bridged.ui.pointIconScale, 1.5);
  assert.equal(signals._map_bridged.ui.viewMode, "2d");
  assert.equal(signals._map_bridged.filters.fromPatchId, "2026-02-26");
  assert.equal(signals._map_bridged.filters.toPatchId, "2026-03-12");
  assert.deepEqual(signals._map_bridged.filters.fishIds, [77, 91]);
  assert.deepEqual(signals._map_bridged.filters.zoneRgbs, [12615551, 3972668]);
  assert.deepEqual(signals._map_bridged.filters.semanticFieldIdsByLayer, {
    region_groups: [295],
  });
  assert.deepEqual(signals._map_bridged.filters.fishFilterTerms, ["favourite", "missing"]);
  assert.deepEqual(signals._map_bridged.filters.layerIdsVisible, ["zones", "terrain"]);
  assert.deepEqual(signals._map_bridged.filters.layerIdsOrdered, ["zones", "terrain", "minimap"]);
  assert.deepEqual(signals._map_bridged.filters.layerOpacities, { terrain: 0.35 });
  assert.deepEqual(signals._map_bridged.filters.layerClipMasks, { terrain: "zones" });
  assert.deepEqual(signals._map_bridged.filters.layerWaypointConnectionsVisible, { terrain: true });
  assert.deepEqual(signals._map_bridged.filters.layerWaypointLabelsVisible, { terrain: false });
  assert.deepEqual(signals._map_bridged.filters.layerPointIconsVisible, { terrain: true });
  assert.deepEqual(signals._map_bridged.filters.layerPointIconScales, { terrain: 1.5 });
  assert.equal("windowUi" in signals, false);
});

test("map-page restore clears the legacy bridge prefs key", () => {
  const env = createContext({
    "fishystuff.map.prefs.v1": JSON.stringify({
      version: 1,
      filters: {},
    }),
  });
  const signals = defaultSignals();

  env.window.__fishystuffMap.restore(signals);

  assert.equal(env.localStorage.getItem("fishystuff.map.prefs.v1"), null);
});

test("map-page restore does not let stored filters override query-owned input state", () => {
  const env = createContext(
    {
      "fishystuff.map.window_ui.v1": JSON.stringify({
        windowUi: {
          search: { open: false, collapsed: false, x: null, y: null },
          settings: { open: true, collapsed: false, x: null, y: null, autoAdjustView: true },
          zoneInfo: { open: true, collapsed: false, x: null, y: null, tab: "" },
          layers: { open: true, collapsed: false, x: null, y: null },
          bookmarks: { open: false, collapsed: false, x: null, y: null },
        },
        inputUi: {
          diagnosticsOpen: true,
          legendOpen: true,
          leftPanelOpen: false,
          showPoints: false,
          showPointIcons: false,
          viewMode: "2d",
          pointIconScale: 1.5,
        },
        inputFilters: {
          fishIds: [77],
          zoneRgbs: [],
          semanticFieldIdsByLayer: {},
          fishFilterTerms: ["favourite"],
          searchText: "stored-search",
          fromPatchId: "stored-from",
          toPatchId: "stored-to",
          layerIdsVisible: ["terrain"],
          layerIdsOrdered: ["terrain", "minimap"],
          layerOpacities: { terrain: 0.35 },
          layerClipMasks: { terrain: "zones" },
          layerWaypointConnectionsVisible: {},
          layerWaypointLabelsVisible: {},
          layerPointIconsVisible: {},
          layerPointIconScales: {},
        },
      }),
    },
    {
      locationHref:
        "https://fishystuff.fish/map/?fish=91&fishTerms=missing&search=url-search&fromPatch=url-from&toPatch=url-to&layers=zones,terrain&diagnostics=true&legend=true",
    },
  );
  const signals = defaultSignals();

  env.window.__fishystuffMap.restore(signals);

  assert.deepEqual(signals._map_bridged.filters.fishIds, []);
  assert.deepEqual(signals._map_bridged.filters.fishFilterTerms, []);
  assert.equal(signals._map_ui.search.query, "");
  assert.equal(signals._map_bridged.filters.fromPatchId, null);
  assert.equal(signals._map_bridged.filters.toPatchId, null);
  assert.deepEqual(signals._map_bridged.filters.layerIdsVisible, DEFAULT_ENABLED_LAYER_IDS);
  assert.equal(signals._map_bridged.ui.diagnosticsOpen, false);
  assert.equal(signals._map_bridged.ui.showPoints, false);
  assert.equal(signals._map_bridged.ui.showPointIcons, false);
  assert.equal(signals._map_bridged.ui.pointIconScale, 1.5);
});

test("map-page persists bookmark signal patches through the Datastar patch event", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffMap.restore(signals);
  env.window.__fishystuffMap.patchSignals({
    _map_bookmarks: {
      entries: [{ id: "bookmark-2", label: "Signal Owned", worldX: 1, worldZ: 2, layerSamples: [] }],
    },
  });
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _map_bookmarks: {
        entries: [{ id: "bookmark-2", label: "Signal Owned", worldX: 1, worldZ: 2, layerSamples: [] }],
      },
    },
  });
  env.flushTimers();

  assert.equal(
    env.localStorage.getItem("fishystuff.map.bookmarks.v1"),
    JSON.stringify(signals._map_bookmarks.entries),
  );
});

test("map-page ignores ephemeral _map_ui patches for persistence", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffMap.restore(signals);
  env.window.__fishystuffMap.patchSignals({
    _map_ui: {
      search: { open: true },
    },
  });
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _map_ui: {
        search: { open: true },
      },
    },
  });
  env.flushTimers();

  assert.equal(env.localStorage.getItem("fishystuff.map.window_ui.v1"), null);
  assert.equal(env.localStorage.getItem("fishystuff.map.bookmarks.v1"), null);
});

test("map-page persists durable _map_ui.windowUi patches", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffMap.restore(signals);
  env.window.__fishystuffMap.patchSignals({
    _map_ui: {
      windowUi: {
        search: { open: false },
      },
    },
  });
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _map_ui: {
        windowUi: {
          search: { open: false },
        },
      },
    },
  });
  env.flushTimers();

  const expected = defaultStoredUiSnapshot();
  expected.windowUi.search.open = false;
  assert.equal(
    env.localStorage.getItem("fishystuff.map.window_ui.v1"),
    JSON.stringify(expected),
  );
});

test("map-page persists durable _map_ui.layers patches", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffMap.restore(signals);
  env.window.__fishystuffMap.patchSignals({
    _map_ui: {
      layers: {
        expandedLayerIds: ["terrain", "resources"],
      },
    },
  });
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _map_ui: {
        layers: {
          expandedLayerIds: ["terrain", "resources"],
        },
      },
    },
  });
  env.flushTimers();

  const expected = defaultStoredUiSnapshot();
  expected.layers.expandedLayerIds = ["terrain", "resources"];
  assert.equal(
    env.localStorage.getItem("fishystuff.map.window_ui.v1"),
    JSON.stringify(expected),
  );
});

test("map-page persists durable _map_bridged diagnostics state", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffMap.restore(signals);
  env.window.__fishystuffMap.patchSignals({
    _map_bridged: {
      ui: {
        diagnosticsOpen: true,
      },
    },
  });
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _map_bridged: {
        ui: {
          diagnosticsOpen: true,
        },
      },
    },
  });
  env.flushTimers();

  const expected = defaultStoredUiSnapshot();
  expected.bridgedUi.diagnosticsOpen = true;
  assert.equal(
    env.localStorage.getItem("fishystuff.map.window_ui.v1"),
    JSON.stringify(expected),
  );
});

test("map-page persists durable bridged layer filter state", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffMap.restore(signals);
  env.window.__fishystuffMap.patchSignals({
    _map_ui: {
      search: {
        query: "velia",
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [77, 91],
        zoneRgbs: [12615551, 3972668],
        semanticFieldIdsByLayer: {
          region_groups: [295],
        },
        fishFilterTerms: ["favourite", "missing"],
        fromPatchId: "2026-02-26",
        toPatchId: "2026-03-12",
        layerIdsVisible: ["zones", "terrain"],
        layerIdsOrdered: ["zones", "terrain", "minimap"],
        layerOpacities: { terrain: 0.35 },
        layerClipMasks: { terrain: "zones" },
        layerWaypointConnectionsVisible: { terrain: true },
        layerWaypointLabelsVisible: { terrain: false },
        layerPointIconsVisible: { terrain: true },
        layerPointIconScales: { terrain: 1.5 },
      },
    },
  });
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _map_ui: {
        search: {
          query: "velia",
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [77, 91],
          zoneRgbs: [12615551, 3972668],
          semanticFieldIdsByLayer: {
            region_groups: [295],
          },
          fishFilterTerms: ["favourite", "missing"],
          fromPatchId: "2026-02-26",
          toPatchId: "2026-03-12",
          layerIdsVisible: ["zones", "terrain"],
          layerIdsOrdered: ["zones", "terrain", "minimap"],
          layerOpacities: { terrain: 0.35 },
          layerClipMasks: { terrain: "zones" },
          layerWaypointConnectionsVisible: { terrain: true },
          layerWaypointLabelsVisible: { terrain: false },
          layerPointIconsVisible: { terrain: true },
          layerPointIconScales: { terrain: 1.5 },
        },
      },
    },
  });
  env.flushTimers();

  const expected = defaultStoredUiSnapshot();
  expected.search.query = "velia";
  expected.bridgedFilters.fishIds = [77, 91];
  expected.bridgedFilters.zoneRgbs = [12615551, 3972668];
  expected.bridgedFilters.semanticFieldIdsByLayer = {
    region_groups: [295],
  };
  expected.bridgedFilters.fishFilterTerms = ["favourite", "missing"];
  expected.bridgedFilters.fromPatchId = "2026-02-26";
  expected.bridgedFilters.toPatchId = "2026-03-12";
  expected.bridgedFilters.layerIdsVisible = ["zones", "terrain"];
  expected.bridgedFilters.layerIdsOrdered = ["zones", "terrain", "minimap"];
  expected.bridgedFilters.layerOpacities = { terrain: 0.35 };
  expected.bridgedFilters.layerClipMasks = { terrain: "zones" };
  expected.bridgedFilters.layerWaypointConnectionsVisible = { terrain: true };
  expected.bridgedFilters.layerWaypointLabelsVisible = { terrain: false };
  expected.bridgedFilters.layerPointIconsVisible = { terrain: true };
  expected.bridgedFilters.layerPointIconScales = { terrain: 1.5 };
  assert.equal(
    env.localStorage.getItem("fishystuff.map.window_ui.v1"),
    JSON.stringify(expected),
  );
});

test("map-page persists durable _map_session state into sessionStorage", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffMap.restore(signals);
  env.window.__fishystuffMap.patchSignals({
    _map_session: {
      view: {
        viewMode: "3d",
        camera: {
          centerWorldX: 10,
          centerWorldZ: 20,
          distance: 7000,
        },
      },
      selection: {
        zoneRgb: 12615551,
        worldX: 321.5,
        worldZ: -654.25,
        pointKind: "clicked",
      },
    },
  });
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _map_session: {
        view: {
          viewMode: "3d",
          camera: {
            centerWorldX: 10,
            centerWorldZ: 20,
            distance: 7000,
          },
        },
        selection: {
          zoneRgb: 12615551,
          worldX: 321.5,
          worldZ: -654.25,
          pointKind: "clicked",
        },
      },
    },
  });
  env.flushTimers();

  assert.equal(
    env.sessionStorage.getItem("fishystuff.map.session.v1"),
    JSON.stringify({
      version: 1,
      view: {
        viewMode: "3d",
        camera: {
          centerWorldX: 10,
          centerWorldZ: 20,
          distance: 7000,
        },
      },
      selection: {
        zoneRgb: 12615551,
        worldX: 321.5,
        worldZ: -654.25,
        pointKind: "clicked",
      },
      filters: {},
    }),
  );
});
