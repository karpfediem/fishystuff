import test from "node:test";
import assert from "node:assert/strict";

import { renderLayerStack } from "./map-layer-panel.js";

function createContainer() {
  return {
    dataset: {},
    innerHTML: "",
  };
}

function renderOptions(overrides = {}) {
  return {
    expandedLayerIds: new Set(["zone_mask"]),
    hover: null,
    selection: null,
    zoneCatalog: [{ zoneRgb: 0x39e58d, name: "Valencia Sea - Depth 5" }],
    hoverFactVisibilityByLayer: {},
    renderLoadingPanelMarkup: (label) => label,
    escapeHtml: (value) => String(value ?? ""),
    dragHandleIcon: () => "",
    layerSettingsIcon: () => "",
    eyeIcon: () => "",
    ...overrides,
  };
}

function baseStateBundle() {
  return {
    state: {
      ready: true,
      catalog: {
        layers: [
          {
            layerId: "zone_mask",
            name: "Zone Mask",
            kind: "field",
            visible: true,
            opacity: 1,
            opacityDefault: 1,
            displayOrder: 10,
          },
        ],
      },
    },
    inputState: {
      filters: {
        layerIdsVisible: ["zone_mask"],
      },
    },
  };
}

test("renderLayerStack prefers hover fact preview over selection preview", () => {
  const container = createContainer();
  renderLayerStack(
    container,
    baseStateBundle(),
    renderOptions({
      hover: {
        layerSamples: [{ layerId: "zone_mask", rgb: [57, 229, 141], rgbU32: 0x39e58d, detailSections: [] }],
      },
      selection: {
        layerSamples: [{ layerId: "zone_mask", rgb: [1, 2, 3], rgbU32: 0x010203, detailSections: [] }],
      },
    }),
  );

  assert.match(container.innerHTML, /Valencia Sea - Depth 5/);
  assert.match(container.innerHTML, /57,229,141/);
  assert.doesNotMatch(container.innerHTML, /1,2,3/);
});

test("renderLayerStack rerenders when hover fact preview values change", () => {
  const container = createContainer();
  const stateBundle = baseStateBundle();

  renderLayerStack(
    container,
    stateBundle,
    renderOptions({
      hover: {
        layerSamples: [{ layerId: "zone_mask", rgb: [57, 229, 141], rgbU32: 0x39e58d, detailSections: [] }],
      },
    }),
  );
  const initialMarkup = container.innerHTML;

  renderLayerStack(
    container,
    stateBundle,
    renderOptions({
      hover: {
        layerSamples: [{ layerId: "zone_mask", rgb: [12, 34, 56], rgbU32: 0x0c2238, detailSections: [] }],
      },
      zoneCatalog: [{ zoneRgb: 0x0c2238, name: "Margoria South" }],
    }),
  );

  assert.notEqual(container.innerHTML, initialMarkup);
  assert.match(container.innerHTML, /Margoria South/);
  assert.match(container.innerHTML, /12,34,56/);
});
