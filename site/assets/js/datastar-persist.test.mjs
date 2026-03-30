import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const DATASTAR_PERSIST_SOURCE = fs.readFileSync(new URL("./datastar-persist.js", import.meta.url), "utf8");

function createDocumentStub() {
  const listeners = new Map();
  return {
    addEventListener(type, listener) {
      if (!listeners.has(type)) {
        listeners.set(type, []);
      }
      listeners.get(type).push(listener);
    },
    removeEventListener(type, listener) {
      const current = listeners.get(type) || [];
      listeners.set(
        type,
        current.filter((candidate) => candidate !== listener),
      );
    },
    dispatchEvent(event) {
      for (const listener of listeners.get(event.type) || []) {
        listener(event);
      }
    },
  };
}

function createContext() {
  const document = createDocumentStub();
  let nextTimerId = 1;
  const timers = new Map();
  const context = {
    window: {},
    document,
    console,
    Object,
    Array,
    String,
    RegExp,
    JSON,
    Map,
    Set,
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
  return {
    window: context.window,
    document,
    flushTimers() {
      const pending = Array.from(timers.values());
      timers.clear();
      for (const callback of pending) {
        callback();
      }
    },
  };
}

test("datastar persist helper matches include filters recursively", () => {
  const env = createContext();
  const helper = env.window.__fishystuffDatastarPersist;

  assert.equal(
    helper.patchMatchesSignalFilter(
      { _map_ui: { windowUi: { search: { open: false } } } },
      { include: /^_(?:map_ui|map_bookmarks)(?:\.|$)/ },
    ),
    true,
  );
  assert.equal(
    helper.patchMatchesSignalFilter(
      { _map_runtime: { state: { ready: true } } },
      { include: /^_(?:map_ui|map_bookmarks)(?:\.|$)/ },
    ),
    false,
  );
});

test("datastar persist helper matches non-ephemeral paths with exclude filters", () => {
  const env = createContext();
  const helper = env.window.__fishystuffDatastarPersist;

  assert.equal(
    helper.patchMatchesSignalFilter(
      { search_query: "pink" },
      { exclude: /^_(?:selected|loading)(?:\.|$)/ },
    ),
    true,
  );
  assert.equal(
    helper.patchMatchesSignalFilter(
      { _selected: { fish: 1 } },
      { exclude: /^_(?:selected|loading)(?:\.|$)/ },
    ),
    false,
  );
});

test("datastar persist helper debounces matching patch events", () => {
  const env = createContext();
  const helper = env.window.__fishystuffDatastarPersist;
  let count = 0;
  let ready = false;
  const binding = helper.createDebouncedSignalPatchPersistor({
    target: env.document,
    delayMs: 10,
    isReady() {
      return ready;
    },
    filter: {
      include: /^_map_ui(?:\.|$)/,
    },
    persist() {
      count += 1;
    },
  });

  binding.bind();
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: { _map_ui: { windowUi: { search: { open: false } } } },
  });
  env.flushTimers();
  assert.equal(count, 0);

  ready = true;
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: { _map_runtime: { state: { ready: true } } },
  });
  env.flushTimers();
  assert.equal(count, 0);

  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: { _map_ui: { windowUi: { search: { open: false } } } },
  });
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: { _map_ui: { windowUi: { search: { open: true } } } },
  });
  env.flushTimers();
  assert.equal(count, 1);
});
