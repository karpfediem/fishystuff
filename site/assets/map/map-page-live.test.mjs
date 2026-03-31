import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const MAP_PAGE_LIVE_SOURCE = fs.readFileSync(new URL("./map-page-live.js", import.meta.url), "utf8");
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

function createEventTarget() {
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
      return true;
    },
  };
}

function createDocumentStub(shell = null) {
  return {
    ...createEventTarget(),
    getElementById(id) {
      if (id === "map-page-shell") {
        return shell;
      }
      return null;
    },
  };
}

function createContext(localStorageInitial = {}, options = {}) {
  const shell = createEventTarget();
  shell.id = "map-page-shell";
  const document = createDocumentStub(shell);
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
    CustomEvent: class CustomEvent {
      constructor(type, options = {}) {
        this.type = type;
        this.detail = options.detail;
        this.bubbles = options.bubbles === true;
      }
    },
  };
  context.globalThis = context;
  window.location = location;
  window.localStorage = localStorage;
  window.sessionStorage = sessionStorage;
  vm.runInNewContext(MAP_PAGE_LIVE_SOURCE, context, { filename: "map-page-live.js" });
  return {
    window,
    document,
    shell,
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

function dispatchShellPatch(env, detail) {
  env.shell.dispatchEvent({
    type: "fishymap-signals-patch",
    detail,
  });
}

test("map-page-live restore loads persisted bookmark entries into Datastar signals", () => {
  const persistedBookmarks = [
    { id: "bookmark:1", label: "Alpha", worldX: 10, worldZ: 20, note: "" },
  ];
  const env = createContext({
    "fishystuff.map.bookmarks.v1": JSON.stringify(persistedBookmarks),
  });
  const signals = defaultSignals();

  env.window.__fishystuffMapLiveRestore(signals);

  assert.deepEqual(signals._map_bookmarks.entries, persistedBookmarks);
  assert.equal(typeof env.shell.__fishystuffMapPage?.signalObject, "function");
  assert.equal(env.shell.__fishystuffMapPage.signalObject(), signals);
});

test("map-page-live applies fishymap shell patches into the live signal graph", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffMapLiveRestore(signals);

  dispatchShellPatch(env, {
    _map_ui: {
      search: {
        query: "tuna",
      },
    },
    _map_bookmarks: {
      entries: [{ id: "bookmark:2", label: "Beta", worldX: 11, worldZ: 22 }],
    },
  });

  assert.equal(signals._map_ui.search.query, "tuna");
  assert.deepEqual(signals._map_bookmarks.entries, [
    { id: "bookmark:2", label: "Beta", worldX: 11, worldZ: 22 },
  ]);
});

test("map-page-live re-emits shell patches as datastar signal patches", () => {
  const env = createContext();
  const signals = defaultSignals();
  const received = [];
  const shellReceived = [];

  env.window.__fishystuffMapLiveRestore(signals);
  env.document.addEventListener("datastar-signal-patch", (event) => {
    received.push(event.detail);
  });
  env.shell.addEventListener("fishymap:datastar-signal-patch", (event) => {
    shellReceived.push(event.detail);
  });

  dispatchShellPatch(env, {
    _map_bridged: {
      filters: {
        layerIdsOrdered: ["minimap", "fish_evidence"],
      },
    },
  });

  assert.deepEqual(received, [
    {
      _map_bridged: {
        filters: {
          layerIdsOrdered: ["minimap", "fish_evidence"],
        },
      },
    },
  ]);
  assert.deepEqual(shellReceived, received);
});

test("map-page-live persists durable map signal patches", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffMapLiveRestore(signals);

  dispatchShellPatch(env, {
    _map_ui: {
      windowUi: {
        search: { open: false, collapsed: true, x: 20, y: 30 },
      },
    },
    _map_bridged: {
      filters: {
        layerIdsVisible: ["bookmarks", "zone_mask"],
      },
    },
  });
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _map_ui: {
        windowUi: {
          search: { open: false, collapsed: true, x: 20, y: 30 },
        },
      },
      _map_bridged: {
        filters: {
          layerIdsVisible: ["bookmarks", "zone_mask"],
        },
      },
    },
  });
  env.flushTimers();

  const storedUi = JSON.parse(env.localStorage.getItem("fishystuff.map.window_ui.v1"));
  assert.deepEqual(storedUi.windowUi.search, {
    open: false,
    collapsed: true,
    x: 20,
    y: 30,
  });
  assert.deepEqual(storedUi.bridgedFilters.layerIdsVisible, ["bookmarks", "zone_mask"]);
});
