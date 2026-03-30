import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const DATASTAR_PERSIST_SOURCE = fs.readFileSync(
  new URL("../datastar-persist.js", import.meta.url),
  "utf8",
);
const MAP_PAGE_SOURCE = fs.readFileSync(new URL("./map-page.js", import.meta.url), "utf8");

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

function createContext(localStorageInitial = {}) {
  const document = createDocumentStub();
  const window = {};
  const localStorage = new MemoryStorage(localStorageInitial);
  const timers = new Map();
  let nextTimerId = 1;
  const context = {
    window,
    document,
    localStorage,
    JSON,
    Object,
    Array,
    String,
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
  vm.runInNewContext(DATASTAR_PERSIST_SOURCE, context, { filename: "datastar-persist.js" });
  vm.runInNewContext(MAP_PAGE_SOURCE, context, { filename: "map-page.js" });
  return {
    window,
    document,
    localStorage,
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
      search: { open: false },
      bookmarks: { placing: false, selectedIds: [] },
    },
    _map_bookmarks: {
      entries: [],
    },
    _map_input: {
      filters: {
        searchText: "",
        fromPatchId: null,
        toPatchId: null,
        layerIdsVisible: [],
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
        legendOpen: false,
        leftPanelOpen: true,
        showPoints: true,
        showPointIcons: true,
        pointIconScale: 1,
      },
    },
    _map_runtime: {},
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
      inputUi: {
        diagnosticsOpen: true,
        legendOpen: true,
        leftPanelOpen: false,
        showPoints: false,
        showPointIcons: false,
        pointIconScale: 1.5,
      },
      inputFilters: {
        searchText: "velia",
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
    }),
  });
  const signals = defaultSignals();

  env.window.__fishystuffMap.restore(signals);

  assert.equal(signals._map_ui.windowUi.search.open, false);
  assert.equal(signals._map_ui.windowUi.zoneInfo.tab, "zone_info");
  assert.equal(signals._map_input.ui.diagnosticsOpen, true);
  assert.equal(signals._map_input.ui.legendOpen, true);
  assert.equal(signals._map_input.ui.leftPanelOpen, false);
  assert.equal(signals._map_input.ui.showPoints, false);
  assert.equal(signals._map_input.ui.showPointIcons, false);
  assert.equal(signals._map_input.ui.pointIconScale, 1.5);
  assert.equal(signals._map_input.filters.searchText, "velia");
  assert.equal(signals._map_input.filters.fromPatchId, "2026-02-26");
  assert.equal(signals._map_input.filters.toPatchId, "2026-03-12");
  assert.deepEqual(signals._map_input.filters.layerIdsVisible, ["zones", "terrain"]);
  assert.deepEqual(signals._map_input.filters.layerIdsOrdered, ["zones", "terrain", "minimap"]);
  assert.deepEqual(signals._map_input.filters.layerOpacities, { terrain: 0.35 });
  assert.deepEqual(signals._map_input.filters.layerClipMasks, { terrain: "zones" });
  assert.deepEqual(signals._map_input.filters.layerWaypointConnectionsVisible, { terrain: true });
  assert.deepEqual(signals._map_input.filters.layerWaypointLabelsVisible, { terrain: false });
  assert.deepEqual(signals._map_input.filters.layerPointIconsVisible, { terrain: true });
  assert.deepEqual(signals._map_input.filters.layerPointIconScales, { terrain: 1.5 });
  assert.equal("windowUi" in signals, false);
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

  assert.equal(
    env.localStorage.getItem("fishystuff.map.window_ui.v1"),
    JSON.stringify({
      windowUi: {
        search: { open: false, collapsed: false, x: null, y: null },
        settings: { open: true, collapsed: false, x: null, y: null, autoAdjustView: true },
        zoneInfo: { open: true, collapsed: false, x: null, y: null, tab: "" },
        layers: { open: true, collapsed: false, x: null, y: null },
        bookmarks: { open: false, collapsed: false, x: null, y: null },
      },
      inputUi: {
        diagnosticsOpen: false,
        legendOpen: false,
        leftPanelOpen: true,
        showPoints: true,
        showPointIcons: true,
        pointIconScale: 1,
      },
      inputFilters: {
        searchText: "",
        fromPatchId: null,
        toPatchId: null,
        layerIdsVisible: [],
        layerIdsOrdered: [],
        layerOpacities: {},
        layerClipMasks: {},
        layerWaypointConnectionsVisible: {},
        layerWaypointLabelsVisible: {},
        layerPointIconsVisible: {},
        layerPointIconScales: {},
      },
    }),
  );
});

test("map-page persists durable _map_input diagnostics state", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffMap.restore(signals);
  env.window.__fishystuffMap.patchSignals({
    _map_input: {
      ui: {
        diagnosticsOpen: true,
      },
    },
  });
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _map_input: {
        ui: {
          diagnosticsOpen: true,
        },
      },
    },
  });
  env.flushTimers();

  assert.equal(
    env.localStorage.getItem("fishystuff.map.window_ui.v1"),
    JSON.stringify({
      windowUi: {
        search: { open: true, collapsed: false, x: null, y: null },
        settings: { open: true, collapsed: false, x: null, y: null, autoAdjustView: true },
        zoneInfo: { open: true, collapsed: false, x: null, y: null, tab: "" },
        layers: { open: true, collapsed: false, x: null, y: null },
        bookmarks: { open: false, collapsed: false, x: null, y: null },
      },
      inputUi: {
        diagnosticsOpen: true,
        legendOpen: false,
        leftPanelOpen: true,
        showPoints: true,
        showPointIcons: true,
        pointIconScale: 1,
      },
      inputFilters: {
        searchText: "",
        fromPatchId: null,
        toPatchId: null,
        layerIdsVisible: [],
        layerIdsOrdered: [],
        layerOpacities: {},
        layerClipMasks: {},
        layerWaypointConnectionsVisible: {},
        layerWaypointLabelsVisible: {},
        layerPointIconsVisible: {},
        layerPointIconScales: {},
      },
    }),
  );
});

test("map-page persists durable _map_input filter state", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffMap.restore(signals);
  env.window.__fishystuffMap.patchSignals({
    _map_input: {
      filters: {
        searchText: "velia",
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
      _map_input: {
        filters: {
          searchText: "velia",
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

  assert.equal(
    env.localStorage.getItem("fishystuff.map.window_ui.v1"),
    JSON.stringify({
      windowUi: {
        search: { open: true, collapsed: false, x: null, y: null },
        settings: { open: true, collapsed: false, x: null, y: null, autoAdjustView: true },
        zoneInfo: { open: true, collapsed: false, x: null, y: null, tab: "" },
        layers: { open: true, collapsed: false, x: null, y: null },
        bookmarks: { open: false, collapsed: false, x: null, y: null },
      },
      inputUi: {
        diagnosticsOpen: false,
        legendOpen: false,
        leftPanelOpen: true,
        showPoints: true,
        showPointIcons: true,
        pointIconScale: 1,
      },
      inputFilters: {
        searchText: "velia",
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
    }),
  );
});
