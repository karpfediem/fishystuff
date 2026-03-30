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
const SHARED_FISH_STATE_SOURCE = fs.readFileSync(
  new URL("../shared-fish-state.js", import.meta.url),
  "utf8",
);
const FISHYDEX_SOURCE = fs.readFileSync(new URL("./fishydex.js", import.meta.url), "utf8");

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

class ElementStub {}
class HTMLElementStub extends ElementStub {}
class HTMLImageElementStub extends HTMLElementStub {}
class HTMLButtonElementStub extends HTMLElementStub {}

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
    getElementById() {
      return null;
    },
    querySelector() {
      return null;
    },
    querySelectorAll() {
      return [];
    },
    createElement() {
      return {
        click() {},
        remove() {},
      };
    },
    body: {
      appendChild() {},
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
    navigator: {},
    Blob,
    JSON,
    Object,
    Array,
    String,
    Number,
    Boolean,
    RegExp,
    Error,
    Map,
    Set,
    URL,
    Intl,
    console,
    Element: ElementStub,
    HTMLElement: HTMLElementStub,
    HTMLImageElement: HTMLImageElementStub,
    HTMLButtonElement: HTMLButtonElementStub,
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
  window.prompt = () => null;
  window.requestAnimationFrame = (callback) => callback();
  vm.runInNewContext(DATASTAR_PERSIST_SOURCE, context, { filename: "datastar-persist.js" });
  vm.runInNewContext(SHARED_FISH_STATE_SOURCE, context, { filename: "shared-fish-state.js" });
  vm.runInNewContext(FISHYDEX_SOURCE, context, { filename: "fishydex.js" });
  return {
    window,
    document,
    navigator: context.navigator,
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
    fish: [],
    revision: "",
    count: 0,
    search_query: "",
    caught_filter: "all",
    favourite_filter: false,
    grade_filters: [],
    method_filters: [],
    show_dried: false,
    sort_field: "price",
    sort_direction: "desc",
    _shared_fish: {
      caughtIds: [],
      favouriteIds: [],
    },
    _selected_fish_id: 0,
    _progress_panel_collapsed: false,
    _filter_panel_collapsed: false,
    _loading: true,
    _fishydex_actions: {
      exportCaughtToken: 0,
      importCaughtToken: 0,
      closeDetailsToken: 0,
    },
    _status_message: "",
    _api_error_message: "",
    _api_error_hint: "",
  };
}

test("fishydex restore loads panel collapse state from fishydex ui storage", () => {
  const env = createContext({
    "fishystuff.fishydex.ui.v1": JSON.stringify({
      search_query: "eel",
      _progress_panel_collapsed: true,
      _filter_panel_collapsed: false,
    }),
    "fishystuff.ui.settings.v1": JSON.stringify({
      dex: {
        panels: {
          progress: { collapsed: false },
          filter: { collapsed: true },
        },
      },
    }),
  });
  const signals = defaultSignals();

  env.window.Fishydex.restore(signals);

  assert.equal(signals.search_query, "eel");
  assert.equal(signals._progress_panel_collapsed, true);
  assert.equal(signals._filter_panel_collapsed, false);
});

test("fishydex persists panel collapse state in fishydex ui storage", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.Fishydex.restore(signals);
  Object.assign(signals, {
    _progress_panel_collapsed: true,
    _filter_panel_collapsed: true,
  });
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _progress_panel_collapsed: true,
      _filter_panel_collapsed: true,
    },
  });
  env.flushTimers();

  assert.equal(
    env.localStorage.getItem("fishystuff.fishydex.ui.v1"),
    JSON.stringify({
      search_query: "",
      caught_filter: "all",
      favourite_filter: false,
      grade_filters: [],
      method_filters: [],
      show_dried: false,
      sort_field: "price",
      sort_direction: "desc",
      _progress_panel_collapsed: true,
      _filter_panel_collapsed: true,
    }),
  );
});

test("fishydex export action token copies caught ids and updates status", async () => {
  const env = createContext();
  const signals = defaultSignals();
  let copiedText = "";
  env.navigator.clipboard = {
    writeText(value) {
      copiedText = String(value);
      return Promise.resolve();
    },
  };

  env.window.Fishydex.restore(signals);
  Object.assign(signals, {
    _shared_fish: {
      caughtIds: [8473, 8476],
      favouriteIds: [],
    },
    _fishydex_actions: {
      exportCaughtToken: 1,
      importCaughtToken: 0,
      closeDetailsToken: 0,
    },
  });

  env.window.Fishydex.sync(signals);
  await Promise.resolve();
  await Promise.resolve();
  await new Promise((resolve) => setTimeout(resolve, 0));

  assert.equal(copiedText, JSON.stringify([8473, 8476], null, 2));
  assert.equal(signals._status_message, "Copied 2 caught fish IDs.");
});

test("fishydex import action token updates caught ids from prompt input", () => {
  const env = createContext();
  const signals = defaultSignals();
  env.window.prompt = () => JSON.stringify({ 8473: true, 8476: true });

  env.window.Fishydex.restore(signals);
  Object.assign(signals, {
    _fishydex_actions: {
      exportCaughtToken: 0,
      importCaughtToken: 1,
      closeDetailsToken: 0,
    },
  });

  env.window.Fishydex.sync(signals);

  assert.deepEqual(JSON.parse(JSON.stringify(signals._shared_fish)), {
    caughtIds: [8473, 8476],
    favouriteIds: [],
  });
  assert.equal(signals._status_message, "Imported 2 caught fish IDs.");
});

test("fishydex clears transient feedback on filter signal patches", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.Fishydex.restore(signals);
  Object.assign(signals, {
    _status_message: "Copied 2 caught fish IDs.",
    _api_error_message: "Fish API request failed.",
    _api_error_hint: "Retry later.",
    search_query: "eel",
  });

  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      search_query: "eel",
    },
  });

  assert.equal(signals._status_message, "");
  assert.equal(signals._api_error_message, "");
  assert.equal(signals._api_error_hint, "");
});
