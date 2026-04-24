import { test } from "bun:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

import { applyMapPageSignalsPatch } from "./map-page-signals.js";
import {
  applyStoredMapPresetState,
  MAP_PRESET_COLLECTION_KEY,
  patchTouchesMapPreset,
  registerMapPresetAdapter,
} from "./map-presets.js";

const USER_PRESETS_SOURCE = fs.readFileSync(
  new URL("../js/user-presets.js", import.meta.url),
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

function createWindowStub() {
  const listeners = new Map();
  return {
    addEventListener(type, listener) {
      if (!listeners.has(type)) {
        listeners.set(type, []);
      }
      listeners.get(type).push(listener);
    },
    removeEventListener(type, listener) {
      listeners.set(
        type,
        (listeners.get(type) || []).filter((candidate) => candidate !== listener),
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

function installUserPresetsGlobal() {
  const previous = {
    window: globalThis.window,
    localStorage: globalThis.localStorage,
    crypto: globalThis.crypto,
    CustomEvent: globalThis.CustomEvent,
    hadWindow: Object.prototype.hasOwnProperty.call(globalThis, "window"),
    hadLocalStorage: Object.prototype.hasOwnProperty.call(globalThis, "localStorage"),
    hadCrypto: Object.prototype.hasOwnProperty.call(globalThis, "crypto"),
    hadCustomEvent: Object.prototype.hasOwnProperty.call(globalThis, "CustomEvent"),
  };
  const window = createWindowStub();
  const localStorage = new MemoryStorage();
  let uuidCounter = 0;
  globalThis.window = window;
  globalThis.localStorage = localStorage;
  Object.defineProperty(globalThis, "crypto", {
    configurable: true,
    value: {
      randomUUID() {
        uuidCounter += 1;
        return `00000000-0000-4000-8000-${String(uuidCounter).padStart(12, "0")}`;
      },
    },
  });
  globalThis.CustomEvent = class CustomEvent {
    constructor(type, options = {}) {
      this.type = type;
      this.detail = options.detail;
    }
  };
  vm.runInThisContext(USER_PRESETS_SOURCE, { filename: "user-presets.js" });
  return {
    helper: window.__fishystuffUserPresets,
    restore() {
      if (previous.hadWindow) {
        globalThis.window = previous.window;
      } else {
        delete globalThis.window;
      }
      if (previous.hadLocalStorage) {
        globalThis.localStorage = previous.localStorage;
      } else {
        delete globalThis.localStorage;
      }
      if (previous.hadCrypto) {
        Object.defineProperty(globalThis, "crypto", {
          configurable: true,
          value: previous.crypto,
        });
      } else {
        delete globalThis.crypto;
      }
      if (previous.hadCustomEvent) {
        globalThis.CustomEvent = previous.CustomEvent;
      } else {
        delete globalThis.CustomEvent;
      }
    },
  };
}

function defaultSignals() {
  return {
    _map_ui: {
      windowUi: {
        search: { open: true, collapsed: false, x: null, y: null },
        settings: { open: false, collapsed: false, x: null, y: null, autoAdjustView: true },
        zoneInfo: { open: true, collapsed: false, x: null, y: null, tab: "" },
        layers: { open: true, collapsed: false, x: null, y: null },
        bookmarks: { open: false, collapsed: false, x: null, y: null },
      },
      search: { query: "", selectedTerms: [] },
      layers: { expandedLayerIds: [], hoverFactsVisibleByLayer: {} },
    },
    _map_bridged: {
      ui: {
        diagnosticsOpen: false,
        showPoints: true,
        showPointIcons: true,
        viewMode: "2d",
        pointIconScale: 2,
      },
      filters: {
        layerIdsVisible: ["bookmarks", "fish_evidence", "zone_mask", "minimap"],
        layerIdsOrdered: [],
        layerClipMasks: { fish_evidence: "zone_mask" },
      },
    },
    _map_bookmarks: {
      entries: [{ id: "bookmark:1", label: "Keep me", worldX: 1, worldZ: 2 }],
    },
    _map_session: {
      view: { viewMode: "2d", camera: {} },
      selection: { zoneRgb: "1,2,3" },
    },
    _map_runtime: {
      catalog: { layers: [] },
    },
  };
}

test("map preset adapter applies durable map state without replacing bookmarks", () => {
  const env = installUserPresetsGlobal();
  try {
    const signals = defaultSignals();
    registerMapPresetAdapter({
      readSignals: () => signals,
      applyPatch: (patch) => applyMapPageSignalsPatch(signals, patch),
    });
    const preset = env.helper.createPreset(MAP_PRESET_COLLECTION_KEY, {
      name: "Zone mask view",
      payload: {
        search: { query: "eel", selectedTerms: [] },
        bridgedUi: {
          showPoints: false,
          showPointIcons: true,
          viewMode: "3d",
          pointIconScale: 1.5,
        },
        bridgedFilters: {
          layerIdsVisible: ["zone_mask"],
          layerIdsOrdered: ["zone_mask", "bookmarks"],
          layerClipMasks: { fish_evidence: "zone_mask" },
        },
        view: { viewMode: "3d", camera: { zoom: 3 } },
      },
      select: false,
    });

    env.helper.activatePreset(MAP_PRESET_COLLECTION_KEY, preset.id);

    assert.equal(signals._map_ui.search.query, "eel");
    assert.equal(signals._map_bridged.ui.showPoints, false);
    assert.deepEqual(signals._map_bridged.filters.layerIdsVisible, ["zone_mask"]);
    assert.deepEqual(signals._map_session.view, { viewMode: "3d", camera: { zoom: 3 } });
    assert.deepEqual(signals._map_bookmarks.entries, [
      { id: "bookmark:1", label: "Keep me", worldX: 1, worldZ: 2 },
    ]);
    assert.equal(env.helper.selectedPresetId(MAP_PRESET_COLLECTION_KEY), preset.id);
    assert.equal(env.helper.current(MAP_PRESET_COLLECTION_KEY), null);
  } finally {
    env.restore();
  }
});

test("map preset adapter treats the default preset camera as a runtime baseline", () => {
  const env = installUserPresetsGlobal();
  try {
    const signals = defaultSignals();
    signals._map_session.view = {
      viewMode: "2d",
      camera: { centerWorldX: -307200, centerWorldZ: 460800, zoom: 3678.33 },
    };
    registerMapPresetAdapter({
      readSignals: () => signals,
      applyPatch: (patch) => applyMapPageSignalsPatch(signals, patch),
    });

    const tracked = env.helper.ensurePersistedSelection(MAP_PRESET_COLLECTION_KEY);

    assert.equal(tracked.action, "matched-fixed");
    assert.equal(env.helper.selectedFixedId(MAP_PRESET_COLLECTION_KEY), "default");
    assert.equal(env.helper.current(MAP_PRESET_COLLECTION_KEY), null);
  } finally {
    env.restore();
  }
});

test("map preset adapter applying default ignores stale session-only camera state", () => {
  const env = installUserPresetsGlobal();
  try {
    const signals = defaultSignals();
    signals._map_bridged.ui.viewMode = "3d";
    signals._map_bridged.ui.showPoints = false;
    signals._map_ui.search.query = "eel";
    signals._map_session.view = {
      viewMode: "3d",
      camera: { centerWorldX: 10, centerWorldZ: 20, zoom: 3 },
    };
    registerMapPresetAdapter({
      readSignals: () => signals,
      applyPatch: (patch) => applyMapPageSignalsPatch(signals, patch),
    });

    env.helper.activateFixedPreset(MAP_PRESET_COLLECTION_KEY, "default");

    assert.equal(signals._map_ui.search.query, "");
    assert.equal(signals._map_bridged.ui.showPoints, true);
    assert.equal(signals._map_bridged.ui.viewMode, "2d");
    assert.deepEqual(signals._map_session.view, {
      viewMode: "3d",
      camera: { centerWorldX: 10, centerWorldZ: 20, zoom: 3 },
    });
    assert.equal(env.helper.selectedFixedId(MAP_PRESET_COLLECTION_KEY), "default");
    assert.equal(env.helper.current(MAP_PRESET_COLLECTION_KEY), null);
  } finally {
    env.restore();
  }
});

test("map preset restore applies a fixed preset selected before the adapter loaded", () => {
  const env = installUserPresetsGlobal();
  try {
    env.helper.activateFixedPreset(MAP_PRESET_COLLECTION_KEY, "default");
    const signals = defaultSignals();
    signals._map_bridged.ui.viewMode = "3d";
    signals._map_bridged.ui.showPoints = false;
    signals._map_ui.search.query = "eel";
    registerMapPresetAdapter({
      readSignals: () => signals,
      applyPatch: (patch) => applyMapPageSignalsPatch(signals, patch),
    });

    const applied = applyStoredMapPresetState({
      readSignals: () => signals,
      applyPatch: (patch) => applyMapPageSignalsPatch(signals, patch),
    });

    assert.equal(applied.id, "default");
    assert.equal(signals._map_ui.search.query, "");
    assert.equal(signals._map_bridged.ui.showPoints, true);
    assert.equal(signals._map_bridged.ui.viewMode, "2d");
  } finally {
    env.restore();
  }
});

test("map preset adapter ignores camera for automatic tracking but captures it for save and clone", () => {
  const env = installUserPresetsGlobal();
  try {
    const signals = defaultSignals();
    signals._map_session.view = {
      viewMode: "2d",
      camera: { centerWorldX: -1, centerWorldZ: 2, zoom: 3 },
    };
    registerMapPresetAdapter({
      readSignals: () => signals,
      readBridgeState: () => ({
        view: {
          viewMode: "2d",
          camera: { centerWorldX: 10, centerWorldZ: 20, zoom: 4 },
        },
      }),
      applyPatch: (patch) => applyMapPageSignalsPatch(signals, patch),
    });

    assert.equal(patchTouchesMapPreset({ _map_session: { view: { camera: { zoom: 9 } } } }), false);
    assert.deepEqual(
      env.helper.capturePayload(MAP_PRESET_COLLECTION_KEY).view,
      { viewMode: "2d", camera: {} },
    );
    assert.deepEqual(
      env.helper.capturePayload(MAP_PRESET_COLLECTION_KEY, { intent: "clone" }).view,
      { viewMode: "2d", camera: { centerWorldX: 10, centerWorldZ: 20, zoom: 4 } },
    );
    assert.deepEqual(
      env.helper.capturePayload(MAP_PRESET_COLLECTION_KEY, { intent: "save" }).view,
      { viewMode: "2d", camera: { centerWorldX: 10, centerWorldZ: 20, zoom: 4 } },
    );
  } finally {
    env.restore();
  }
});
