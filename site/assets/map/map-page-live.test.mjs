import test from "node:test";
import assert from "node:assert/strict";

import {
  FISHYMAP_LIVE_BOOTSTRAP_REQUEST_EVENT,
  FISHYMAP_LIVE_INIT_EVENT,
  FISHYMAP_LIVE_READY_EVENT,
  createMapPageLive,
} from "./map-page-live.js";

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
    removeEventListener(type, listener) {
      if (!listeners.has(type)) {
        return;
      }
      listeners.set(
        type,
        listeners.get(type).filter((candidate) => candidate !== listener),
      );
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
  const localStorage = new MemoryStorage(localStorageInitial);
  const sessionStorage = new MemoryStorage(options.sessionStorageInitial || {});
  const location = {
    href: options.locationHref || "https://fishystuff.fish/map/",
  };
  const timers = new Map();
  let nextTimerId = 1;
  const globalRef = {
    document,
    location,
    localStorage,
    sessionStorage,
    window: { location, localStorage, sessionStorage },
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

  const pageLive = createMapPageLive({ globalRef });
  pageLive.start();

  return {
    document,
    globalRef,
    localStorage,
    sessionStorage,
    shell,
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
      search: { open: false, query: "", selectedTerms: [] },
      bookmarks: { placing: false, selectedIds: [] },
      layers: {
        expandedLayerIds: [],
        hoverFactsVisibleByLayer: {},
      },
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
        layerClipMasks: { fish_evidence: "zone_mask" },
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

function dispatchLiveInit(env, detail) {
  env.shell.dispatchEvent({
    type: FISHYMAP_LIVE_INIT_EVENT,
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
  let readyDetail = null;
  env.shell.addEventListener(FISHYMAP_LIVE_READY_EVENT, (event) => {
    readyDetail = event.detail;
  });

  dispatchLiveInit(env, signals);
  env.shell.dispatchEvent({ type: FISHYMAP_LIVE_BOOTSTRAP_REQUEST_EVENT });

  assert.equal(typeof readyDetail?.patchSignals, "function");
  assert.deepEqual(signals._map_bookmarks.entries, persistedBookmarks);
  assert.equal(typeof readyDetail?.signalObject, "function");
  assert.equal(readyDetail.signalObject(), signals);
});

test("map-page-live restore loads shared fish state without the site-global helper", () => {
  const env = createContext({
    "fishystuff.fishydex.caught.v1": JSON.stringify({ "912": true, "77": false, "5": 1 }),
    "fishystuff.fishydex.favourites.v1": JSON.stringify(["77", 77, "bad"]),
  });
  const signals = defaultSignals();

  dispatchLiveInit(env, signals);

  assert.deepEqual(signals._shared_fish.caughtIds, [5, 912]);
  assert.deepEqual(signals._shared_fish.favouriteIds, [77]);
});

test("map-page-live exposes direct signal patching on the shell api", () => {
  const env = createContext();
  const signals = defaultSignals();
  let readyDetail = null;
  env.shell.addEventListener(FISHYMAP_LIVE_READY_EVENT, (event) => {
    readyDetail = event.detail;
  });

  dispatchLiveInit(env, signals);
  env.shell.dispatchEvent({ type: FISHYMAP_LIVE_BOOTSTRAP_REQUEST_EVENT });

  readyDetail.patchSignals({
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

test("map-page-live persists durable map signal patches", () => {
  const env = createContext();
  const signals = defaultSignals();
  let readyDetail = null;
  env.shell.addEventListener(FISHYMAP_LIVE_READY_EVENT, (event) => {
    readyDetail = event.detail;
  });

  dispatchLiveInit(env, signals);
  env.shell.dispatchEvent({ type: FISHYMAP_LIVE_BOOTSTRAP_REQUEST_EVENT });

  readyDetail.patchSignals({
    _map_ui: {
      windowUi: {
        search: { open: false, collapsed: true, x: 20, y: 30 },
      },
      layers: {
        hoverFactsVisibleByLayer: {
          regions: {
            origin_region: true,
          },
        },
      },
    },
    _map_bridged: {
      filters: {
        layerIdsVisible: ["bookmarks", "zone_mask"],
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
  assert.deepEqual(storedUi.layers.hoverFactsVisibleByLayer, {
    regions: {
      origin_region: true,
    },
  });
  assert.deepEqual(storedUi.bridgedFilters.layerIdsVisible, ["bookmarks", "zone_mask"]);
});
