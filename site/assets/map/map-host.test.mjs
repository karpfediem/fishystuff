import test from "node:test";
import assert from "node:assert/strict";

import {
  FISHYMAP_EVENTS,
  applyStatePatch,
  buildInitialRestorePatch,
  createFishyMapBridge,
  extractThemeSnapshot,
  mergeStatePatch,
  normalizeStatePatch,
  parseQueryState,
  resolveApiBaseUrl,
  resolveCdnBaseUrl,
  resolveMapRuntimeManifestUrl,
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
    bootstrapReads: 0,
    profilingResets: [],
  };
  return {
    calls,
    profilingSummary: {
      scenario: "browser",
      bevy_version: "0.18.0",
      git_revision: null,
      build_profile: "dev",
      frames: 0,
      warmup_frames: 0,
      wall_clock_ms: 0,
      frame_time_ms: { avg: 0, p50: 0, p95: 0, p99: 0, max: 0 },
      named_spans: {},
      counters: {},
    },
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
    fishymap_get_bootstrap_state_json() {
      calls.bootstrapReads += 1;
      return JSON.stringify({
        version: snapshotRef.current?.version ?? 1,
        ready: snapshotRef.current?.ready === true,
        statuses: snapshotRef.current?.statuses || {},
      });
    },
    fishymap_reset_profiling_json(json) {
      calls.profilingResets.push(JSON.parse(json));
    },
    fishymap_get_profiling_summary_json() {
      return JSON.stringify(this.profilingSummary);
    },
    fishymap_get_profiling_trace_json() {
      return JSON.stringify({ traceEvents: [] });
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
            zoneRgbs: [0xc17f7f, 0x3c963c, 0xc17f7f],
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
            bookmarks: [
              {
                id: "bookmark-a",
                label: "Velia",
                worldX: 123.5,
                worldZ: -45.25,
              },
              {
                id: "bookmark-a",
                worldX: 999,
                worldZ: 999,
              },
            ],
          },
        },
      }),
    );

    await new Promise((resolve) => setTimeout(resolve, 0));

    assert.equal(wasm.calls.applied.length, 1);
    assert.deepEqual(wasm.calls.applied[0].filters, {
      patchId: null,
      zoneRgbs: [0xc17f7f, 0x3c963c],
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
    assert.deepEqual(wasm.calls.applied[0].ui.bookmarks, [
      {
        id: "bookmark-a",
        label: "Velia",
        worldX: 123.5,
        worldZ: -45.25,
      },
    ]);
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("search text patches are ignored by the bridge input state", () => {
  const next = applyStatePatch(undefined, {
    version: 1,
    filters: {
      searchText: "Zenato Sea ",
    },
  });

  assert.equal("searchText" in next.filters, false);
});

test("view mode patches are preserved in host input state", () => {
  const next = applyStatePatch(undefined, {
    version: 1,
    ui: {
      viewMode: "3d",
    },
  });

  assert.equal(next.ui.viewMode, "3d");
});

test("view mode input patches send setViewMode when runtime differs", async () => {
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
      debounceMs: 0,
      wasmModule: wasm,
      locationHref: "https://fishystuff.fish/map/",
      localStorage: env.localStorage,
      sessionStorage: env.sessionStorage,
    });
    wasm.calls.commands.length = 0;

    bridge.setState({
      version: 1,
      ui: {
        viewMode: "3d",
      },
    });

    assert.equal(bridge.getCurrentInputState().ui.viewMode, "3d");
    assert.deepEqual(wasm.calls.commands.at(-1), {
      setViewMode: "3d",
    });
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("fish filter term patches normalize aliases for favorites and grades", () => {
  const next = applyStatePatch(undefined, {
    version: 1,
    filters: {
      fishFilterTerms: ["favorite", "rare", "trash", "favourites", ""],
    },
  });

  assert.deepEqual(next.filters.fishFilterTerms, ["favourite", "yellow", "white"]);
});

test("shared fish state input patches are normalized in host input state", () => {
  const next = applyStatePatch(undefined, {
    version: 1,
    ui: {
      sharedFishState: {
        caughtIds: ["912", 912, "bad"],
        favouriteIds: ["77", 77, 0],
      },
    },
  });

  assert.deepEqual(next.ui.sharedFishState, {
    caughtIds: [912],
    favouriteIds: [77],
  });
});

test("fish filter terms derive outbound fishIds for the wasm point filter", async () => {
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
        catalog: {
          capabilities: [],
          layers: [],
          patches: [],
          fish: [
            { fishId: 912, name: "Cron Dart", grade: "Rare", isPrize: false },
            { fishId: 77, name: "Serendia Carp", grade: "General", isPrize: false },
          ],
        },
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

    bridge.setState({
      version: 1,
      ui: {
        sharedFishState: {
          caughtIds: [912],
          favouriteIds: [77],
        },
      },
      filters: {
        fishFilterTerms: ["missing"],
      },
    });

    await new Promise((resolve) => setTimeout(resolve, 0));

    assert.deepEqual(bridge.getCurrentInputState().filters.fishIds, []);
    assert.deepEqual(bridge.getCurrentInputState().filters.fishFilterTerms, ["missing"]);
    assert.equal(wasm.calls.applied.length, 1);
    assert.deepEqual(wasm.calls.applied[0].filters.fishIds, [77]);

    wasm.calls.applied.length = 0;
    bridge.setState({
      version: 1,
      ui: {
        sharedFishState: {
          caughtIds: [912],
          favouriteIds: [77],
        },
      },
      filters: {
        fishIds: [77, 912],
        fishFilterTerms: ["favourite"],
      },
    });

    await new Promise((resolve) => setTimeout(resolve, 0));

    assert.deepEqual(bridge.getCurrentInputState().filters.fishIds, [77, 912]);
    assert.deepEqual(bridge.getCurrentInputState().filters.fishFilterTerms, ["favourite"]);
    assert.equal(wasm.calls.applied.length, 1);
    assert.deepEqual(wasm.calls.applied[0].filters.fishIds, [77]);

    wasm.calls.applied.length = 0;
    bridge.setState({
      version: 1,
      ui: {
        sharedFishState: {
          caughtIds: [77],
          favouriteIds: [912],
        },
      },
      filters: {
        fishFilterTerms: ["missing"],
      },
    });

    await new Promise((resolve) => setTimeout(resolve, 0));

    assert.deepEqual(bridge.getCurrentInputState().ui.sharedFishState, {
      caughtIds: [77],
      favouriteIds: [912],
    });
    assert.equal(wasm.calls.applied.length, 1);
    assert.deepEqual(wasm.calls.applied[0].filters.fishIds, [912]);
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("grade fish filter terms derive outbound fishIds with OR semantics", async () => {
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
        catalog: {
          capabilities: [],
          layers: [],
          patches: [],
          fish: [
            { fishId: 912, name: "Cron Dart", grade: "Rare", isPrize: false },
            { fishId: 77, name: "Serendia Carp", grade: "General", isPrize: false },
            { fishId: 61, name: "Ancient Relic Crystal Shard", grade: "Prize", isPrize: true },
          ],
        },
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

    bridge.setState({
      version: 1,
      filters: {
        fishFilterTerms: ["yellow", "green"],
      },
    });

    await new Promise((resolve) => setTimeout(resolve, 0));

    assert.deepEqual(bridge.getCurrentInputState().filters.fishFilterTerms, ["yellow", "green"]);
    assert.equal(wasm.calls.applied.length, 1);
    assert.deepEqual(wasm.calls.applied[0].filters.fishIds, [912, 77]);

    wasm.calls.applied.length = 0;
    bridge.setState({
      version: 1,
      filters: {
        fishFilterTerms: ["red"],
      },
    });

    await new Promise((resolve) => setTimeout(resolve, 0));

    assert.deepEqual(bridge.getCurrentInputState().filters.fishFilterTerms, ["red"]);
    assert.equal(wasm.calls.applied.length, 1);
    assert.deepEqual(wasm.calls.applied[0].filters.fishIds, [61]);
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("semantic field filter patches are normalized and keep zone ids in sync", () => {
  const next = applyStatePatch(undefined, {
    version: 1,
    filters: {
      semanticFieldIdsByLayer: {
        regions: [76, 76, 92],
        zone_mask: [0xc17f7f, 0xc17f7f],
        "": [1],
      },
    },
  });

  assert.deepEqual(next.filters.semanticFieldIdsByLayer, {
    regions: [76, 92],
    zone_mask: [0xc17f7f],
  });
  assert.deepEqual(next.filters.zoneRgbs, [0xc17f7f]);
});

test("setState updates cached input state without forcing a wasm state read", async () => {
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
      debounceMs: 0,
      wasmModule: wasm,
      locationHref: "https://fishystuff.fish/map/",
      localStorage: env.localStorage,
      sessionStorage: env.sessionStorage,
    });
    wasm.calls.stateReads = 0;

    bridge.setState({
      version: 1,
      filters: {
        searchText: "Padjal",
      },
    });

    assert.equal("searchText" in bridge.getCurrentInputState().filters, false);
    assert.equal(wasm.calls.stateReads, 0);
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("bookmark ui patches are normalized in input state and omitted from bridge session snapshots", () => {
  const next = applyStatePatch(undefined, {
    version: 1,
    ui: {
      bookmarkSelectedIds: [" bookmark-a ", "bookmark-a", "bookmark-b", ""],
      bookmarks: [
        {
          id: " bookmark-a ",
          label: " Velia ",
          pointLabel: " Coastal Cliff ",
          worldX: 123.5,
          worldZ: -45.25,
        },
        {
          id: "bookmark-a",
          worldX: 999,
          worldZ: 999,
        },
        {
          id: "",
          worldX: 1,
          worldZ: 2,
        },
      ],
    },
  });

    assert.deepEqual(next.ui.bookmarks, [
      {
        id: "bookmark-a",
        label: "Velia",
        pointLabel: "Coastal Cliff",
        worldX: 123.5,
        worldZ: -45.25,
      },
    ]);
  assert.deepEqual(next.ui.bookmarkSelectedIds, ["bookmark-a", "bookmark-b"]);

  const bridge = createFishyMapBridge();
  bridge.inputState = next;
  const sessionUi = bridge.createSessionSnapshot().ui || {};
  assert.equal("bookmarkSelectedIds" in sessionUi, false);
  assert.equal("bookmarks" in sessionUi, false);
});

test("search text and patch range stay out of persisted bridge session snapshots", () => {
  const bridge = createFishyMapBridge();
  bridge.inputState = applyStatePatch(undefined, {
    version: 1,
    filters: {
      fishIds: [77, 91],
      zoneRgbs: [0xc17f7f, 0x3c963c],
      semanticFieldIdsByLayer: {
        region_groups: [295],
      },
      fishFilterTerms: ["favourite", "missing"],
      searchText: "velia",
      patchId: null,
      fromPatchId: "2026-02-26",
      toPatchId: "2026-03-12",
      layerIdsVisible: ["zones", "terrain"],
      layerIdsOrdered: ["zones", "terrain", "minimap"],
      layerOpacities: { terrain: 0.35 },
      layerClipMasks: { terrain: "zones" },
      layerWaypointConnectionsVisible: { terrain: true },
      layerWaypointLabelsVisible: { terrain: false },
      layerPointIconsVisible: { terrain: true },
      layerPointIconScales: { terrain: 1.5 },
    },
  });

  const sessionFilters = bridge.createSessionSnapshot().filters || {};

  assert.equal("searchText" in sessionFilters, false);
  assert.equal("fishIds" in sessionFilters, false);
  assert.equal("zoneRgbs" in sessionFilters, false);
  assert.equal("semanticFieldIdsByLayer" in sessionFilters, false);
  assert.equal("fishFilterTerms" in sessionFilters, false);
  assert.equal("patchId" in sessionFilters, false);
  assert.equal("fromPatchId" in sessionFilters, false);
  assert.equal("toPatchId" in sessionFilters, false);
  assert.equal("layerIdsVisible" in sessionFilters, false);
  assert.equal("layerIdsOrdered" in sessionFilters, false);
  assert.equal("layerOpacities" in sessionFilters, false);
  assert.equal("layerClipMasks" in sessionFilters, false);
  assert.equal("layerWaypointConnectionsVisible" in sessionFilters, false);
  assert.equal("layerWaypointLabelsVisible" in sessionFilters, false);
  assert.equal("layerPointIconsVisible" in sessionFilters, false);
  assert.equal("layerPointIconScales" in sessionFilters, false);
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
    wasm.calls.stateReads = 0;

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
    assert.equal(received.inputState.ui.viewMode, "3d");
    assert.equal(received.state.view.camera.distance, 5000);
    assert.equal(wasm.calls.stateReads, 0);
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("requestState refreshes the current wasm snapshot before responding", async () => {
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
        statuses: { metaStatus: "meta: old" },
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
    snapshotRef.current = {
      ...snapshotRef.current,
      statuses: { metaStatus: "meta: refreshed" },
    };

    const detail = {};
    container.dispatchEvent(new CustomEvent(FISHYMAP_EVENTS.requestState, { detail }));

    assert.equal(wasm.calls.stateReads, 1);
    assert.equal(detail.state.statuses.metaStatus, "meta: refreshed");
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
          layerSamples: [
            {
              layerId: "zone_mask",
              layerName: "Zone Mask",
              kind: "field",
              rgb: [18, 52, 86],
              rgbU32: 1193046,
              fieldId: 1193046,
              detailSections: [
                {
                  id: "zone",
                  kind: "facts",
                  title: "Zone",
                  facts: [{ key: "zone", label: "Zone", value: "Coastal Shelf", icon: "hover-zone" }],
                  targets: [],
                },
              ],
              targets: [],
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
      layerSamples: [
        {
          layerId: "zone_mask",
          layerName: "Zone Mask",
          kind: "field",
          rgb: [18, 52, 86],
          rgbU32: 1193046,
          fieldId: 1193046,
          detailSections: [
            {
              id: "zone",
              kind: "facts",
              title: "Zone",
              facts: [{ key: "zone", label: "Zone", value: "Coastal Shelf", icon: "hover-zone" }],
              targets: [],
            },
          ],
          targets: [],
        },
      ],
    });
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("selection output events include semantic payload and refreshed state", async () => {
  const env = installDomGlobals();
  let bridge;
  try {
    const canvas = new FakeCanvas();
    const container = new FakeContainer(canvas);
    const semanticSelection = {
      worldX: 11,
      worldZ: 22,
      layerSamples: [
        {
          layerId: "zone_mask",
          layerName: "Zone Mask",
          kind: "field",
          rgb: [18, 52, 86],
          rgbU32: 1193046,
          fieldId: 1193046,
          detailSections: [
            {
              id: "zone",
              kind: "facts",
              title: "Zone",
              facts: [{ key: "zone", label: "Zone", value: "Coastal Shelf", icon: "hover-zone" }],
              targets: [],
            },
          ],
          targets: [],
        },
      ],
    };
    const snapshotRef = {
      current: {
        version: 1,
        ready: true,
        filters: { fishIds: [], searchText: "", patchId: null, layerIdsVisible: [] },
        ui: { diagnosticsOpen: false, legendOpen: false, leftPanelOpen: true },
        view: { viewMode: "2d", camera: {} },
        selection: semanticSelection,
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
    bridge.setState({
      version: 1,
      filters: {
        searchText: "coastal",
        fishIds: [77],
        zoneRgbs: [0xc17f7f],
      },
    });
    wasm.calls.stateReads = 0;

    const received = await new Promise((resolve) => {
      container.addEventListener(
        FISHYMAP_EVENTS.selectionChanged,
        (event) => resolve(event.detail),
        { once: true },
      );
      wasm.calls.sink(
        JSON.stringify({
          type: "selection-changed",
          version: 1,
          worldX: 11,
          worldZ: 22,
          pointKind: "waypoint",
          pointLabel: "Olvia Academy",
          layerSamples: semanticSelection.layerSamples,
        }),
      );
    });

    assert.equal(wasm.calls.stateReads, 1);
    assert.equal(received.zoneRgb, undefined);
    assert.equal(received.worldX, 11);
    assert.equal(received.worldZ, 22);
    assert.equal(received.pointKind, "waypoint");
    assert.equal(received.pointLabel, "Olvia Academy");
    assert.deepEqual(received.layerSamples, semanticSelection.layerSamples);
    assert.deepEqual(received.state.selection, {
      ...semanticSelection,
      pointKind: "waypoint",
      pointLabel: "Olvia Academy",
    });
    assert.deepEqual(received.inputState.filters.fishIds, [77]);
    assert.deepEqual(received.inputState.filters.zoneRgbs, [0xc17f7f]);
    assert.equal("searchText" in received.inputState.filters, false);
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
    assert.ok(wasm.calls.bootstrapReads >= 1);
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("refreshCurrentStateNow forces a wasm read and updates the cached snapshot", async () => {
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
        statuses: { metaStatus: "meta: old" },
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

    snapshotRef.current = {
      ...snapshotRef.current,
      statuses: { metaStatus: "meta: refreshed" },
    };
    wasm.calls.stateReads = 0;

    const refreshed = bridge.refreshCurrentStateNow();

    assert.equal(wasm.calls.stateReads, 1);
    assert.equal(refreshed.statuses.metaStatus, "meta: refreshed");
    assert.equal(bridge.getCurrentState().statuses.metaStatus, "meta: refreshed");
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("getCurrentState refreshes incomplete bootstrap snapshots from wasm", async () => {
  const env = installDomGlobals();
  let bridge;
  try {
    const canvas = new FakeCanvas();
    const container = new FakeContainer(canvas);
    const snapshotRef = {
      current: {
        version: 1,
        ready: true,
        filters: { fishIds: [], searchText: "", patchId: null, layerIdsVisible: ["zone_mask"] },
        ui: { diagnosticsOpen: false, legendOpen: false, leftPanelOpen: true },
        view: { viewMode: "2d", camera: {} },
        selection: {},
        hover: {},
        catalog: {
          capabilities: [],
          layers: [{ layerId: "zone_mask", name: "Zone Mask" }],
          patches: [],
          fish: [{ fishId: 1, name: "A" }],
        },
        statuses: { metaStatus: "meta: loaded", fishStatus: "fish: 1" },
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

    bridge.currentState = {
      version: 1,
      ready: false,
      filters: { fishIds: [], searchText: "", patchId: null, layerIdsVisible: [] },
      ui: { diagnosticsOpen: false, legendOpen: false, leftPanelOpen: true },
      view: { viewMode: "2d", camera: {} },
      selection: {},
      hover: {},
      catalog: { capabilities: [], layers: [], patches: [], fish: [] },
      statuses: { metaStatus: "meta: pending", fishStatus: "fish: pending" },
    };
    wasm.calls.stateReads = 0;

    const current = bridge.getCurrentState();

    assert.equal(wasm.calls.stateReads, 1);
    assert.equal(current.ready, true);
    assert.equal(current.statuses.metaStatus, "meta: loaded");
    assert.equal(current.catalog.layers.length, 1);
    assert.equal(current.catalog.fish.length, 1);
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("getCurrentState refreshes dirty snapshots after live input patches", async () => {
  const env = installDomGlobals();
  let bridge;
  try {
    const canvas = new FakeCanvas();
    const container = new FakeContainer(canvas);
    const snapshotRef = {
      current: {
        version: 1,
        ready: true,
        filters: {
          fishIds: [],
          zoneRgbs: [],
          semanticFieldIdsByLayer: {},
          fishFilterTerms: [],
          patchId: null,
          fromPatchId: null,
          toPatchId: null,
          layerIdsVisible: ["zone_mask", "fish_evidence"],
          layerIdsOrdered: ["bookmarks", "fish_evidence", "zone_mask", "minimap"],
          zoneMembershipLayerIds: [],
        },
        ui: {
          diagnosticsOpen: false,
          showPoints: true,
          showPointIcons: true,
          pointIconScale: 0.5,
          bookmarkSelectedIds: [],
          bookmarks: [],
        },
        view: { viewMode: "2d", camera: {} },
        selection: {},
        hover: {},
        catalog: { capabilities: [], layers: [], patches: [], fish: [] },
        statuses: { metaStatus: "meta: loaded", fishStatus: "fish: loaded" },
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

    wasm.calls.stateReads = 0;
    snapshotRef.current = {
      ...snapshotRef.current,
      filters: {
        ...snapshotRef.current.filters,
        zoneMembershipLayerIds: ["fish_evidence"],
      },
    };

    bridge.setState({
      version: 1,
      filters: {
        zoneMembershipLayerIds: ["fish_evidence"],
      },
    });

    const current = bridge.getCurrentState();

    assert.equal(wasm.calls.stateReads, 1);
    assert.deepEqual(current.filters.zoneMembershipLayerIds, ["fish_evidence"]);
    assert.deepEqual(bridge.getCurrentInputState().filters.zoneMembershipLayerIds, [
      "fish_evidence",
    ]);
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("bootstrap sync uses lightweight polling until the map becomes ready", async () => {
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
        catalog: { capabilities: [], layers: [], patches: [], fish: [{ fishId: 1, name: "A" }] },
        statuses: { metaStatus: "meta: pending" },
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

    const initialStateReads = wasm.calls.stateReads;
    await new Promise((resolve) => setTimeout(resolve, 250));
    assert.equal(
      wasm.calls.stateReads,
      initialStateReads,
      "bootstrap polling should not repeatedly read the full snapshot while loading",
    );
    assert.ok(
      wasm.calls.bootstrapReads >= 1,
      "bootstrap polling should use the lightweight bootstrap getter",
    );

    const readyEvent = new Promise((resolve) => {
      container.addEventListener(FISHYMAP_EVENTS.ready, (event) => resolve(event.detail), {
        once: true,
      });
    });
    snapshotRef.current = {
      ...snapshotRef.current,
      ready: true,
      filters: { fishIds: [], searchText: "", patchId: null, layerIdsVisible: ["zones"] },
      statuses: { metaStatus: "meta: loaded" },
    };

    const detail = await readyEvent;
    assert.equal(detail.state.ready, true);
    assert.deepEqual(detail.state.filters.layerIdsVisible, ["zones"]);
    assert.equal(wasm.calls.stateReads, initialStateReads + 1);
  } finally {
    bridge?.destroy();
    env.restore();
  }
});

test("bootstrap sync refreshes the host cache when fish finishes loading after ready", async () => {
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
        statuses: { metaStatus: "meta: pending", fishStatus: "fish: pending" },
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

    const initialStateReads = wasm.calls.stateReads;
    const readyEvent = new Promise((resolve) => {
      container.addEventListener(FISHYMAP_EVENTS.ready, (event) => resolve(event.detail), {
        once: true,
      });
    });
    snapshotRef.current = {
      ...snapshotRef.current,
      ready: true,
      statuses: { metaStatus: "meta: loaded", fishStatus: "fish: pending" },
    };

    const readyDetail = await readyEvent;
    assert.equal(readyDetail.state.ready, true);
    assert.equal(readyDetail.state.catalog.fish.length, 0);
    assert.equal(wasm.calls.stateReads, initialStateReads + 1);

    const stateChangedEvent = new Promise((resolve) => {
      container.addEventListener(
        FISHYMAP_EVENTS.stateChanged,
        (event) => resolve(event.detail),
        { once: true },
      );
    });
    snapshotRef.current = {
      ...snapshotRef.current,
      catalog: {
        capabilities: [],
        layers: [],
        patches: [],
        fish: [
          { fishId: 1, name: "A" },
          { fishId: 2, name: "B" },
        ],
      },
      statuses: { metaStatus: "meta: loaded", fishStatus: "fish: 2" },
    };

    const stateChangedDetail = await stateChangedEvent;
    assert.equal(stateChangedDetail.state.statuses.fishStatus, "fish: 2");
    assert.equal(stateChangedDetail.state.catalog.fish.length, 2);
    assert.equal(wasm.calls.stateReads, initialStateReads + 2);
    assert.ok(
      wasm.calls.bootstrapReads >= 2,
      "bootstrap polling should continue while the fish catalog is pending",
    );
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

    assert.equal(env.localStorage.getItem("fishystuff.map.prefs.v1"), null);
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

test("buildInitialRestorePatch ignores session storage and page-owned query state", () => {
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
        worldX: 321.5,
        worldZ: -654.25,
      },
      filters: {
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
      "https://fishystuff.fish/map/?fish=77&view=3d&layers=zones,terrain",
    sessionStorage,
  });

  const filters = patch.filters || {};
  const commands = patch.commands || {};
  assert.deepEqual(filters, {});
  assert.equal("layerIdsOrdered" in filters, false);
  assert.equal("layerOpacities" in filters, false);
  assert.equal("layerClipMasks" in filters, false);
  assert.equal("patchId" in filters, false);
  assert.equal("fromPatchId" in filters, false);
  assert.equal("toPatchId" in filters, false);
  assert.equal("ui" in patch, false);
  assert.equal("selectWorldPoint" in commands, false);
  assert.equal("selectZoneRgb" in commands, false);
  assert.equal("setViewMode" in commands, false);
  assert.equal("restoreView" in commands, false);
});

test("normalizeStatePatch keeps minimap layer opacity and clip mask overrides", () => {
  const patch = normalizeStatePatch({
    filters: {
      layerOpacities: {
        minimap: 0.4,
      },
      layerClipMasks: {
        minimap: "zone_mask",
      },
    },
  });

  assert.deepEqual(patch.filters.layerOpacities, { minimap: 0.4 });
  assert.deepEqual(patch.filters.layerClipMasks, { minimap: "zone_mask" });
});

test("buildInitialRestorePatch does not restore page-owned filters from session storage", () => {
  const sessionStorage = new MemoryStorage({
    "fishystuff.map.session.v1": JSON.stringify({
      version: 1,
      filters: {
        fishIds: [44, 55],
        zoneRgbs: [0x123456],
        semanticFieldIdsByLayer: {
          region_groups: [295],
        },
        fishFilterTerms: ["missing"],
        searchText: "session sea",
        fromPatchId: "session-from",
        toPatchId: "session-to",
        layerIdsVisible: ["zones"],
        layerIdsOrdered: ["zones", "minimap"],
        layerOpacities: { zones: 0.6 },
        layerClipMasks: { terrain: "zones" },
        layerWaypointConnectionsVisible: { zones: true },
        layerWaypointLabelsVisible: { zones: false },
        layerPointIconsVisible: { zones: true },
        layerPointIconScales: { zones: 2.2 },
      },
    }),
  });

  const patch = buildInitialRestorePatch({
    locationHref: "https://fishystuff.fish/map/",
    sessionStorage,
  });

  assert.deepEqual(patch.filters || {}, {});
  assert.equal("commands" in patch, false);
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

test("waypoint layer override maps replace the previous map instead of merging stale entries", () => {
  const patch = mergeStatePatch(
    {
      filters: {
        layerWaypointConnectionsVisible: {
          region_nodes: false,
          bookmarks: true,
        },
        layerWaypointLabelsVisible: {
          region_nodes: false,
          bookmarks: true,
        },
      },
    },
    {
      filters: {
        layerWaypointConnectionsVisible: {
          region_nodes: true,
        },
        layerWaypointLabelsVisible: {
          region_nodes: false,
        },
      },
    },
  );

  assert.deepEqual(patch.filters.layerWaypointConnectionsVisible, { region_nodes: true });
  assert.deepEqual(patch.filters.layerWaypointLabelsVisible, { region_nodes: false });
});

test("applyStatePatch keeps waypoint layer override maps in host input state", () => {
  const next = applyStatePatch(
    {},
    {
      filters: {
        layerWaypointConnectionsVisible: {
          region_nodes: false,
        },
        layerWaypointLabelsVisible: {
          region_nodes: false,
        },
      },
    },
  );

  assert.deepEqual(next.filters.layerWaypointConnectionsVisible, { region_nodes: false });
  assert.deepEqual(next.filters.layerWaypointLabelsVisible, { region_nodes: false });
});

test("fish evidence layer icon override maps replace the previous map instead of merging stale entries", () => {
  const patch = mergeStatePatch(
    {
      filters: {
        layerPointIconsVisible: {
          fish_evidence: false,
          region_nodes: true,
        },
        layerPointIconScales: {
          fish_evidence: 1.5,
          region_nodes: 2.5,
        },
      },
    },
    {
      filters: {
        layerPointIconsVisible: {
          fish_evidence: true,
        },
        layerPointIconScales: {
          fish_evidence: 2.25,
        },
      },
    },
  );

  assert.deepEqual(patch.filters.layerPointIconsVisible, { fish_evidence: true });
  assert.deepEqual(patch.filters.layerPointIconScales, { fish_evidence: 2.25 });
});

test("applyStatePatch keeps fish evidence icon override maps in host input state", () => {
  const next = applyStatePatch(
    {},
    {
      filters: {
        layerPointIconsVisible: {
          fish_evidence: false,
        },
        layerPointIconScales: {
          fish_evidence: 2.4,
        },
      },
    },
  );

  assert.deepEqual(next.filters.layerPointIconsVisible, { fish_evidence: false });
  assert.deepEqual(next.filters.layerPointIconScales, { fish_evidence: 2.4 });
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

test("buildInitialRestorePatch ignores session fish selection state", () => {
  const sessionStorage = new MemoryStorage({
    "fishystuff.map.session.v1": JSON.stringify({
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
    sessionStorage,
  });

  assert.deepEqual(patch.filters || {}, {});
  assert.equal("commands" in patch, false);
});

test("buildInitialRestorePatch ignores session zone selection state", () => {
  const sessionStorage = new MemoryStorage({
    "fishystuff.map.session.v1": JSON.stringify({
      version: 1,
      selection: {
        zoneRgb: 0xc17f7f,
      },
      filters: {
        zoneRgbs: [0xc17f7f, 0x3c963c, 0xc17f7f],
      },
    }),
  });

  const patch = buildInitialRestorePatch({
    locationHref: "https://fishystuff.fish/map/",
    sessionStorage,
  });

  assert.deepEqual(patch.filters || {}, {});
  assert.equal("commands" in patch, false);
});

test("buildInitialRestorePatch ignores session semantic selection state", () => {
  const sessionStorage = new MemoryStorage({
    "fishystuff.map.session.v1": JSON.stringify({
      version: 1,
      selection: {
        semanticLayerId: "regions",
        semanticFieldId: 76,
      },
      filters: {
        semanticFieldIdsByLayer: {
          regions: [76, 92],
        },
      },
    }),
  });

  const patch = buildInitialRestorePatch({
    locationHref: "https://fishystuff.fish/map/",
    sessionStorage,
  });

  assert.deepEqual(patch.filters || {}, {});
  assert.equal("commands" in patch, false);
});

test("state patch normalizes selectWorldPoint commands", () => {
  const patch = normalizeStatePatch({
    commands: {
      selectWorldPoint: {
        worldX: "12.5",
        worldZ: "-7.25",
        pointKind: " waypoint ",
        pointLabel: " Olvia Academy ",
      },
    },
  });

  assert.deepEqual(patch.commands.selectWorldPoint, {
    worldX: 12.5,
    worldZ: -7.25,
    pointKind: "waypoint",
    pointLabel: "Olvia Academy",
  });
});

test("state patch normalizes selectSemanticField commands", () => {
  const patch = normalizeStatePatch({
    commands: {
      selectSemanticField: {
        layerId: " region_groups ",
        fieldId: "295",
        targetKey: " resource_node ",
      },
    },
  });

  assert.deepEqual(patch.commands.selectSemanticField, {
    layerId: "region_groups",
    fieldId: 295,
    targetKey: "resource_node",
  });
});

test("host query parsing ignores page-owned patch params", () => {
  const patch = parseQueryState("https://fishystuff.fish/map/?patch=2026-02-26");

  assert.equal("filters" in patch, false);
  assert.equal("ui" in patch, false);
  assert.equal("commands" in patch, false);
});

test("host query parsing ignores page-owned range params", () => {
  const patch = parseQueryState(
    "https://fishystuff.fish/map/?fromPatch=2026-02-26&toPatch=2026-03-12",
  );

  assert.equal("filters" in patch, false);
  assert.equal("ui" in patch, false);
  assert.equal("commands" in patch, false);
});

test("host query parsing ignores page-owned shared filters", () => {
  const patch = parseQueryState(
    "https://fishystuff.fish/map/?fish=77&fishTerms=favourite,missing&search=velia&layers=zones,terrain&diagnostics=true&view=3d",
  );

  assert.equal("filters" in patch, false);
  assert.equal("ui" in patch, false);
  assert.equal("commands" in patch, false);
});

test("query state supports direct world-point selection", () => {
  const patch = parseQueryState(
    "https://fishystuff.fish/map/?worldX=123.4567&worldZ=-45.6789&zone=16711935",
  );

  assert.deepEqual(patch.commands.selectWorldPoint, {
    worldX: 123.4567,
    worldZ: -45.6789,
  });
  assert.equal("selectZoneRgb" in patch.commands, false);
});

test("query state supports direct semantic field selection", () => {
  const patch = parseQueryState(
    "https://fishystuff.fish/map/?semanticLayer=region_groups&semanticField=295&zone=16711935",
  );

  assert.deepEqual(patch.commands.selectSemanticField, {
    layerId: "region_groups",
    fieldId: 295,
  });
  assert.equal("selectZoneRgb" in patch.commands, false);
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

test("API base falls back to local loopback in dev and production otherwise", () => {
  assert.equal(
    resolveApiBaseUrl({ hostname: "localhost", protocol: "http:", href: "http://localhost:1990/map/" }),
    "http://localhost:8080",
  );
  assert.equal(resolveApiBaseUrl({ hostname: "fishystuff.fish" }), "https://api.fishystuff.fish");
});

test("API base prefers an explicit window override", () => {
  const previousWindow = globalThis.window;
  globalThis.window = { __fishystuffApiBaseUrl: "https://override.example.com/" };
  try {
    assert.equal(
      resolveApiBaseUrl({ hostname: "localhost" }),
      "https://override.example.com",
    );
  } finally {
    globalThis.window = previousWindow;
  }
});

test("base URLs prefer runtime config when present", () => {
  const previousWindow = globalThis.window;
  globalThis.window = {
    __fishystuffRuntimeConfig: {
      apiBaseUrl: "http://127.0.0.1:18080/",
      cdnBaseUrl: "http://127.0.0.1:14040/",
    },
  };
  try {
    assert.equal(
      resolveApiBaseUrl({ hostname: "localhost", protocol: "http:", href: "http://localhost:1990/map/" }),
      "http://localhost:18080",
    );
    assert.equal(
      resolveCdnBaseUrl({ hostname: "localhost", protocol: "http:", href: "http://localhost:1990/map/" }),
      "http://localhost:14040",
    );
  } finally {
    globalThis.window = previousWindow;
  }
});

test("CDN base resolves to production or an explicit override", () => {
  assert.equal(
    resolveCdnBaseUrl({ hostname: "localhost", protocol: "http:", href: "http://localhost:1990/map/" }),
    "http://localhost:4040",
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
    resolveMapRuntimeManifestUrl(
      { hostname: "localhost", protocol: "http:", href: "http://localhost:1990/map/" },
      123,
      "http://127.0.0.1:4040",
    ),
    "http://localhost:4040/map/runtime-manifest.123.json",
  );
  assert.equal(
    resolveMapRuntimeManifestUrl({ hostname: "fishystuff.fish" }, "deploy-456"),
    "https://cdn.fishystuff.fish/map/runtime-manifest.deploy-456.json",
  );
  assert.equal(
    resolveMapRuntimeManifestUrl({ hostname: "fishystuff.fish" }, "  release / candidate  "),
    "https://cdn.fishystuff.fish/map/runtime-manifest.release-candidate.json",
  );
  assert.equal(
    resolveMapRuntimeManifestUrl({ hostname: "fishystuff.fish" }, ""),
    "https://cdn.fishystuff.fish/map/runtime-manifest.json",
  );
});

test("performance snapshot merges host and wasm profiling summaries", async () => {
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
        catalog: { capabilities: [], layers: [], patches: [], fish: [{ fishId: 1 }] },
        statuses: { fishStatus: "fish: 1" },
      },
    };
    const wasm = createFakeWasm(snapshotRef);
    wasm.profilingSummary = {
      scenario: "vector_region_groups_enable",
      bevy_version: "0.18.0",
      git_revision: null,
      build_profile: "profiling",
      frames: 120,
      warmup_frames: 12,
      wall_clock_ms: 2500,
      frame_time_ms: { avg: 1.2, p50: 1.1, p95: 2.2, p99: 2.8, max: 4.0 },
      named_spans: {
        "bridge.state_apply": {
          count: 8,
          avg_ms: 0.15,
          p50_ms: 0.1,
          p95_ms: 0.25,
          p99_ms: 0.25,
          max_ms: 0.3,
          total_ms: 1.2,
        },
      },
      counters: {
        "bridge.events.ready": 1,
      },
    };
    bridge = createFishyMapBridge();
    await bridge.mount(container, {
      canvas,
      debounceMs: 0,
      wasmModule: wasm,
      locationHref: "https://fishystuff.fish/map/",
      localStorage: env.localStorage,
      sessionStorage: env.sessionStorage,
    });

    bridge.resetPerformanceSnapshot({
      scenario: "vector_region_groups_enable",
      warmupFrames: 12,
    });
    bridge.sendCommand({ resetView: true });

    const report = bridge.getPerformanceSnapshot();
    assert.deepEqual(wasm.calls.profilingResets.at(-1), {
      scenario: "vector_region_groups_enable",
      warmupFrames: 12,
      captureTrace: false,
    });
    assert.equal(report.scenario, "vector_region_groups_enable");
    assert.equal(report.frames, 120);
    assert.equal(report.warmup_frames, 12);
    assert.equal(report.counters["bridge.events.ready"], 1);
    assert.ok(report.counters["host.commands.sent"] >= 1);
    assert.ok(report.named_spans["host.send_command"]);
    assert.deepEqual(
      report.named_spans["bridge.state_apply"],
      wasm.profilingSummary.named_spans["bridge.state_apply"],
    );
  } finally {
    bridge?.destroy();
    env.restore();
  }
});
