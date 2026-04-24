import { test } from "bun:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const SOURCE = fs.readFileSync(
  new URL("./ui-settings.js", import.meta.url),
  "utf8",
);

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
      return true;
    },
  };
}

function createEnv(initialStorage = {}) {
  const localStorage = new MemoryStorage(initialStorage);
  const window = createEventTarget();
  const context = {
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
    console,
    localStorage,
    window,
    CustomEvent: class CustomEvent {
      constructor(type, options = {}) {
        this.type = type;
        this.detail = options.detail;
      }
    },
    globalThis: null,
  };
  context.globalThis = context;
  vm.runInNewContext(SOURCE, context, { filename: "ui-settings.js" });
  return {
    localStorage,
    helper: context.window.__fishystuffUiSettings,
  };
}

test("ui settings remove clears a nested path and preserves siblings", () => {
  const env = createEnv({
    "fishystuff.ui.settings.v1": JSON.stringify({
      app: {
        theme: {
          selected: "dark",
        },
        density: {
          compact: true,
        },
      },
      map: {
        panel: "layers",
      },
    }),
  });

  env.helper.remove(["app", "theme"]);

  assert.deepEqual(JSON.parse(JSON.stringify(env.helper.snapshot())), {
    app: {
      density: {
        compact: true,
      },
    },
    map: {
      panel: "layers",
    },
  });
});

test("ui settings remove clears storage entirely when the last path is removed", () => {
  const env = createEnv({
    "fishystuff.ui.settings.v1": JSON.stringify({
      app: {
        theme: {
          selected: "dark",
        },
      },
    }),
  });

  env.helper.remove("app.theme");

  assert.deepEqual(JSON.parse(JSON.stringify(env.helper.snapshot())), {});
  assert.equal(env.localStorage.getItem("fishystuff.ui.settings.v1"), null);
});
