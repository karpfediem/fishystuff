import test from "node:test";
import assert from "node:assert/strict";

import {
  buildZoneInfoViewModel,
  patchTouchesZoneInfoSignals,
} from "./map-zone-info-state.js";

test("buildZoneInfoViewModel derives descriptor, tabs, and facts from selection", () => {
  const viewModel = buildZoneInfoViewModel({
    _map_runtime: {
      selection: {
        pointKind: "clicked",
        pointLabel: "Cron Castle",
        worldX: 123.4,
        worldZ: 456.7,
        layerSamples: [
          { layerId: "zone_mask", label: "Cron Castle", rgbU32: 123456 },
          { layerId: "regions", fieldId: 57, fieldLabel: "Cron Islands" },
        ],
      },
      catalog: {
        layers: [
          { layerId: "zone_mask", name: "Zone Mask" },
          { layerId: "regions", name: "Regions" },
        ],
      },
    },
    _map_ui: {
      windowUi: {
        zoneInfo: {
          tab: "regions",
        },
      },
    },
  });

  assert.equal(viewModel.descriptor.title, "Cron Castle");
  assert.equal(viewModel.descriptor.statusText, "Clicked point");
  assert.deepEqual(
    viewModel.tabs.map((tab) => tab.id),
    ["zone_mask", "regions"],
  );
  assert.equal(viewModel.activeTabId, "regions");
  assert.equal(viewModel.empty, false);
  assert.equal(viewModel.facts.some((fact) => fact.label === "Layer" && fact.value === "Regions"), true);
});

test("patchTouchesZoneInfoSignals stays scoped to zone info branches", () => {
  assert.equal(patchTouchesZoneInfoSignals({ _map_runtime: { selection: {} } }), true);
  assert.equal(patchTouchesZoneInfoSignals({ _map_ui: { windowUi: { zoneInfo: { tab: "x" } } } }), true);
  assert.equal(patchTouchesZoneInfoSignals({ _map_runtime: { statuses: { layersStatus: "ok" } } }), false);
});
