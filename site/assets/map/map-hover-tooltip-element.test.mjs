import { test } from "bun:test";
import assert from "node:assert/strict";

import { FISHYMAP_SIGNAL_PATCHED_EVENT } from "./map-signal-patch.js";
import { FISHYMAP_ZONE_CATALOG_READY_EVENT } from "./map-zone-catalog-live.js";

const originalHTMLElement = globalThis.HTMLElement;
const originalDocument = globalThis.document;

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
  const samples = tooltip.querySelector("#fishymap-hover-samples");
  assert.equal(tooltip.hidden, false);
  assert.equal(layers.hidden, false);
  assert.equal(samples.hidden, true);
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

test("FishyMapHoverTooltipElement renders ranking point sample summaries", async () => {
  globalThis.window = globalThis.window || {};
  globalThis.window.__fishystuffResolveFishItemIconUrl = (itemId) => `/items/${itemId}.webp`;
  const { FishyMapHoverTooltipElement } = await loadModule();
  const { shell, canvas, tooltip } = createShellAndTooltip(FishyMapHoverTooltipElement);
  shell.__fishymapLiveSignals = {
    ...createSignals(),
    _map_runtime: {
      catalog: {
        layers: [],
        fish: [
          { fishId: 10, itemId: 900010, name: "Sea Eel", grade: "general" },
          { fishId: 20, itemId: 900020, name: "Mako Shark", grade: "rare" },
        ],
      },
    },
  };

  tooltip.connectedCallback();
  shell.dispatchEvent(
    new CustomEvent(FISHYMAP_ZONE_CATALOG_READY_EVENT, {
      detail: {
        zoneCatalog: [
          { zoneRgb: 0x39e58d, name: "Velia Coast" },
          { zoneRgb: 0x123456, name: "Demi River" },
          { zoneRgb: 0x654321, name: "Balenos River" },
        ],
      },
    }),
  );
  canvas.dispatchEvent(new FakePointerEvent("pointermove", { bubbles: true, clientX: 50, clientY: 70 }));
  shell.dispatchEvent(
    new CustomEvent("fishymap:hover-changed", {
      detail: {
        hover: {
          pointSamples: [
            {
              fishId: 20,
              sampleCount: 1,
              lastTsUtc: 1_700_200_000,
              zoneRgbs: [0x123456, 0x654321],
              fullZoneRgbs: [],
            },
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
  const samples = tooltip.querySelector("#fishymap-hover-samples");
  assert.equal(tooltip.hidden, false);
  assert.equal(layers.hidden, true);
  assert.equal(samples.hidden, false);
  assert.equal(tooltip.dataset.pointSamples, "true");
  assert.match(samples.innerHTML, /Sea Eel/);
  assert.match(samples.innerHTML, /Mako Shark/);
  assert.match(samples.innerHTML, /fishy-item-icon-frame is-native/);
  assert.match(samples.innerHTML, /fishy-item-icon-frame is-hover-rest/);
  assert.ok(samples.innerHTML.indexOf("Sea Eel") < samples.innerHTML.indexOf("Mako Shark"));
  assert.match(samples.innerHTML, /x2/);
  assert.match(samples.innerHTML, /2023-11-14/);
  assert.match(samples.innerHTML, /Velia Coast/);
  assert.doesNotMatch(samples.innerHTML, /Demi River/);
  assert.doesNotMatch(samples.innerHTML, /Balenos River/);
  assert.match(samples.innerHTML, /href="#fishy-date-confirmed"/);
});

test("FishyMapHoverTooltipElement hides ranking sample summaries when disabled", async () => {
  const { FishyMapHoverTooltipElement } = await loadModule();
  const { shell, canvas, tooltip } = createShellAndTooltip(FishyMapHoverTooltipElement);
  shell.__fishymapLiveSignals = {
    ...createSignals(),
    _map_runtime: {
      catalog: {
        layers: [],
        fish: [{ fishId: 10, itemId: 900010, name: "Sea Eel", grade: "general" }],
      },
    },
    _map_ui: {
      layers: {
        hoverFactsVisibleByLayer: {},
        sampleHoverVisibleByLayer: { fish_evidence: false },
      },
    },
  };

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
  const samples = tooltip.querySelector("#fishymap-hover-samples");
  assert.equal(tooltip.hidden, true);
  assert.equal(layers.hidden, true);
  assert.equal(samples.hidden, true);
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
