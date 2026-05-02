import { test } from "bun:test";
import assert from "node:assert/strict";

import { FISHYMAP_SIGNAL_PATCHED_EVENT } from "./map-signal-patch.js";
import { FISHYMAP_ZONE_CATALOG_READY_EVENT } from "./map-zone-catalog-live.js";

const originalHTMLElement = globalThis.HTMLElement;
const originalDocument = globalThis.document;
const originalRequestAnimationFrame = globalThis.requestAnimationFrame;
const originalCancelAnimationFrame = globalThis.cancelAnimationFrame;

class FakeStyle {
  constructor() {
    this.values = new Map();
  }

  setProperty(name, value) {
    this.values.set(String(name), String(value));
  }
}

class FakeElement extends EventTarget {
  constructor() {
    super();
    this.hidden = false;
    this.innerHTML = "";
    this.dataset = {};
    this.style = new FakeStyle();
    this.id = "";
    this._queryMap = new Map();
    this._closestMap = new Map();
  }

  setQuery(selector, element) {
    this._queryMap.set(selector, element);
  }

  setClosest(selector, element) {
    this._closestMap.set(selector, element);
  }

  querySelector(selector) {
    return this._queryMap.get(selector) || null;
  }

  closest(selector) {
    return this._closestMap.get(selector) || null;
  }

  replaceChildren(...children) {
    this._queryMap.clear();
    for (const child of children) {
      if (child?.id) {
        this._queryMap.set(`#${child.id}`, child);
      }
    }
  }
}

class FakePointerEvent extends Event {
  constructor(type, init = {}) {
    super(type, { bubbles: init.bubbles === true });
    this.clientX = init.clientX ?? 0;
    this.clientY = init.clientY ?? 0;
  }
}

function createDocumentStub() {
  const document = new EventTarget();
  document.createElement = () => new FakeElement();
  document.getElementById = () => null;
  return document;
}

function createSignals() {
  return {
    _map_runtime: {
      catalog: {
        layers: [
          { layerId: "zone_mask", displayOrder: 20 },
          { layerId: "region_groups", displayOrder: 30 },
          { layerId: "regions", displayOrder: 40 },
        ],
        fish: [{ fishId: 10, itemId: 123, name: "Grunt", grade: "white" }],
      },
    },
    _map_bridged: {
      filters: {},
    },
    _map_ui: {
      layers: {
        hoverFactsVisibleByLayer: {
          region_groups: { resource_group: true },
          regions: { origin_region: false },
        },
        sampleHoverVisibleByLayer: {},
      },
    },
  };
}

function hoverPayload() {
  return {
    hover: {
      worldX: 1,
      worldZ: 2,
      layerSamples: [
        {
          layerId: "zone_mask",
          rgb: [57, 229, 141],
          rgbU32: 0x39e58d,
          detailSections: [],
        },
        {
          layerId: "region_groups",
          detailSections: [
            {
              id: "resource-group",
              kind: "facts",
              title: "Resources",
              facts: [
                {
                  key: "resource_group",
                  label: "Resources",
                  value: "(RG212|Arehaza)",
                  icon: "hover-resources",
                },
              ],
              targets: [],
            },
          ],
        },
        {
          layerId: "regions",
          detailSections: [
            {
              id: "origin",
              kind: "facts",
              title: "Origin",
              facts: [
                {
                  key: "origin_region",
                  label: "Origin",
                  value: "(R430|Hakoven Islands)",
                  icon: "trade-origin",
                },
              ],
              targets: [],
            },
          ],
        },
      ],
    },
  };
}

async function loadModule() {
  globalThis.HTMLElement = FakeElement;
  globalThis.document = createDocumentStub();
  return import(`./map-hover-tooltip-element.js?test=${Date.now()}-${Math.random()}`);
}

function createShellAndTooltip(FishyMapHoverTooltipElement) {
  const shell = new FakeElement();
  const canvas = new FakeElement();
  const tooltip = new FishyMapHoverTooltipElement();
  tooltip.id = "fishymap-hover-tooltip";
  tooltip.setClosest("#map-page-shell", shell);
  shell.setQuery("#bevy", canvas);
  return { shell, canvas, tooltip };
}

test("registerFishyMapHoverTooltipElement defines the custom element once", async () => {
  const { registerFishyMapHoverTooltipElement } = await loadModule();
  const registry = {
    definitions: new Map(),
    get(name) {
      return this.definitions.get(name) || null;
    },
    define(name, constructor) {
      this.definitions.set(name, constructor);
    },
  };

  assert.equal(registerFishyMapHoverTooltipElement(registry), true);
  assert.equal(registerFishyMapHoverTooltipElement(registry), true);
  assert.equal(registry.definitions.size, 1);
  assert.ok(registry.get("fishymap-hover-tooltip"));
});

test("FishyMapHoverTooltipElement renders ordered visible hover facts only", async () => {
  const { FishyMapHoverTooltipElement } = await loadModule();
  const { shell, canvas, tooltip } = createShellAndTooltip(FishyMapHoverTooltipElement);
  shell.__fishymapLiveSignals = createSignals();

  tooltip.connectedCallback();
  shell.dispatchEvent(
    new CustomEvent(FISHYMAP_ZONE_CATALOG_READY_EVENT, {
      detail: {
        zoneCatalog: [{ zoneRgb: 0x39e58d, name: "Valencia Sea - Depth 5" }],
      },
    }),
  );

  canvas.dispatchEvent(
    new FakePointerEvent("pointermove", {
      bubbles: true,
      clientX: 120,
      clientY: 160,
    }),
  );
  shell.dispatchEvent(new CustomEvent("fishymap:hover-changed", { detail: hoverPayload() }));

  const layers = tooltip.querySelector("#fishymap-hover-layers");
  const info = tooltip.querySelector("#fishymap-hover-info");
  assert.equal(tooltip.hidden, false);
  assert.equal(layers.hidden, false);
  assert.equal(info.hidden, true);
  assert.match(layers.innerHTML, /Valencia Sea - Depth 5/);
  assert.match(layers.innerHTML, /57,229,141/);
  assert.match(layers.innerHTML, /\(RG212\|Arehaza\)/);
  assert.match(layers.innerHTML, /href="#fishy-hover-zone"/);
  assert.doesNotMatch(layers.innerHTML, /\/img\/icons\.svg/);
  assert.doesNotMatch(layers.innerHTML, /\(R430\|Hakoven Islands\)/);
  assert.ok(
    layers.innerHTML.indexOf("Valencia Sea - Depth 5") < layers.innerHTML.indexOf("(RG212|Arehaza)"),
  );
});

test("FishyMapHoverTooltipElement renders concrete landmark hover rows separately from layer facts", async () => {
  const { FishyMapHoverTooltipElement } = await loadModule();
  const { shell, canvas, tooltip } = createShellAndTooltip(FishyMapHoverTooltipElement);
  shell.__fishymapLiveSignals = createSignals();

  tooltip.connectedCallback();
  canvas.dispatchEvent(new FakePointerEvent("pointermove", { bubbles: true, clientX: 120, clientY: 160 }));
  shell.dispatchEvent(
    new CustomEvent("fishymap:hover-changed", {
      detail: {
        hover: {
          worldX: 10,
          worldZ: 20,
          layerSamples: [
            {
              layerId: "trade_npcs",
              targets: [{ key: "trade_npc", label: "Bahar", worldX: 10, worldZ: 20 }],
              detailSections: [],
            },
            {
              layerId: "bookmarks",
              targets: [{ key: "bookmark", label: "Saved Hotspot", worldX: 30, worldZ: 40 }],
              detailSections: [],
            },
            {
              layerId: "regions",
              targets: [{ key: "origin_node", label: "Origin: Margoria (R829)", worldX: 1, worldZ: 2 }],
              detailSections: [],
            },
            {
              layerId: "region_groups",
              targets: [{ key: "resource_node", label: "Resources: Margoria (RG218)", worldX: 1, worldZ: 2 }],
              detailSections: [],
            },
          ],
        },
      },
    }),
  );

  const layers = tooltip.querySelector("#fishymap-hover-layers");
  const info = tooltip.querySelector("#fishymap-hover-info");
  assert.equal(tooltip.hidden, false);
  assert.equal(layers.hidden, true);
  assert.equal(info.hidden, false);
  assert.equal(tooltip.dataset.landmarkHover, "true");
  assert.equal(tooltip.dataset.pointSamples, undefined);
  assert.match(info.innerHTML, /data-hover-row-kind="landmark-hover"/);
  assert.match(info.innerHTML, /NPC/);
  assert.match(info.innerHTML, /Bahar/);
  assert.match(info.innerHTML, /Bookmark/);
  assert.match(info.innerHTML, /Saved Hotspot/);
  assert.doesNotMatch(info.innerHTML, /Origin: Margoria/);
  assert.doesNotMatch(info.innerHTML, /Resources: Margoria/);
});

test("FishyMapHoverTooltipElement renders fish sample landmarks without layer facts", async () => {
  const { FishyMapHoverTooltipElement } = await loadModule();
  const { shell, canvas, tooltip } = createShellAndTooltip(FishyMapHoverTooltipElement);
  shell.__fishymapLiveSignals = createSignals();

  tooltip.connectedCallback();
  canvas.dispatchEvent(new FakePointerEvent("pointermove", { bubbles: true, clientX: 50, clientY: 70 }));
  shell.dispatchEvent(
    new CustomEvent("fishymap:hover-changed", {
      detail: {
        hover: {
          pointSamples: [
            {
              fishId: 10,
              sampleCount: 2,
              lastTsUtc: 1_700_000_000,
              zoneRgbs: [0x39e58d],
              fullZoneRgbs: [0x39e58d],
            },
          ],
          layerSamples: [],
        },
      },
    }),
  );

  const layers = tooltip.querySelector("#fishymap-hover-layers");
  const info = tooltip.querySelector("#fishymap-hover-info");
  assert.equal(tooltip.hidden, false);
  assert.equal(layers.hidden, true);
  assert.equal(info.hidden, false);
  assert.equal(tooltip.dataset.landmarkHover, "true");
  assert.equal(tooltip.dataset.pointSamples, "true");
  assert.match(info.innerHTML, /fishymap-point-sample-group/);
  assert.match(info.innerHTML, /Grunt/);
  assert.match(info.innerHTML, /x2/);
  assert.doesNotMatch(info.innerHTML, /data-hover-row-kind="landmark-hover"/);
});

test("FishyMapHoverTooltipElement hides fish sample landmarks when sample hover is disabled", async () => {
  const { FishyMapHoverTooltipElement } = await loadModule();
  const { shell, canvas, tooltip } = createShellAndTooltip(FishyMapHoverTooltipElement);
  const signals = createSignals();
  signals._map_runtime.catalog.layers = [];
  signals._map_ui.layers.hoverFactsVisibleByLayer = {};
  signals._map_ui.layers.sampleHoverVisibleByLayer = { fish_evidence: false };
  shell.__fishymapLiveSignals = signals;

  tooltip.connectedCallback();
  canvas.dispatchEvent(new FakePointerEvent("pointermove", { bubbles: true, clientX: 50, clientY: 70 }));
  shell.dispatchEvent(
    new CustomEvent("fishymap:hover-changed", {
      detail: {
        hover: {
          pointSamples: [
            {
              fishId: 10,
              sampleCount: 2,
              lastTsUtc: 1_700_000_000,
              zoneRgbs: [0x39e58d],
              fullZoneRgbs: [0x39e58d],
            },
          ],
          layerSamples: [],
        },
      },
    }),
  );

  const layers = tooltip.querySelector("#fishymap-hover-layers");
  const info = tooltip.querySelector("#fishymap-hover-info");
  assert.equal(tooltip.hidden, true);
  assert.equal(layers.hidden, true);
  assert.equal(info.hidden, true);
  assert.equal(tooltip.dataset.landmarkHover, undefined);
  assert.equal(tooltip.dataset.pointSamples, undefined);
});

test("FishyMapHoverTooltipElement hides the tooltip on pointerleave", async () => {
  const { FishyMapHoverTooltipElement } = await loadModule();
  const { shell, canvas, tooltip } = createShellAndTooltip(FishyMapHoverTooltipElement);
  shell.__fishymapLiveSignals = createSignals();

  tooltip.connectedCallback();
  canvas.dispatchEvent(new FakePointerEvent("pointermove", { bubbles: true, clientX: 10, clientY: 20 }));
  shell.dispatchEvent(new CustomEvent("fishymap:hover-changed", { detail: hoverPayload() }));
  assert.equal(tooltip.hidden, false);

  canvas.dispatchEvent(new Event("pointerleave"));
  assert.equal(tooltip.hidden, true);
  tooltip.render();
});

test("FishyMapHoverTooltipElement batches visible pointer position writes", async () => {
  try {
    globalThis.requestAnimationFrame = undefined;
    globalThis.cancelAnimationFrame = undefined;
    const { FishyMapHoverTooltipElement } = await loadModule();
    const { shell, canvas, tooltip } = createShellAndTooltip(FishyMapHoverTooltipElement);
    shell.__fishymapLiveSignals = createSignals();

    tooltip.connectedCallback();
    canvas.dispatchEvent(new FakePointerEvent("pointermove", { bubbles: true, clientX: 10, clientY: 20 }));
    shell.dispatchEvent(new CustomEvent("fishymap:hover-changed", { detail: hoverPayload() }));
    const layers = tooltip.querySelector("#fishymap-hover-layers");
    const info = tooltip.querySelector("#fishymap-hover-info");
    assert.equal(layers.style.transform, "translate3d(28px, 42px, 0)");
    assert.equal(info.style.transform, "translate3d(calc(10px - 50%), calc(20px - 100% - 18px), 0)");

    const scheduled = [];
    globalThis.requestAnimationFrame = (callback) => {
      scheduled.push(callback);
      return scheduled.length;
    };
    globalThis.cancelAnimationFrame = () => {};

    canvas.dispatchEvent(new FakePointerEvent("pointermove", { bubbles: true, clientX: 20, clientY: 30 }));
    canvas.dispatchEvent(new FakePointerEvent("pointermove", { bubbles: true, clientX: 40, clientY: 50 }));

    assert.equal(scheduled.length, 1);
    assert.equal(layers.style.transform, "translate3d(28px, 42px, 0)");
    scheduled.shift()();
    assert.equal(layers.style.transform, "translate3d(58px, 72px, 0)");
    assert.equal(info.style.transform, "translate3d(calc(40px - 50%), calc(50px - 100% - 18px), 0)");
  } finally {
    globalThis.requestAnimationFrame = originalRequestAnimationFrame;
    globalThis.cancelAnimationFrame = originalCancelAnimationFrame;
  }
});

test("FishyMapHoverTooltipElement rerenders on shell-local patch events", async () => {
  const { FishyMapHoverTooltipElement } = await loadModule();
  const { shell, canvas, tooltip } = createShellAndTooltip(FishyMapHoverTooltipElement);
  const signals = createSignals();
  shell.__fishymapLiveSignals = signals;

  tooltip.connectedCallback();
  shell.dispatchEvent(
    new CustomEvent(FISHYMAP_ZONE_CATALOG_READY_EVENT, {
      detail: {
        zoneCatalog: [{ zoneRgb: 0x39e58d, name: "Valencia Sea - Depth 5" }],
      },
    }),
  );

  canvas.dispatchEvent(new FakePointerEvent("pointermove", { bubbles: true, clientX: 10, clientY: 20 }));
  shell.dispatchEvent(new CustomEvent("fishymap:hover-changed", { detail: hoverPayload() }));

  const layers = tooltip.querySelector("#fishymap-hover-layers");
  assert.doesNotMatch(layers.innerHTML, /\(R430\|Hakoven Islands\)/);

  signals._map_ui.layers.hoverFactsVisibleByLayer.regions.origin_region = true;
  shell.dispatchEvent(
    new CustomEvent(FISHYMAP_SIGNAL_PATCHED_EVENT, {
      detail: {
        _map_ui: {
          layers: {
            hoverFactsVisibleByLayer: {
              regions: {
                origin_region: true,
              },
            },
          },
        },
      },
    }),
  );

  assert.match(layers.innerHTML, /\(R430\|Hakoven Islands\)/);

  signals._map_runtime.catalog.layers = [
    { layerId: "region_groups", displayOrder: 20 },
    { layerId: "zone_mask", displayOrder: 30 },
    { layerId: "regions", displayOrder: 40 },
  ];
  shell.dispatchEvent(
    new CustomEvent(FISHYMAP_SIGNAL_PATCHED_EVENT, {
      detail: {
        _map_runtime: {
          catalog: {
            layers: signals._map_runtime.catalog.layers,
          },
        },
      },
    }),
  );

  assert.ok(
    layers.innerHTML.indexOf("(RG212|Arehaza)") < layers.innerHTML.indexOf("Valencia Sea - Depth 5"),
  );
});

process.on("exit", () => {
  globalThis.HTMLElement = originalHTMLElement;
  globalThis.document = originalDocument;
});
