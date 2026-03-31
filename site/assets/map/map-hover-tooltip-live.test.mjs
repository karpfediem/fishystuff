import test from "node:test";
import assert from "node:assert/strict";

import { createMapHoverTooltipController } from "./map-hover-tooltip-live.js";

const originalHTMLElement = globalThis.HTMLElement;

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
    this._queryMap = new Map();
  }

  setQuery(selector, element) {
    this._queryMap.set(selector, element);
  }

  querySelector(selector) {
    return this._queryMap.get(selector) || null;
  }
}

globalThis.HTMLElement = FakeElement;

class FakePointerEvent extends Event {
  constructor(type, init = {}) {
    super(type, { bubbles: init.bubbles === true });
    this.clientX = init.clientX ?? 0;
    this.clientY = init.clientY ?? 0;
  }
}

function createShell() {
  const shell = new FakeElement();
  const canvas = new FakeElement();
  const tooltip = new FakeElement();
  const layers = new FakeElement();
  shell.setQuery("#bevy", canvas);
  shell.setQuery("#fishymap-hover-tooltip", tooltip);
  shell.setQuery("#fishymap-hover-layers", layers);
  return { shell, canvas, tooltip, layers };
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

test("createMapHoverTooltipController renders ordered visible hover facts only", () => {
  const { shell, canvas, tooltip, layers } = createShell();
  const signals = createSignals();
  const controller = createMapHoverTooltipController({
    shell,
    getSignals: () => signals,
    canvas,
    requestAnimationFrameImpl: null,
    listenToSignalPatches: false,
  });
  controller.setZoneCatalog([{ zoneRgb: 0x39e58d, name: "Valencia Sea - Depth 5" }]);

  canvas.dispatchEvent(
    new FakePointerEvent("pointermove", {
      bubbles: true,
      clientX: 120,
      clientY: 160,
    }),
  );
  shell.dispatchEvent(new CustomEvent("fishymap:hover-changed", { detail: hoverPayload() }));

  assert.equal(tooltip.hidden, false);
  assert.equal(layers.hidden, false);
  assert.match(layers.innerHTML, /Valencia Sea - Depth 5/);
  assert.match(layers.innerHTML, /57,229,141/);
  assert.match(layers.innerHTML, /\(RG212\|Arehaza\)/);
  assert.doesNotMatch(layers.innerHTML, /\(R430\|Hakoven Islands\)/);
  assert.ok(
    layers.innerHTML.indexOf("Valencia Sea - Depth 5") < layers.innerHTML.indexOf("(RG212|Arehaza)"),
  );
});

test("createMapHoverTooltipController hides the tooltip on pointerleave", () => {
  const { shell, canvas, tooltip } = createShell();
  const controller = createMapHoverTooltipController({
    shell,
    getSignals: () => createSignals(),
    canvas,
    requestAnimationFrameImpl: null,
    listenToSignalPatches: false,
  });

  canvas.dispatchEvent(new FakePointerEvent("pointermove", { bubbles: true, clientX: 10, clientY: 20 }));
  shell.dispatchEvent(new CustomEvent("fishymap:hover-changed", { detail: hoverPayload() }));
  assert.equal(tooltip.hidden, false);

  canvas.dispatchEvent(new Event("pointerleave"));
  assert.equal(tooltip.hidden, true);
  controller.render();
});

process.on("exit", () => {
  globalThis.HTMLElement = originalHTMLElement;
});
