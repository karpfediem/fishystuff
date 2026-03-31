import test from "node:test";
import assert from "node:assert/strict";

import {
  buildHoverTooltipRows,
  buildLayerHoverSettingsRows,
  patchTouchesHoverTooltipSignals,
} from "./map-hover-facts.js";

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

test("buildHoverTooltipRows follows layer order from lowest layer first", () => {
  const rows = buildHoverTooltipRows({
    hover: {
      layerSamples: [
        {
          layerId: "regions",
          detailSections: [detailSectionFact("origin_region", "Origin", "(R430|Hakoven Islands)", "trade-origin")],
        },
        {
          layerId: "zone_mask",
          rgb: [57, 229, 141],
          rgbU32: 0x39e58d,
          detailSections: [],
        },
        {
          layerId: "region_groups",
          detailSections: [
            detailSectionFact("resource_group", "Resources", "(RG212|Arehaza)", "hover-resources"),
          ],
        },
      ],
    },
    stateBundle: {
      state: {
        catalog: {
          layers: [
            { layerId: "zone_mask", displayOrder: 20 },
            { layerId: "region_groups", displayOrder: 30 },
            { layerId: "regions", displayOrder: 40 },
          ],
        },
      },
      inputState: {
        filters: {},
      },
    },
    visibilityByLayer: {
      region_groups: { resource_group: true },
      regions: { origin_region: true },
    },
    zoneCatalog: [{ zoneRgb: 0x39e58d, name: "Valencia Sea - Depth 5" }],
  });

  assert.deepEqual(
    rows.map((row) => [row.layerId, row.key, row.value]),
    [
      ["zone_mask", "zone", "Valencia Sea - Depth 5"],
      ["zone_mask", "rgb", "57,229,141"],
      ["region_groups", "resource_group", "(RG212|Arehaza)"],
      ["regions", "origin_region", "(R430|Hakoven Islands)"],
    ],
  );
});

test("buildLayerHoverSettingsRows keeps page-owned defaults and preview values", () => {
  const rows = buildLayerHoverSettingsRows({
    layerId: "regions",
    sample: {
      layerId: "regions",
      detailSections: [
        detailSectionFact("origin_region", "Origin", "(R430|Hakoven Islands)", "trade-origin"),
      ],
    },
    visibilityByLayer: {},
  });

  assert.deepEqual(rows, [
    {
      key: "origin_region",
      name: "Origin",
      label: "Origin",
      value: "(R430|Hakoven Islands)",
      icon: "trade-origin",
      defaultVisible: false,
      enabled: false,
    },
  ]);
});

test("patchTouchesHoverTooltipSignals stays narrow", () => {
  assert.equal(
    patchTouchesHoverTooltipSignals({
      _map_ui: {
        layers: {
          hoverFactsVisibleByLayer: {
            regions: { origin_region: true },
          },
        },
      },
    }),
    true,
  );
  assert.equal(
    patchTouchesHoverTooltipSignals({
      _map_bridged: {
        filters: {
          layerIdsOrdered: ["zone_mask"],
        },
      },
    }),
    true,
  );
  assert.equal(
    patchTouchesHoverTooltipSignals({
      _map_runtime: {
        catalog: {
          layers: [],
        },
      },
    }),
    true,
  );
  assert.equal(
    patchTouchesHoverTooltipSignals({
      _map_runtime: {
        selection: {},
      },
    }),
    false,
  );
});
