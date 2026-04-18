import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const SOURCE = fs.readFileSync(
  new URL("./user-overlays.js", import.meta.url),
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
}

function createEnv(initialStorage = {}) {
  const localStorage = new MemoryStorage(initialStorage);
  const windowListeners = new Map();
  const window = {
    addEventListener(type, listener) {
      if (!windowListeners.has(type)) {
        windowListeners.set(type, []);
      }
      windowListeners.get(type).push(listener);
    },
    removeEventListener(type, listener) {
      const current = windowListeners.get(type) || [];
      windowListeners.set(
        type,
        current.filter((candidate) => candidate !== listener),
      );
    },
    dispatchEvent(event) {
      for (const listener of windowListeners.get(event.type) || []) {
        listener(event);
      }
    },
  };
  const context = {
    JSON,
    Object,
    Array,
    String,
    Number,
    Boolean,
    RegExp,
    Error,
    Date,
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
  vm.runInNewContext(SOURCE, context, { filename: "user-overlays.js" });
  return {
    context,
    localStorage,
    helper: context.window.__fishystuffUserOverlays,
  };
}

test("user overlays import replaces snapshot and preserves precise raw rates", () => {
  const env = createEnv();

  const snapshot = env.helper.importText(JSON.stringify({
    format: "fishystuff-user-overlay-v2",
    exportedAt: "2026-04-19T12:00:00.000Z",
    overlay: {
      zones: {
        "zone-b": {
          groups: {
            1: { rawRatePercent: "0.000000123456789" },
          },
          items: {
            "item:8473": {
              present: true,
              slotIdx: 2,
              rawRatePercent: "10.000000123456",
              name: "Golden Fish",
              isFish: true,
            },
          },
        },
      },
    },
    priceOverrides: {
      "item:8473": {
        basePrice: "1234.56789",
        tradePriceCurvePercent: "12.3456789",
      },
    },
  }));

  assert.deepEqual(snapshot, {
    overlay: {
      zones: {
        "zone-b": {
          groups: {
            1: { rawRatePercent: 0.000000123456789 },
          },
          items: {
            8473: {
              present: true,
              slotIdx: 2,
              rawRatePercent: 10.000000123456,
              name: "Golden Fish",
              isFish: true,
            },
          },
        },
      },
    },
    priceOverrides: {
      8473: {
        basePrice: 1234.56789,
        tradePriceCurvePercent: 12.3456789,
      },
    },
  });
  assert.deepEqual(
    JSON.parse(env.localStorage.getItem(env.helper.STORAGE_KEY)),
    snapshot,
  );
});

test("user overlays import rejects invalid json and unsupported formats", () => {
  const env = createEnv();

  assert.throws(
    () => env.helper.importText("not json"),
    /Overlay JSON file is not valid JSON/,
  );
  assert.throws(
    () => env.helper.importText(JSON.stringify({ format: "fishystuff-user-overlay-v1" })),
    /Overlay JSON must use fishystuff-user-overlay-v2 format/,
  );
});
