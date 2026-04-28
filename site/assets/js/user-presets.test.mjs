import { test } from "bun:test";
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
    selectedFixedId: "",
    current: null,
    presets: [],
  });
});

test("user presets export and import collection payloads while preserving selection", () => {
  const env = createEnv();
  const helper = env.helper;
  helper.registerCollectionAdapter("calculator-layouts", {
    normalizePayload(payload) {
      return {
        custom_layout: Array.isArray(payload?.custom_layout) ? payload.custom_layout : [],
      };
    },
  });
  const alpha = helper.createPreset("calculator-layouts", {
    name: "Alpha",
    payload: {
      custom_layout: [[["overview"]]],
    },
  });
  const beta = helper.createPreset("calculator-layouts", {
    name: "Beta",
    payload: {
      custom_layout: [[["zone"]]],
    },
  });
  helper.setSelectedPresetId("calculator-layouts", beta.id);

  const exported = helper.exportCollectionPayload("calculator-layouts");
  const importedEnv = createEnv();
  const importedHelper = importedEnv.helper;
  importedHelper.registerCollectionAdapter("calculator-layouts", {
    normalizePayload(payload) {
      return {
        custom_layout: Array.isArray(payload?.custom_layout) ? payload.custom_layout : [],
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

test("user presets can sync selected preset to the current payload", () => {
  const env = createEnv();
  const helper = env.helper;
  let currentPayload = { row: 0 };
  helper.registerCollectionAdapter("calculator-layouts", {
    normalizePayload(payload) {
      return {
        row: Number.parseInt(payload?.row ?? 0, 10) || 0,
      };
    },
    capture() {
      return currentPayload;
    },
  });
  const preset = helper.createPreset("calculator-layouts", {
    name: "Row Two",
    payload: { row: 2 },
    select: false,
  });

  currentPayload = { row: 2 };
  const matched = helper.syncSelectedPresetToCurrent("calculator-layouts");

  assert.equal(matched?.id, preset.id);
  assert.equal(helper.selectedPresetId("calculator-layouts"), preset.id);

  currentPayload = { row: 9 };
  const unmatched = helper.syncSelectedPresetToCurrent("calculator-layouts");

  assert.equal(unmatched, null);
  assert.equal(helper.selectedPresetId("calculator-layouts"), "");
});

test("user presets track modified current state from a fixed preset without creating a saved preset", () => {
  const env = createEnv();
  const helper = env.helper;
  let currentPayload = {
    row: 0,
  };
  helper.registerCollectionAdapter("calculator-layouts", {
    normalizePayload(payload) {
      return {
        row: Number.parseInt(payload?.row ?? 0, 10) || 0,
      };
    },
    fixedPresets() {
      return [{
        id: "default",
        name: "Default",
        payload: {
          row: 0,
        },
      }];
    },
    capture() {
      return currentPayload;
    },
    defaultPresetName(index) {
      return `Layout ${index}`;
    },
  });

  const fixed = helper.ensurePersistedSelection("calculator-layouts");
  assert.equal(fixed.kind, "fixed");
  assert.equal(fixed.action, "matched-fixed");
  assert.equal(helper.selectedPresetId("calculator-layouts"), "");
  assert.equal(helper.selectedFixedId("calculator-layouts"), "default");
  assert.equal(helper.current("calculator-layouts"), null);

  currentPayload = {
    row: 2,
  };
  const tracked = helper.ensurePersistedSelection("calculator-layouts");

  assert.equal(tracked.action, "created-current");
  assert.equal(tracked.kind, "current");
  assert.deepEqual(tracked.source, { kind: "fixed", id: "default" });
  assert.deepEqual(tracked.current.payload, { row: 2 });
  assert.equal(helper.presets("calculator-layouts").length, 0);
  assert.equal(helper.selectedPresetId("calculator-layouts"), "");
  assert.equal(helper.selectedFixedId("calculator-layouts"), "default");
});

test("user presets do not emit Datastar updates when tracking unchanged payloads", () => {
  const env = createEnv();
  const helper = env.helper;
  let currentPayload = {
    row: 0,
  };
  helper.registerCollectionAdapter("calculator-layouts", {
    normalizePayload(payload) {
      return {
        row: Number.parseInt(payload?.row ?? 0, 10) || 0,
      };
    },
    fixedPresets() {
      return [{
        id: "default",
        name: "Default",
        payload: {
          row: 0,
        },
      }];
    },
    capture() {
      return currentPayload;
    },
  });
  const signals = {};
  helper.bindDatastar(signals);
  const initialMatch = helper.trackCurrentPayload("calculator-layouts");
  assert.equal(initialMatch.action, "matched-fixed");
  const matchedVersion = signals._user_presets.version;

  const repeatedMatch = helper.trackCurrentPayload("calculator-layouts");

  assert.equal(repeatedMatch.action, "none");
  assert.equal(signals._user_presets.version, matchedVersion);

  currentPayload = {
    row: 2,
  };
  const createdCurrent = helper.trackCurrentPayload("calculator-layouts");
  assert.equal(createdCurrent.action, "created-current");
  const currentVersion = signals._user_presets.version;

  const repeatedCurrent = helper.trackCurrentPayload("calculator-layouts");

  assert.equal(repeatedCurrent.action, "none");
  assert.equal(signals._user_presets.version, currentVersion);
});

test("user presets do not reassign unchanged bound Datastar snapshots", () => {
  const env = createEnv();
  const helper = env.helper;
  let currentPayload = {
    row: 0,
  };
  helper.registerCollectionAdapter("calculator-layouts", {
    normalizePayload(payload) {
      return {
        row: Number.parseInt(payload?.row ?? 0, 10) || 0,
      };
    },
    fixedPresets() {
      return [{
        id: "default",
        name: "Default",
        payload: {
          row: 0,
        },
      }];
    },
    capture() {
      return currentPayload;
    },
  });
  const assignmentProps = [];
  const signals = new Proxy({}, {
    set(target, property, value) {
      assignmentProps.push(String(property));
      target[property] = value;
      return true;
    },
  });
  helper.bindDatastar(signals);
  helper.trackCurrentPayload("calculator-layouts");
  const assignmentCount = assignmentProps.filter((property) => property === "_user_presets").length;

  helper.trackCurrentPayload("calculator-layouts");
  currentPayload = { row: 0 };
  helper.trackCurrentPayload("calculator-layouts");

  assert.equal(
    assignmentProps.filter((property) => property === "_user_presets").length,
    assignmentCount,
  );
});

test("user presets use adapter payload equality when matching fixed presets", () => {
  const env = createEnv();
  const helper = env.helper;
  let currentPayload = {
    row: 0,
    runtimeOnly: "initial-camera",
  };
  helper.registerCollectionAdapter("map-presets", {
    normalizePayload(payload) {
      return {
        row: Number.parseInt(payload?.row ?? 0, 10) || 0,
        runtimeOnly: String(payload?.runtimeOnly ?? ""),
      };
    },
    payloadsEqual(left, right) {
      return left.row === right.row;
    },
    fixedPresets() {
      return [{
        id: "default",
        name: "Default",
        payload: {
          row: 0,
          runtimeOnly: "",
        },
      }];
    },
    capture() {
      return currentPayload;
    },
  });

  const fixed = helper.ensurePersistedSelection("map-presets");

  assert.equal(fixed.kind, "fixed");
  assert.equal(fixed.action, "matched-fixed");
  assert.equal(helper.selectedFixedId("map-presets"), "default");
  assert.equal(helper.current("map-presets"), null);
});

test("user presets do not apply a source through the adapter when the captured payload already matches", () => {
  const env = createEnv();
  const helper = env.helper;
  let applyCount = 0;
  helper.registerCollectionAdapter("map-presets", {
    normalizePayload(payload) {
      return {
        row: Number.parseInt(payload?.row ?? 0, 10) || 0,
        camera: payload?.camera && typeof payload.camera === "object" ? { ...payload.camera } : {},
      };
    },
    payloadsEqual(left, right) {
      return left.row === right.row;
    },
    fixedPresets() {
      return [{
        id: "default",
        name: "Default",
        payload: {
          row: 0,
          camera: {},
        },
      }];
    },
    capture() {
      return {
        row: 0,
        camera: { zoom: 3 },
      };
    },
    apply() {
      applyCount += 1;
      return {
        row: 0,
        camera: {},
      };
    },
  });

  const activated = helper.activateFixedPreset("map-presets", "default");

  assert.equal(activated?.id, "default");
  assert.equal(applyCount, 0);
  assert.equal(helper.selectedFixedId("map-presets"), "default");
  assert.equal(helper.current("map-presets"), null);
});

test("user presets keep selected preset immutable until current changes are explicitly saved", () => {
  const env = createEnv();
  const helper = env.helper;
  let currentPayload = {
    row: 1,
  };
  helper.registerCollectionAdapter("calculator-layouts", {
    normalizePayload(payload) {
      return {
        row: Number.parseInt(payload?.row ?? 0, 10) || 0,
      };
    },
    capture() {
      return currentPayload;
    },
  });
  const preset = helper.createPreset("calculator-layouts", {
    name: "Layout 1",
    payload: {
      row: 1,
    },
  });

  currentPayload = {
    row: 4,
  };
  const tracked = helper.ensurePersistedSelection("calculator-layouts");

  assert.equal(tracked.action, "created-current");
  assert.equal(tracked.kind, "current");
  assert.deepEqual(tracked.current.payload, { row: 4 });
  assert.equal(helper.selectedPresetId("calculator-layouts"), preset.id);
  assert.deepEqual(helper.preset("calculator-layouts", preset.id).payload, { row: 1 });

  const saved = helper.saveCurrentToSelectedPreset("calculator-layouts");

  assert.equal(saved.id, preset.id);
  assert.deepEqual(saved.payload, { row: 4 });
  assert.equal(helper.current("calculator-layouts"), null);
});

test("user presets do not clear current state when source activation does not change the captured payload", () => {
  const env = createEnv();
  const helper = env.helper;
  let currentPayload = {
    row: 1,
  };
  helper.registerCollectionAdapter("calculator-layouts", {
    normalizePayload(payload) {
      return {
        row: Number.parseInt(payload?.row ?? 0, 10) || 0,
      };
    },
    capture() {
      return currentPayload;
    },
    apply() {
      return null;
    },
  });
  const preset = helper.createPreset("calculator-layouts", {
    name: "Layout 1",
    payload: {
      row: 1,
    },
  });
  currentPayload = {
    row: 4,
  };
  helper.trackCurrentPayload("calculator-layouts");

  helper.activatePreset("calculator-layouts", preset.id);

  assert.deepEqual(helper.current("calculator-layouts")?.payload, { row: 4 });
  assert.deepEqual(helper.preset("calculator-layouts", preset.id).payload, { row: 1 });
  assert.equal(helper.selectedPresetId("calculator-layouts"), preset.id);
});

test("user presets can select saved presets before their page adapter is loaded", () => {
  const env = createEnv();
  const helper = env.helper;
  const preset = helper.createPreset("map-presets", {
    name: "Night map",
    payload: { row: 1 },
    select: false,
  });
  helper.trackCurrentPayload("map-presets", {
    payload: { row: 2 },
    origin: { kind: "fixed", id: "default" },
  });

  const activated = helper.activatePreset("map-presets", preset.id);

  assert.equal(activated.id, preset.id);
  assert.equal(helper.selectedPresetId("map-presets"), preset.id);
  assert.equal(helper.selectedFixedId("map-presets"), "");
  assert.equal(helper.current("map-presets"), null);
});

test("user presets can select fixed presets before their page adapter is loaded", () => {
  const env = createEnv();
  const helper = env.helper;
  helper.trackCurrentPayload("map-presets", {
    payload: { row: 2 },
    origin: { kind: "preset", id: "missing" },
  });

  const activated = helper.activateFixedPreset("map-presets", "default");

  assert.equal(activated.id, "default");
  assert.equal(helper.selectedPresetId("map-presets"), "");
  assert.equal(helper.selectedFixedId("map-presets"), "default");
  assert.equal(helper.current("map-presets"), null);
});

test("user presets keep current state when selecting a different source without applying it", () => {
  const env = createEnv();
  const helper = env.helper;
  let currentPayload = {
    row: 1,
  };
  helper.registerCollectionAdapter("calculator-layouts", {
    normalizePayload(payload) {
      return {
        row: Number.parseInt(payload?.row ?? 0, 10) || 0,
      };
    },
    capture() {
      return currentPayload;
    },
  });
  const alpha = helper.createPreset("calculator-layouts", {
    name: "Alpha",
    payload: {
      row: 1,
    },
  });
  const beta = helper.createPreset("calculator-layouts", {
    name: "Beta",
    payload: {
      row: 9,
    },
    select: false,
  });
  currentPayload = {
    row: 4,
  };
  helper.trackCurrentPayload("calculator-layouts");

  helper.setSelectedPresetId("calculator-layouts", beta.id);

  assert.equal(helper.selectedPresetId("calculator-layouts"), beta.id);
  assert.deepEqual(helper.current("calculator-layouts")?.origin, { kind: "preset", id: alpha.id });
  assert.deepEqual(helper.current("calculator-layouts")?.payload, { row: 4 });
});

test("user presets clear current state when copying the current payload into a new selected preset", () => {
  const env = createEnv();
  const helper = env.helper;
  let currentPayload = {
    row: 1,
  };
  helper.registerCollectionAdapter("calculator-layouts", {
    normalizePayload(payload) {
      return {
        row: Number.parseInt(payload?.row ?? 0, 10) || 0,
      };
    },
    capture() {
      return currentPayload;
    },
  });
  helper.createPreset("calculator-layouts", {
    name: "Alpha",
    payload: {
      row: 1,
    },
  });
  currentPayload = {
    row: 4,
  };
  helper.trackCurrentPayload("calculator-layouts");

  const copied = helper.createPreset("calculator-layouts", {
    name: "Copied current",
    payload: helper.current("calculator-layouts").payload,
  });

  assert.equal(helper.selectedPresetId("calculator-layouts"), copied.id);
  assert.equal(helper.current("calculator-layouts"), null);
});

test("user presets discard current state by applying the original source payload", () => {
  const env = createEnv();
  const helper = env.helper;
  let currentPayload = {
    row: 1,
  };
  helper.registerCollectionAdapter("calculator-layouts", {
    normalizePayload(payload) {
      return {
        row: Number.parseInt(payload?.row ?? 0, 10) || 0,
      };
    },
    capture() {
      return currentPayload;
    },
    apply(payload) {
      currentPayload = payload;
      return payload;
    },
  });
  const preset = helper.createPreset("calculator-layouts", {
    name: "Layout 1",
    payload: {
      row: 1,
    },
  });
  currentPayload = {
    row: 4,
  };
  helper.trackCurrentPayload("calculator-layouts");

  const discarded = helper.discardCurrent("calculator-layouts");

  assert.equal(discarded.action, "matched-preset");
  assert.deepEqual(currentPayload, { row: 1 });
  assert.equal(helper.current("calculator-layouts"), null);
  assert.equal(helper.selectedPresetId("calculator-layouts"), preset.id);
});

test("user presets can save a selected preset from an explicit save capture", () => {
  const env = createEnv();
  const helper = env.helper;
  let currentPayload = {
    row: 1,
  };
  helper.registerCollectionAdapter("map-presets", {
    normalizePayload(payload) {
      return {
        row: Number.parseInt(payload?.row ?? 0, 10) || 0,
        camera: payload?.camera && typeof payload.camera === "object" ? { ...payload.camera } : {},
      };
    },
    capture(options = {}) {
      return options.intent === "save"
        ? { row: currentPayload.row, camera: { zoom: 5 } }
        : currentPayload;
    },
  });
  const preset = helper.createPreset("map-presets", {
    name: "Camera",
    payload: {
      row: 1,
      camera: {},
    },
  });

  const saved = helper.saveCurrentToSelectedPreset("map-presets");

  assert.equal(saved.id, preset.id);
  assert.deepEqual(saved.payload, { row: 1, camera: { zoom: 5 } });
  assert.equal(helper.current("map-presets"), null);
  assert.equal(helper.selectedPresetId("map-presets"), preset.id);
});

test("user presets do not normalize a failed adapter capture into a default payload", () => {
  const env = createEnv();
  const helper = env.helper;
  helper.registerCollectionAdapter("map-presets", {
    normalizePayload(payload) {
      return {
        row: Number.parseInt(payload?.row ?? 0, 10) || 0,
      };
    },
    capture() {
      return null;
    },
  });

  assert.equal(helper.capturePayload("map-presets"), null);
  assert.equal(helper.ensurePersistedSelection("map-presets").action, "cleared");
});

test("user presets requiring live save capture fail instead of saving stale current payload", () => {
  const env = createEnv();
  const helper = env.helper;
  helper.registerCollectionAdapter("map-presets", {
    captureOnSave: true,
    normalizePayload(payload) {
      return {
        row: Number.parseInt(payload?.row ?? 0, 10) || 0,
      };
    },
    capture() {
      return null;
    },
  });
  const preset = helper.createPreset("map-presets", {
    name: "Needs camera",
    payload: { row: 1 },
  });
  helper.trackCurrentPayload("map-presets", {
    origin: { kind: "preset", id: preset.id },
    payload: { row: 9 },
  });

  assert.throws(
    () => helper.saveCurrentToSelectedPreset("map-presets"),
    /Preset save failed\./,
  );
  assert.deepEqual(helper.preset("map-presets", preset.id).payload, { row: 1 });
  assert.deepEqual(helper.current("map-presets").payload, { row: 9 });
});

test("user presets keep an action log for undo and redo of the current modified preset", () => {
  const env = createEnv();
  const helper = env.helper;
  let currentPayload = {
    row: 1,
  };
  const applied = [];
  helper.registerCollectionAdapter("calculator-layouts", {
    normalizePayload(payload) {
      return {
        row: Number.parseInt(payload?.row ?? 0, 10) || 0,
      };
    },
    capture() {
      return currentPayload;
    },
    apply(payload) {
      applied.push(payload);
      currentPayload = payload;
      return payload;
    },
  });
  const preset = helper.createPreset("calculator-layouts", {
    name: "Layout 1",
    payload: {
      row: 1,
    },
  });

  currentPayload = { row: 2 };
  helper.trackCurrentPayload("calculator-layouts");
  currentPayload = { row: 3 };
  helper.trackCurrentPayload("calculator-layouts");

  assert.equal(helper.currentHistoryState("calculator-layouts").canUndo, true);
  assert.equal(helper.current("calculator-layouts").events.length, 2);
  assert.deepEqual(helper.preset("calculator-layouts", preset.id).payload, { row: 1 });

  const undone = helper.undoCurrent("calculator-layouts");
  assert.deepEqual(undone.payload, { row: 2 });
  assert.deepEqual(applied.at(-1), { row: 2 });
  assert.equal(helper.currentHistoryState("calculator-layouts").canRedo, true);

  const redone = helper.redoCurrent("calculator-layouts");
  assert.deepEqual(redone.payload, { row: 3 });
  assert.deepEqual(applied.at(-1), { row: 3 });
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
                custom_layout: [],
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
              custom_layout: [[["overview"]]],
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

test("user presets patch bound Datastar signals on collection and adapter changes", () => {
  const env = createEnv();
  const signals = {};
  env.helper.bindDatastar(signals);

  assert.deepEqual(signals._user_presets, {
    version: 0,
    collections: {},
  });

  env.helper.registerCollectionAdapter("calculator-layouts", {
    fixedPresets() {
      return [{ id: "default", name: "Default", payload: { rows: [] } }];
    },
    normalizePayload(payload) {
      return { rows: Array.isArray(payload?.rows) ? payload.rows : [] };
    },
  });

  assert.equal(signals._user_presets.version, 1);
  assert.deepEqual(signals._user_presets.collections["calculator-layouts"], {
    selectedPresetId: "",
    selectedFixedId: "",
    hasCurrent: false,
    currentOrigin: { kind: "none", id: "" },
    presetCount: 0,
    fixedPresetCount: 1,
  });

  const preset = env.helper.createPreset("calculator-layouts", {
    name: "Compact",
    payload: { rows: [1] },
  });

  assert.equal(signals._user_presets.version, 2);
  assert.equal(signals._user_presets.collections["calculator-layouts"].selectedPresetId, preset.id);
  assert.equal(signals._user_presets.collections["calculator-layouts"].presetCount, 1);

  env.helper.unbindDatastar(signals);
  env.helper.renamePreset("calculator-layouts", preset.id, "Wide");
  assert.equal(signals._user_presets.version, 2);
});
