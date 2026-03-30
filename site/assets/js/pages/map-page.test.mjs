import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

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
    _map_input: {},
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
