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
        },
        catalog: {
          layers: [
            { layerId: "zone_mask", displayOrder: 20 },
            { layerId: "region_groups", displayOrder: 30 },
            { layerId: "regions", displayOrder: 40 },
          ],
        },
      },
    },
    {
      zoneCatalog: [{ zoneRgb: 0x39e58d, name: "Valencia Sea - Depth 5", biteTimeMin: 5, biteTimeMax: 7 }],
      zoneLootStatus: "loaded",
      zoneLootSummary: {
        available: true,
        profileLabel: "Calculator defaults",
        note: "Zone loot uses calculator default session settings.",
        groups: [
          {
            slotIdx: 4,
            label: "General",
            fillColor: "#eef6ff",
            strokeColor: "#89a8d8",
            textColor: "#1f2937",
            dropRateText: "80%",
            dropRateSourceKind: "database",
            dropRateTooltip: "Source-backed General group share",
          },
        ],
        speciesRows: [
          {
            slotIdx: 4,
            groupLabel: "General",
            label: "Sea Eel",
            iconUrl: "/i/sea-eel.png",
            iconGradeTone: "general",
            fillColor: "#eef6ff",
            strokeColor: "#89a8d8",
            textColor: "#1f2937",
            dropRateText: "80%",
            dropRateSourceKind: "database",
            dropRateTooltip: "DB-backed drop rate",
          },
        ],
      },
    },
  );

  assert.equal(viewModel.descriptor.title, "Valencia Sea - Depth 5");
  assert.equal(viewModel.descriptor.titleIcon, "information-circle");
  assert.equal(viewModel.descriptor.statusIcon, "information-circle");
  assert.deepEqual(viewModel.panes.map((pane) => pane.id), ["zone", "territory", "trade"]);
  assert.equal(viewModel.activePaneId, "territory");
  assert.deepEqual(
    viewModel.panes.find((pane) => pane.id === "zone")?.sections.map((section) => section.kind),
    ["facts", "zone-loot"],
  );
  assert.equal(
    viewModel.panes.find((pane) => pane.id === "zone")?.sections[1]?.title,
    "Catch Profile",
  );
  assert.equal(
    viewModel.panes.find((pane) => pane.id === "zone")?.sections[1]?.groups?.[0]?.rows?.[0]?.label,
    "Sea Eel",
  );
  assert.equal(
    viewModel.panes.find((pane) => pane.id === "zone")?.sections[1]?.groups?.[0]?.dropRateText,
    "80%",
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

test("patchTouchesInfoSignals stays narrow to selection, pane tab, and runtime layer inputs", () => {
  assert.equal(
    patchTouchesInfoSignals({
      _map_runtime: { selection: {} },
    }),
    true,
  );
  assert.equal(
    patchTouchesInfoSignals({
      _map_runtime: { catalog: { layers: [] } },
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

test("buildInfoViewModel falls back to Details when no layer label is available", () => {
  const viewModel = buildInfoViewModel({
    _map_runtime: {
      selection: {
        pointKind: "clicked",
        worldX: 0,
        worldZ: 0,
        layerSamples: [],
      },
    },
  });

  assert.equal(viewModel.descriptor.title, "Details");
  assert.equal(viewModel.descriptor.titleIcon, "information-circle");
  assert.equal(viewModel.descriptor.statusIcon, "information-circle");
});
