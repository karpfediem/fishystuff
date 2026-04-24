import { test } from "bun:test";
import assert from "node:assert/strict";

import { createMapPagePersistController } from "./map-page-persist.js";
import { FISHYMAP_SIGNAL_PATCHED_EVENT } from "./map-signal-patch.js";

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
}

test("map-page persist controller seeds and writes only changed storage payloads", () => {
  const localStorage = new MemoryStorage();
  const sessionStorage = new MemoryStorage();
  const timers = new Map();
  let nextTimerId = 1;
  const snapshot = {
    version: 1,
  };
  const controller = createMapPagePersistController({
    globalRef: {
      localStorage,
      sessionStorage,
      setTimeout(callback) {
        const id = nextTimerId++;
        timers.set(id, callback);
        return id;
      },
      clearTimeout(id) {
        timers.delete(id);
      },
    },
    readSnapshot() {
      return snapshot;
    },
    isReady: () => true,
    createPersistedStateImpl(value) {
      return {
        uiJson: JSON.stringify({ ui: value.version }),
        bookmarksJson: JSON.stringify({ bookmarks: value.version }),
        sessionJson: JSON.stringify({ session: value.version }),
      };
    },
    shouldPersistPatch: () => true,
  });

  assert.equal(controller.seed(snapshot), true);
  assert.equal(controller.handleSignalPatch({ _map_ui: { search: { query: "eel" } } }), true);
  assert.equal(timers.size, 1);

  for (const callback of timers.values()) {
    callback();
  }

  assert.equal(localStorage.getItem("fishystuff.map.window_ui.v1"), null);
  assert.equal(localStorage.getItem("fishystuff.map.bookmarks.v1"), null);
  assert.equal(sessionStorage.getItem("fishystuff.map.session.v1"), null);

  snapshot.version = 2;
  controller.schedulePersist();
  for (const callback of timers.values()) {
    callback();
  }

  assert.equal(localStorage.getItem("fishystuff.map.window_ui.v1"), JSON.stringify({ ui: 2 }));
  assert.equal(
    localStorage.getItem("fishystuff.map.bookmarks.v1"),
    JSON.stringify({ bookmarks: 2 }),
  );
  assert.equal(
    sessionStorage.getItem("fishystuff.map.session.v1"),
    JSON.stringify({ session: 2 }),
  );
});

test("map-page persist controller ignores patches until ready and when filters do not match", () => {
  let ready = false;
  let scheduled = 0;
  const controller = createMapPagePersistController({
    globalRef: {
      setTimeout() {
        scheduled += 1;
        return scheduled;
      },
      clearTimeout() {},
    },
    readSnapshot: () => ({}),
    isReady: () => ready,
    createPersistedStateImpl: () => ({ uiJson: "{}", bookmarksJson: "[]", sessionJson: "{}" }),
    shouldPersistPatch: (patch) => Boolean(patch?._map_ui),
  });

  assert.equal(controller.handleSignalPatch({ _map_ui: { search: { query: "eel" } } }), false);
  ready = true;
  assert.equal(controller.handleSignalPatch({ _map_runtime: { ready: true } }), false);
  assert.equal(controller.handleSignalPatch({ _map_ui: { search: { query: "eel" } } }), true);
  assert.equal(scheduled, 1);
});

test("map-page persist controller can subscribe to shell-local applied patch events", () => {
  let scheduled = 0;
  const listeners = new Map();
  const controller = createMapPagePersistController({
    shell: {
      addEventListener(type, listener) {
        listeners.set(type, listener);
      },
    },
    globalRef: {
      setTimeout() {
        scheduled += 1;
        return scheduled;
      },
      clearTimeout() {},
    },
    readSnapshot: () => ({}),
    isReady: () => true,
    createPersistedStateImpl: () => ({ uiJson: "{}", bookmarksJson: "[]", sessionJson: "{}" }),
    shouldPersistPatch: (patch) => Boolean(patch?._map_ui),
  });

  assert.equal(typeof controller.handleSignalPatch, "function");
  listeners.get(FISHYMAP_SIGNAL_PATCHED_EVENT)?.({
    detail: {
      _map_ui: {
        search: { query: "eel" },
      },
    },
  });

  assert.equal(scheduled, 1);
});

test("map-page persist controller subscribes to shell-local patch events only", () => {
  let scheduled = 0;
  const shellListeners = new Map();
  createMapPagePersistController({
    shell: {
      addEventListener(type, listener) {
        shellListeners.set(type, listener);
      },
    },
    globalRef: {
      setTimeout() {
        scheduled += 1;
        return scheduled;
      },
      clearTimeout() {},
    },
    readSnapshot: () => ({}),
    isReady: () => true,
    createPersistedStateImpl: () => ({ uiJson: "{}", bookmarksJson: "[]", sessionJson: "{}" }),
    shouldPersistPatch: (patch) => Boolean(patch?._map_ui),
  });

  assert.equal(typeof shellListeners.get(FISHYMAP_SIGNAL_PATCHED_EVENT), "function");
  shellListeners.get(FISHYMAP_SIGNAL_PATCHED_EVENT)?.({
    detail: {
      _map_ui: {
        search: { query: "cod" },
      },
    },
  });

  assert.equal(scheduled, 1);
});
