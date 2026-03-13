import test from "node:test";
import assert from "node:assert/strict";

import {
  FISHYMAP_EVENTS,
  FISHYMAP_STORAGE_KEYS,
  buildInitialRestorePatch,
  createFishyMapBridge,
  extractThemeSnapshot,
  mergeStatePatch,
  parseQueryState,
  resolveApiBaseUrl,
  resolveCdnBaseUrl,
  resolveMapRuntimeManifestUrl,
  rewriteApiUrl,
} from "./map-host.js";

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

class FakeElement extends EventTarget {
  constructor() {
    super();
    this.style = {};
    this.children = [];
    this.open = false;
  }

  appendChild(child) {
    this.children.push(child);
    return child;
  }

  querySelector() {
    return null;
  }

  setAttribute(name, value) {
    this[name] = value;
  }

  getAttribute(name) {
    return this[name] ?? null;
  }
}

class FakeCanvas extends FakeElement {
  constructor() {
    super();
    this.width = 0;
    this.height = 0;
    this.clientWidth = 640;
    this.clientHeight = 480;
    this.parentElement = new FakeElement();
    this.rectWidth = 640;
    this.rectHeight = 480;
  }

  getBoundingClientRect() {
    return {
      width: this.rectWidth,
      height: this.rectHeight,
    };
  }
}

class FakeContainer extends FakeElement {
  constructor(canvas) {
    super();
    this.canvas = canvas;
    this.clientWidth = 640;
    this.clientHeight = 480;
    this.rectWidth = 640;
    this.rectHeight = 480;
  }

  querySelector(selector) {
    if (selector === "canvas") {
      return this.canvas;
    }
    return null;
  }

  getBoundingClientRect() {
    return {
      width: this.rectWidth,
      height: this.rectHeight,
    };
  }
}

function installDomGlobals() {
  const original = {
    window: globalThis.window,
    document: globalThis.document,
    location: globalThis.location,
    localStorage: globalThis.localStorage,
    sessionStorage: globalThis.sessionStorage,
    MutationObserver: globalThis.MutationObserver,
    ResizeObserver: globalThis.ResizeObserver,
    fetch: globalThis.fetch,
    CustomEvent: globalThis.CustomEvent,
  };

  if (typeof globalThis.CustomEvent !== "function") {
    globalThis.CustomEvent = class CustomEvent extends Event {
      constructor(type, options = {}) {
        super(type);
        this.detail = options.detail;
      }
    };
  }

  const documentElement = new FakeElement();
  documentElement.getAttribute = (name) => (name === "data-theme" ? "fishy" : null);
  const body = new FakeElement();
  const document = {
    body,
    documentElement,
    visibilityState: "visible",
    getElementById() {
      return null;
    },
    createElement() {
      return new FakeElement();
    },
    addEventListener() {},
    removeEventListener() {},
  };
  const localStorage = new MemoryStorage();
  const sessionStorage = new MemoryStorage();
  const window = {
    location: {
      href: "https://fishystuff.fish/map/",
      hostname: "fishystuff.fish",
    },
    devicePixelRatio: 1,
    __fishystuffTheme: {
      resolvedTheme: "fishy",
      colors: {
        base100: "rgb(16 24 32 / 1)",
        primary: "rgb(240 120 60 / 1)",
        primaryContent: "rgb(255 255 255 / 1)",
      },
    },
    addEventListener() {},
    removeEventListener() {},
    getComputedStyle() {
      return {
        getPropertyValue() {
          return "";
        },
      };
    },
    fetch: async (input) => ({ ok: true, input }),
  };

  globalThis.window = window;
  globalThis.document = document;
  globalThis.location = window.location;
  globalThis.localStorage = localStorage;
  globalThis.sessionStorage = sessionStorage;
  globalThis.fetch = window.fetch;
  globalThis.MutationObserver = class {
    observe() {}
    disconnect() {}
  };
  globalThis.ResizeObserver = class {
    observe() {}
    disconnect() {}
  };

  return {
    document,
    window,
    localStorage,
    sessionStorage,
    restore() {
      globalThis.window = original.window;
      globalThis.document = original.document;
      globalThis.location = original.location;
      globalThis.localStorage = original.localStorage;
      globalThis.sessionStorage = original.sessionStorage;
      globalThis.MutationObserver = original.MutationObserver;
      globalThis.ResizeObserver = original.ResizeObserver;
      globalThis.fetch = original.fetch;
      globalThis.CustomEvent = original.CustomEvent;
    },
  };
}

function createFakeWasm(snapshotRef) {
  const calls = {
    applied: [],
    commands: [],
    sink: null,
    stateReads: 0,
  };
  return {
    calls,
    async default() {},
    fishymap_set_event_sink(callback) {
      calls.sink = callback;
    },
    fishymap_mount() {},
    fishymap_destroy() {},
    fishymap_apply_state_patch_json(json) {
      calls.applied.push(JSON.parse(json));
    },
    fishymap_send_command_json(json) {
      calls.commands.push(JSON.parse(json));
    },
    fishymap_get_current_state_json() {
      calls.stateReads += 1;
      return JSON.stringify(snapshotRef.current);
    },
  };
}

test("DOM state patches are forwarded to the wasm bridge", async () => {
  const env = installDomGlobals();
  let bridge;
  try {
    const canvas = new FakeCanvas();
    const container = new FakeContainer(canvas);
    const snapshotRef = {
      current: {
        version: 1,
        ready: false,
        filters: { fishIds: [], searchText: "", patchId: null, layerIdsVisible: [] },
        ui: { diagnosticsOpen: false, legendOpen: false, leftPanelOpen: true },
        view: { viewMode: "2d", camera: {} },
        selection: {},
        hover: {},
        catalog: { capabilities: [], layers: [], patches: [], fish: [] },
        statuses: {},
      },
    };
    const wasm = createFakeWasm(snapshotRef);
    bridge = createFishyMapBridge();
    await bridge.mount(container, {
      canvas,
      debounceMs: 0,
      wasmModule: wasm,
      locationHref: "https://fishystuff.fish/map/",
      localStorage: env.localStorage,
      sessionStorage: env.sessionStorage,
    });
    wasm.calls.applied.length = 0;

    container.dispatchEvent(
      new CustomEvent(FISHYMAP_EVENTS.setState, {
        detail: {
          version: 1,
          filters: {
            fromPatchId: "2026-02-26",
            toPatchId: "2026-03-12",
            layerIdsVisible: ["zones", "terrain"],
            layerIdsOrdered: ["zones", "terrain", "minimap"],
            layerOpacities: {
              zones: 0.7,
              terrain: 0.35,
            },
            layerClipMasks: {
              terrain: "zones",
            },
          },
          ui: {
            showPoints: false,
            showPointIcons: false,
            pointIconScale: 2.5,
          },
        },
      }),
    );

    await new Promise((resolve) => setTimeout(resolve, 0));

    assert.equal(wasm.calls.applied.length, 1);
    assert.deepEqual(wasm.calls.applied[0].filters, {
      patchId: null,
      fromPatchId: "2026-02-26",
      toPatchId: "2026-03-12",
      layerIdsVisible: ["zones", "terrain"],
      layerIdsOrdered: ["zones", "terrain", "minimap"],
      layerOpacities: {
        zones: 0.7,
        terrain: 0.35,
      },
      layerClipMasks: {
        terrain: "zones",
      },
    });
    assert.equal(wasm.calls.applied[0].ui.showPoints, false);
    assert.equal(wasm.calls.applied[0].ui.showPointIcons, false);
    assert.equal(wasm.calls.applied[0].ui.pointIconScale, 2.5);
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("wasm output events are redispatched as DOM CustomEvents", async () => {
  const env = installDomGlobals();
  let bridge;
  try {
    const canvas = new FakeCanvas();
    const container = new FakeContainer(canvas);
    const snapshotRef = {
      current: {
        version: 1,
        ready: true,
        filters: { fishIds: [], searchText: "", patchId: null, layerIdsVisible: [] },
        ui: { diagnosticsOpen: false, legendOpen: false, leftPanelOpen: true },
        view: {
          viewMode: "3d",
          camera: {
            pivotWorldX: 10,
            pivotWorldZ: 20,
            distance: 5000,
          },
        },
        selection: {},
        hover: {},
        catalog: { capabilities: ["restore"], layers: [], patches: [], fish: [] },
        statuses: {},
      },
    };
    const wasm = createFakeWasm(snapshotRef);
    bridge = createFishyMapBridge();
    await bridge.mount(container, {
      canvas,
      wasmModule: wasm,
      locationHref: "https://fishystuff.fish/map/",
      localStorage: env.localStorage,
      sessionStorage: env.sessionStorage,
    });

    const received = await new Promise((resolve) => {
      container.addEventListener(
        FISHYMAP_EVENTS.viewChanged,
        (event) => resolve(event.detail),
        { once: true },
      );
      wasm.calls.sink(
        JSON.stringify({
          type: "view-changed",
          version: 1,
          viewMode: "3d",
          camera: {
            pivotWorldX: 10,
            pivotWorldZ: 20,
            distance: 5000,
          },
        }),
      );
    });

    assert.equal(received.type, "view-changed");
    assert.equal(received.state.view.viewMode, "3d");
    assert.equal(received.state.view.camera.distance, 5000);
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("hover output events are redispatched without cloning the full map state", async () => {
  const env = installDomGlobals();
  let bridge;
  try {
    const canvas = new FakeCanvas();
    const container = new FakeContainer(canvas);
    const snapshotRef = {
      current: {
        version: 1,
        ready: true,
        filters: { fishIds: [], searchText: "", patchId: null, layerIdsVisible: [] },
        ui: { diagnosticsOpen: false, legendOpen: false, leftPanelOpen: true },
        view: { viewMode: "2d", camera: {} },
        selection: {},
        hover: {},
        catalog: { capabilities: [], layers: [], patches: [], fish: [] },
        statuses: {},
      },
    };
    const wasm = createFakeWasm(snapshotRef);
    bridge = createFishyMapBridge();
    await bridge.mount(container, {
      canvas,
      wasmModule: wasm,
      locationHref: "https://fishystuff.fish/map/",
      localStorage: env.localStorage,
      sessionStorage: env.sessionStorage,
    });
    wasm.calls.stateReads = 0;

    const received = await new Promise((resolve) => {
      container.addEventListener(
        FISHYMAP_EVENTS.hoverChanged,
        (event) => resolve(event.detail),
        { once: true },
      );
      wasm.calls.sink(
        JSON.stringify({
          type: "hover-changed",
          version: 1,
          worldX: 11,
          worldZ: 22,
          zoneRgb: 1193046,
          zoneName: "Coastal Shelf",
          layerSamples: [
            {
              layerId: "zones",
              layerName: "Zones",
              kind: "tiled-raster",
              rgb: [18, 52, 86],
              rgbU32: 1193046,
            },
          ],
        }),
      );
    });

    assert.equal(wasm.calls.stateReads, 0);
    assert.equal(received.state, undefined);
    assert.deepEqual(received.hover, {
      worldX: 11,
      worldZ: 22,
      zoneRgb: 1193046,
      zoneName: "Coastal Shelf",
      layerSamples: [
        {
          layerId: "zones",
          layerName: "Zones",
          kind: "tiled-raster",
          rgb: [18, 52, 86],
          rgbU32: 1193046,
        },
      ],
    });
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("bootstrap sync replays state changes that happen after mount without wasm push events", async () => {
  const env = installDomGlobals();
  let bridge;
  try {
    const canvas = new FakeCanvas();
    const container = new FakeContainer(canvas);
    const snapshotRef = {
      current: {
        version: 1,
        ready: false,
        filters: { fishIds: [], searchText: "", patchId: null, layerIdsVisible: [] },
        ui: { diagnosticsOpen: false, legendOpen: false, leftPanelOpen: true },
        view: { viewMode: "2d", camera: {} },
        selection: {},
        hover: {},
        catalog: { capabilities: [], layers: [], patches: [], fish: [] },
        statuses: {},
      },
    };
    const wasm = createFakeWasm(snapshotRef);
    bridge = createFishyMapBridge();
    await bridge.mount(container, {
      canvas,
      wasmModule: wasm,
      locationHref: "https://fishystuff.fish/map/",
      localStorage: env.localStorage,
      sessionStorage: env.sessionStorage,
    });

    const readyEvent = new Promise((resolve) => {
      container.addEventListener(
        FISHYMAP_EVENTS.ready,
        (event) => resolve(event.detail),
        { once: true },
      );
    });

    snapshotRef.current = {
      version: 1,
      ready: true,
      filters: { fishIds: [], searchText: "", patchId: null, layerIdsVisible: ["zones"] },
      ui: { diagnosticsOpen: false, legendOpen: false, leftPanelOpen: true },
      view: { viewMode: "2d", camera: { centerWorldX: 12, centerWorldZ: 34, zoom: 2 } },
      selection: {},
      hover: {},
      catalog: { capabilities: ["restore"], layers: [], patches: [], fish: [] },
      statuses: { metaStatus: "meta: loaded" },
    };

    const detail = await readyEvent;
    assert.equal(detail.type, "ready");
    assert.equal(detail.state.ready, true);
    assert.equal(detail.state.statuses.metaStatus, "meta: loaded");
    assert.deepEqual(detail.state.filters.layerIdsVisible, ["zones"]);
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("mount does not persist an implicit hidden-all layer override before the map is ready", async () => {
  const env = installDomGlobals();
  let bridge;
  try {
    const canvas = new FakeCanvas();
    const container = new FakeContainer(canvas);
    const snapshotRef = {
      current: {
        version: 1,
        ready: false,
        filters: { fishIds: [], searchText: "", patchId: null, layerIdsVisible: [] },
        ui: { diagnosticsOpen: false, legendOpen: false, leftPanelOpen: true },
        view: { viewMode: "2d", camera: {} },
        selection: {},
        hover: {},
        catalog: { capabilities: [], layers: [], patches: [], fish: [] },
        statuses: {},
      },
    };
    const wasm = createFakeWasm(snapshotRef);
    bridge = createFishyMapBridge();
    await bridge.mount(container, {
      canvas,
      debounceMs: 0,
      wasmModule: wasm,
      locationHref: "https://fishystuff.fish/map/",
      localStorage: env.localStorage,
      sessionStorage: env.sessionStorage,
    });

    const savedPrefs = JSON.parse(env.localStorage.getItem(FISHYMAP_STORAGE_KEYS.prefs));
    assert.equal("layerIdsVisible" in (savedPrefs.filters || {}), false);
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("canvas sizing falls back to the map container when the canvas rect is not ready yet", async () => {
  const env = installDomGlobals();
  let bridge;
  try {
    const canvas = new FakeCanvas();
    canvas.rectWidth = 0;
    canvas.rectHeight = 0;
    canvas.clientWidth = 0;
    canvas.clientHeight = 0;
    const container = new FakeContainer(canvas);
    container.rectWidth = 780;
    container.rectHeight = 459;
    container.clientWidth = 780;
    container.clientHeight = 459;
    const snapshotRef = {
      current: {
        version: 1,
        ready: false,
        filters: { fishIds: [], searchText: "", patchId: null, layerIdsVisible: [] },
        ui: { diagnosticsOpen: false, legendOpen: false, leftPanelOpen: true },
        view: { viewMode: "2d", camera: {} },
        selection: {},
        hover: {},
        catalog: { capabilities: [], layers: [], patches: [], fish: [] },
        statuses: {},
      },
    };
    const wasm = createFakeWasm(snapshotRef);
    bridge = createFishyMapBridge();
    await bridge.mount(container, {
      canvas,
      wasmModule: wasm,
      locationHref: "https://fishystuff.fish/map/",
      localStorage: env.localStorage,
      sessionStorage: env.sessionStorage,
    });

    assert.equal(canvas.width, 780);
    assert.equal(canvas.height, 459);
    assert.equal(canvas.style.width, "780px");
    assert.equal(canvas.style.height, "459px");
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("restore priority is URL over session over local preferences", () => {
  const localStorage = new MemoryStorage({
    "fishystuff.map.prefs.v1": JSON.stringify({
      version: 1,
      filters: {
        fromPatchId: "local-from",
        toPatchId: "local-to",
        layerIdsVisible: ["terrain"],
        layerIdsOrdered: ["terrain", "minimap"],
        layerOpacities: {
          terrain: 0.25,
        },
        layerClipMasks: {
          terrain: "zones",
        },
      },
      ui: {
        legendOpen: true,
        showPointIcons: false,
        pointIconScale: 2.4,
      },
    }),
  });
  const sessionStorage = new MemoryStorage({
    "fishystuff.map.session.v1": JSON.stringify({
      version: 1,
      view: {
        viewMode: "2d",
        camera: {
          centerWorldX: 100,
          centerWorldZ: 200,
          zoom: 2,
        },
      },
      selection: {
        fishId: 33,
        zoneRgb: 16711935,
      },
      filters: {
        fromPatchId: "session-from",
        toPatchId: "session-to",
        layerIdsOrdered: ["zones", "terrain", "minimap"],
        layerOpacities: {
          zones: 0.6,
        },
        layerClipMasks: {
          terrain: "zones",
        },
      },
      ui: {
        legendOpen: false,
        showPoints: false,
        pointIconScale: 1.8,
      },
    }),
  });

  const patch = buildInitialRestorePatch({
    locationHref:
      "https://fishystuff.fish/map/?fromPatch=url-from&toPatch=url-to&fish=77&view=3d&layers=zones,terrain",
    localStorage,
    sessionStorage,
  });

  assert.equal(patch.filters.patchId, null);
  assert.equal(patch.filters.fromPatchId, "url-from");
  assert.equal(patch.filters.toPatchId, "url-to");
  assert.deepEqual(patch.filters.layerIdsVisible, ["zones", "terrain"]);
  assert.deepEqual(patch.filters.layerIdsOrdered, ["zones", "terrain", "minimap"]);
  assert.deepEqual(patch.filters.layerOpacities, { zones: 0.6 });
  assert.deepEqual(patch.filters.layerClipMasks, { terrain: "zones" });
  assert.equal(patch.ui.legendOpen, false);
  assert.equal(patch.ui.showPoints, false);
  assert.equal(patch.ui.showPointIcons, false);
  assert.equal(patch.ui.pointIconScale, 1.8);
  assert.equal(patch.commands.focusFishId, 77);
  assert.equal(patch.commands.selectZoneRgb, 16711935);
  assert.equal(patch.commands.setViewMode, "3d");
  assert.equal(patch.commands.restoreView.viewMode, "3d");
});

test("layer opacity overrides replace the previous map instead of merging stale entries", () => {
  const patch = mergeStatePatch(
    {
      filters: {
        layerOpacities: {
          zones: 0.7,
          terrain: 0.35,
        },
      },
    },
    {
      filters: {
        layerOpacities: {
          terrain: 0.2,
        },
      },
    },
  );

  assert.deepEqual(patch.filters.layerOpacities, { terrain: 0.2 });
});

test("layer clip mask overrides replace the previous map instead of merging stale entries", () => {
  const patch = mergeStatePatch(
    {
      filters: {
        layerClipMasks: {
          terrain: "zones",
          fish_density: "terrain",
        },
      },
    },
    {
      filters: {
        layerClipMasks: {
          terrain: "region_groups",
        },
      },
    },
  );

  assert.deepEqual(patch.filters.layerClipMasks, { terrain: "region_groups" });
});

test("layer clip mask normalization flattens nested attachments to a single root", () => {
  const patch = mergeStatePatch(
    {},
    {
      filters: {
        layerClipMasks: {
          terrain: "zones",
          fish_density: "terrain",
          region_groups: "fish_density",
        },
      },
    },
  );

  assert.deepEqual(patch.filters.layerClipMasks, {
    terrain: "zones",
    fish_density: "zones",
    region_groups: "zones",
  });
});

test("session restore preserves multiple selected fish terms", () => {
  const sessionStorage = new MemoryStorage({
    [FISHYMAP_STORAGE_KEYS.session]: JSON.stringify({
      version: 1,
      selection: {
        fishId: 33,
      },
      filters: {
        fishIds: [11, 22, 33],
      },
    }),
  });

  const patch = buildInitialRestorePatch({
    locationHref: "https://fishystuff.fish/map/",
    localStorage: new MemoryStorage(),
    sessionStorage,
  });

  assert.deepEqual(patch.filters.fishIds, [11, 22, 33]);
  assert.equal(patch.commands.focusFishId, 33);
});

test("legacy empty layer visibility snapshots do not hide every layer on restore", () => {
  const localStorage = new MemoryStorage({
    [FISHYMAP_STORAGE_KEYS.prefs]: JSON.stringify({
      version: 1,
      filters: {
        layerIdsVisible: [],
      },
    }),
  });

  const patch = buildInitialRestorePatch({
    locationHref: "https://fishystuff.fish/map/",
    localStorage,
    sessionStorage: new MemoryStorage(),
  });

  assert.equal("layerIdsVisible" in (patch.filters || {}), false);
});

test("explicit empty layer visibility snapshots still restore intentionally hidden layers", () => {
  const localStorage = new MemoryStorage({
    [FISHYMAP_STORAGE_KEYS.prefs]: JSON.stringify({
      version: 1,
      filters: {
        layerIdsVisible: [],
        layerVisibilityExplicit: true,
      },
    }),
  });

  const patch = buildInitialRestorePatch({
    locationHref: "https://fishystuff.fish/map/",
    localStorage,
    sessionStorage: new MemoryStorage(),
  });

  assert.deepEqual(patch.filters.layerIdsVisible, []);
});

test("legacy patch query alias expands to an exact range", () => {
  const patch = parseQueryState("https://fishystuff.fish/map/?patch=2026-02-26");

  assert.equal(patch.filters.patchId, "2026-02-26");
  assert.equal(patch.filters.fromPatchId, "2026-02-26");
  assert.equal(patch.filters.toPatchId, "2026-02-26");
});

test("explicit query range keeps the canonical patch id empty", () => {
  const patch = parseQueryState(
    "https://fishystuff.fish/map/?fromPatch=2026-02-26&toPatch=2026-03-12",
  );

  assert.equal(patch.filters.patchId, null);
  assert.equal(patch.filters.fromPatchId, "2026-02-26");
  assert.equal(patch.filters.toPatchId, "2026-03-12");
});

test("theme extraction returns resolved theme tokens", () => {
  const snapshot = extractThemeSnapshot({
    doc: {
      documentElement: {
        getAttribute(name) {
          return name === "data-theme" ? "retro-fishy" : null;
        },
      },
    },
    win: {
      __fishystuffTheme: {
        resolvedTheme: "retro-fishy",
        colors: {
          base100: "rgb(12 34 56 / 1)",
          primary: "rgb(200 150 90 / 1)",
          primaryContent: "rgb(255 255 255 / 1)",
        },
      },
    },
  });

  assert.equal(snapshot.name, "retro-fishy");
  assert.equal(snapshot.colors.base100, "rgb(12 34 56 / 1)");
  assert.equal(snapshot.colors.primary, "rgb(200 150 90 / 1)");
});

test("theme extraction preserves oklch colors when the browser keeps them", () => {
  const snapshot = extractThemeSnapshot({
    doc: {
      documentElement: {
        getAttribute(name) {
          return name === "data-theme" ? "retro-fishy" : null;
        },
      },
      createElement(tag) {
        return {
          style: {},
        };
      },
    },
    win: {
      __fishystuffTheme: {
        resolvedTheme: "retro-fishy",
        colors: {
          base100: "oklch(12% 0.03 250)",
        },
      },
    },
  });

  assert.equal(snapshot.name, "retro-fishy");
  assert.equal(snapshot.colors.base100, "oklch(12% 0.03 250)");
});

test("theme extraction reads distinct base200 and base300 tokens from the probe", () => {
  const base = {
    styles: {
      "background-color": "rgb(10 11 12 / 1)",
      color: "rgb(240 241 242 / 1)",
    },
  };
  const base200 = {
    styles: {
      "background-color": "rgb(20 21 22 / 1)",
    },
  };
  const base300 = {
    styles: {
      "background-color": "rgb(30 31 32 / 1)",
    },
  };
  const probe = {
    querySelector(selector) {
      if (selector === '[data-role="base"]') {
        return base;
      }
      if (selector === '[data-role="base-200"]') {
        return base200;
      }
      if (selector === '[data-role="base-300"]') {
        return base300;
      }
      return null;
    },
  };

  const snapshot = extractThemeSnapshot({
    doc: {
      documentElement: {
        getAttribute(name) {
          return name === "data-theme" ? "retro-fishy" : null;
        },
      },
      body: {},
      getElementById(id) {
        return id === "fishystuff-theme-probe" ? probe : null;
      },
    },
    win: {
      getComputedStyle(element) {
        return {
          getPropertyValue(name) {
            return element?.styles?.[name] || "";
          },
        };
      },
    },
  });

  assert.equal(snapshot.colors.base100, "rgb(10 11 12 / 1)");
  assert.equal(snapshot.colors.base200, "rgb(20 21 22 / 1)");
  assert.equal(snapshot.colors.base300, "rgb(30 31 32 / 1)");
});

test("local dev API base resolves to loopback", () => {
  assert.equal(resolveApiBaseUrl({ hostname: "localhost" }), "http://127.0.0.1:8080");
  assert.equal(resolveApiBaseUrl({ hostname: "127.0.0.1" }), "http://127.0.0.1:8080");
  assert.equal(
    resolveApiBaseUrl({ hostname: "map.localhost" }),
    "http://127.0.0.1:8080",
  );
  assert.equal(
    resolveApiBaseUrl({ hostname: "fishystuff.fish" }),
    "https://api.fishystuff.fish",
  );
});

test("CDN base resolves to local dev or production host", () => {
  assert.equal(resolveCdnBaseUrl({ hostname: "localhost" }), "http://127.0.0.1:4040");
  assert.equal(resolveCdnBaseUrl({ hostname: "127.0.0.1" }), "http://127.0.0.1:4040");
  assert.equal(
    resolveCdnBaseUrl({ hostname: "map.localhost" }),
    "http://127.0.0.1:4040",
  );
  assert.equal(
    resolveCdnBaseUrl({ hostname: "fishystuff.fish" }),
    "https://cdn.fishystuff.fish",
  );
  assert.equal(
    resolveCdnBaseUrl({ hostname: "fishystuff.fish" }, "https://override.example.com/"),
    "https://override.example.com",
  );
});

test("runtime manifest URL is cache-busted against the CDN base", () => {
  assert.equal(
    resolveMapRuntimeManifestUrl({ hostname: "localhost" }, 123),
    "http://127.0.0.1:4040/map/runtime-manifest.json?v=123",
  );
  assert.equal(
    resolveMapRuntimeManifestUrl({ hostname: "fishystuff.fish" }, "deploy-456"),
    "https://cdn.fishystuff.fish/map/runtime-manifest.json?v=deploy-456",
  );
});

test("only API requests are rewritten to the API origin", () => {
  const locationHref = "http://127.0.0.1:1990/map/";
  const apiBaseUrl = "http://127.0.0.1:8080";

  assert.equal(
    rewriteApiUrl("/api/v1/meta", apiBaseUrl, locationHref),
    "http://127.0.0.1:8080/api/v1/meta",
  );
  assert.equal(
    rewriteApiUrl("/images/tiles/minimap/v1/tileset.json", apiBaseUrl, locationHref),
    "/images/tiles/minimap/v1/tileset.json",
  );
  assert.equal(
    rewriteApiUrl("/images/tiles/mask/v1/tileset.json", apiBaseUrl, locationHref),
    "/images/tiles/mask/v1/tileset.json",
  );
  assert.equal(
    rewriteApiUrl("/images/terrain/v1/manifest.json", apiBaseUrl, locationHref),
    "/images/terrain/v1/manifest.json",
  );
});
