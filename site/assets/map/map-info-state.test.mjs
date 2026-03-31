import test from "node:test";
import assert from "node:assert/strict";

import { buildInfoViewModel, patchTouchesInfoSignals } from "./map-info-state.js";

function detailSectionFact(key, label, value, icon) {
  return {
    id: key,
    kind: "facts",
    title: label,
    facts: [
      {
        key,
        label,
        value,
        icon,
      },
    ],
    targets: [],
  };
}

test("buildInfoViewModel groups selection data into zone, territory, and trade panes", () => {
  const viewModel = buildInfoViewModel(
    {
      _map_ui: {
        windowUi: {
          zoneInfo: { tab: "territory" },
        },
      },
      _map_runtime: {
        selection: {
          pointKind: "clicked",
          pointLabel: "Valencia Sea - Depth 5",
          layerSamples: [
            {
              layerId: "zone_mask",
              rgbU32: 0x39e58d,
              rgb: [57, 229, 141],
              detailSections: [detailSectionFact("zone", "Zone", "Valencia Sea - Depth 5", "hover-zone")],
            },
            {
              layerId: "region_groups",
              detailSections: [detailSectionFact("resource_group", "Resources", "(RG212|Arehaza)", "hover-resources")],
            },
            {
              layerId: "regions",
              detailSections: [detailSectionFact("origin_region", "Origin", "(R430|Hakoven Islands)", "trade-origin")],
            },
          ],
          zoneStats: {
            confidence: {
              status: "FRESH",
              ess: 42.5,
              totalWeight: 71.3,
              notes: ["recent ranking evidence"],
            },
            distribution: [
              {
                fishId: 41,
                fishName: "Yellowfin Sole",
                pMean: 0.24,
                evidenceWeight: 15.6,
                ciLow: 0.18,
                ciHigh: 0.31,
              },
            ],
          },
        },
        catalog: {
          layers: [
            { layerId: "zone_mask", displayOrder: 20 },
            { layerId: "region_groups", displayOrder: 30 },
            { layerId: "regions", displayOrder: 40 },
          ],
          fish: [{ fishId: 41, name: "Yellowfin Sole", itemId: 9041 }],
        },
        statuses: {
          zoneStatsStatus: "zone stats: loaded",
        },
      },
    },
    {
      zoneCatalog: [{ zoneRgb: 0x39e58d, name: "Valencia Sea - Depth 5", biteTimeMin: 5, biteTimeMax: 7 }],
    },
  );

  assert.equal(viewModel.descriptor.title, "Valencia Sea - Depth 5");
  assert.deepEqual(viewModel.panes.map((pane) => pane.id), ["zone", "territory", "trade"]);
  assert.equal(viewModel.activePaneId, "territory");
  assert.deepEqual(
    viewModel.panes.find((pane) => pane.id === "zone")?.sections.map((section) => section.kind),
    ["facts", "evidence"],
  );
  assert.deepEqual(
    viewModel.panes.find((pane) => pane.id === "territory")?.sections[0].facts,
    [
      {
        key: "resources",
        icon: "hover-resources",
        label: "Resources",
        value: "(RG212|Arehaza)",
      },
    ],
  );
  assert.deepEqual(
    viewModel.panes.find((pane) => pane.id === "trade")?.sections[0].facts,
    [
      {
        key: "origin",
        icon: "trade-origin",
        label: "Origin",
        value: "(R430|Hakoven Islands)",
      },
    ],
  );
});

test("patchTouchesInfoSignals stays narrow to selection, pane tab, and zone-stats inputs", () => {
  assert.equal(
    patchTouchesInfoSignals({
      _map_runtime: { selection: {} },
    }),
    true,
  );
  assert.equal(
    patchTouchesInfoSignals({
      _map_runtime: { statuses: { zoneStatsStatus: "loaded" } },
    }),
    true,
  );
  assert.equal(
    patchTouchesInfoSignals({
      _map_ui: { windowUi: { zoneInfo: { tab: "trade" } } },
    }),
    true,
  );
  assert.equal(
    patchTouchesInfoSignals({
      _map_ui: { search: { open: true } },
    }),
    false,
  );
});
