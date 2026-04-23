import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const SOURCE = fs.readFileSync(
  new URL("./user-presets.js", import.meta.url),
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

function createEnv(localStorageValues = {}) {
  const localStorage = new MemoryStorage(localStorageValues);
  const window = createEventTarget();
  let uuidCounter = 0;
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
    Math,
    console,
    localStorage,
    window,
    crypto: {
      randomUUID() {
        uuidCounter += 1;
        return `00000000-0000-4000-8000-${String(uuidCounter).padStart(12, "0")}`;
      },
    },
    CustomEvent: class CustomEvent {
      constructor(type, options = {}) {
        this.type = type;
        this.detail = options.detail;
      }
    },
    globalThis: null,
  };
  context.globalThis = context;
  vm.runInNewContext(SOURCE, context, { filename: "user-presets.js" });
  return {
    helper: context.window.__fishystuffUserPresets,
    localStorage,
    window,
  };
}

test("user presets can create, update, rename, delete, and select collection presets", () => {
  const env = createEnv();
  const helper = env.helper;
  helper.registerCollectionAdapter("calculator-layouts", {
    normalizePayload(payload) {
      return {
        rows: Array.isArray(payload?.rows) ? payload.rows.slice(0, 2) : [],
      };
    },
    defaultPresetName(index) {
      return `Layout ${index}`;
    },
  });

  const created = helper.createPreset("calculator-layouts", {
    payload: {
      rows: [1, 2, 3],
    },
  });

  assert.equal(created.name, "Layout 1");
  assert.deepEqual(created.payload, { rows: [1, 2] });
  assert.equal(helper.selectedPresetId("calculator-layouts"), created.id);

  const updated = helper.updatePreset("calculator-layouts", created.id, {
    name: "Velia",
    payload: {
      rows: [7],
    },
  });
  assert.equal(updated.name, "Velia");
  assert.deepEqual(updated.payload, { rows: [7] });

  const renamed = helper.renamePreset("calculator-layouts", created.id, "Harpoon");
  assert.equal(renamed.name, "Harpoon");

  helper.setSelectedPresetId("calculator-layouts", "");
  assert.equal(helper.selectedPresetId("calculator-layouts"), "");

  helper.deletePreset("calculator-layouts", created.id);
  assert.deepEqual(helper.collection("calculator-layouts"), {
    selectedPresetId: "",
    presets: [],
  });
});

test("user presets export and import collection payloads while preserving selection", () => {
  const env = createEnv();
  const helper = env.helper;
  helper.registerCollectionAdapter("calculator-layouts", {
    normalizePayload(payload) {
      return {
        pinned_layout: Array.isArray(payload?.pinned_layout) ? payload.pinned_layout : [],
      };
    },
  });
  const alpha = helper.createPreset("calculator-layouts", {
    name: "Alpha",
    payload: {
      pinned_layout: [[["overview"]]],
    },
  });
  const beta = helper.createPreset("calculator-layouts", {
    name: "Beta",
    payload: {
      pinned_layout: [[["zone"]]],
    },
  });
  helper.setSelectedPresetId("calculator-layouts", beta.id);

  const exported = helper.exportCollectionPayload("calculator-layouts");
  const importedEnv = createEnv();
  const importedHelper = importedEnv.helper;
  importedHelper.registerCollectionAdapter("calculator-layouts", {
    normalizePayload(payload) {
      return {
        pinned_layout: Array.isArray(payload?.pinned_layout) ? payload.pinned_layout : [],
      };
    },
  });
  const result = importedHelper.importCollectionPayload("calculator-layouts", exported);

  assert.deepEqual(importedHelper.presets("calculator-layouts").map((preset) => preset.name), [
    "Alpha",
    "Beta",
  ]);
  assert.equal(importedHelper.selectedPresetId("calculator-layouts"), result.selectedPresetId);
  assert.equal(
    importedHelper.selectedPreset("calculator-layouts")?.name,
    "Beta",
  );
  assert.equal(result.presetIds.length, 2);
  assert.ok(result.presetIds.every(Boolean));
  assert.deepEqual(Array.from(result.presetIds), [alpha.id, beta.id]);
});

test("user presets can activate a preset through the registered adapter", () => {
  const env = createEnv();
  const helper = env.helper;
  const applied = [];
  helper.registerCollectionAdapter("calculator-layouts", {
    normalizePayload(payload) {
      return {
        row: Number.parseInt(payload?.row ?? 0, 10) || 0,
      };
    },
    apply(payload) {
      applied.push(payload);
      return payload;
    },
  });
  const preset = helper.createPreset("calculator-layouts", {
    name: "Row Two",
    payload: {
      row: "2",
    },
    select: false,
  });

  const activated = helper.activatePreset("calculator-layouts", preset.id);

  assert.equal(activated?.id, preset.id);
  assert.deepEqual(applied, [{ row: 2 }]);
  assert.equal(helper.selectedPresetId("calculator-layouts"), preset.id);
});

test("user presets snapshot reloads from local storage changes", () => {
  const env = createEnv({
    "fishystuff.user-presets.v1": JSON.stringify({
      collections: {
        "calculator-layouts": {
          selectedPresetId: "preset_a",
          presets: [
            {
              id: "preset_a",
              name: "Alpha",
              payload: {
                pinned_layout: [],
              },
            },
          ],
        },
      },
    }),
  });

  assert.equal(env.helper.selectedPreset("calculator-layouts")?.name, "Alpha");

  env.localStorage.setItem("fishystuff.user-presets.v1", JSON.stringify({
    collections: {
      "calculator-layouts": {
        selectedPresetId: "",
        presets: [
          {
            id: "preset_b",
            name: "Beta",
            payload: {
              pinned_layout: [[["overview"]]],
            },
          },
        ],
      },
    },
  }));
  env.window.dispatchEvent({
    type: "storage",
    key: "fishystuff.user-presets.v1",
  });

  assert.equal(env.helper.selectedPresetId("calculator-layouts"), "");
  assert.equal(env.helper.presets("calculator-layouts")[0]?.name, "Beta");
});
