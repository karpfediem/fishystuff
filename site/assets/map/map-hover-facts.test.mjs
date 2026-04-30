import { test } from "bun:test";
import assert from "node:assert/strict";
import { installMapTestI18n } from "./test-i18n.js";

import {
  buildHoverTooltipRows,
  buildLayerHoverSettingsRows,
  buildLayerPanelHoverFactPreview,
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

test("buildHoverTooltipRows prepends ranking sample summaries sorted by occurrence", () => {
  globalThis.window = globalThis.window || {};
  globalThis.window.__fishystuffResolveFishItemIconUrl = (itemId) => `/items/${itemId}.webp`;
  const rows = buildHoverTooltipRows({
    hover: {
      pointSamples: [
        {
          fishId: 20,
          sampleCount: 2,
          lastTsUtc: 1_700_100_000,
          zoneRgbs: [0x39e58d, 0x654321],
          fullZoneRgbs: [],
        },
        {
          fishId: 10,
          sampleCount: 3,
          lastTsUtc: 1_700_000_000,
          zoneRgbs: [0x39e58d],
          fullZoneRgbs: [0x39e58d],
        },
      ],
      layerSamples: [
        {
          layerId: "zone_mask",
          rgb: [57, 229, 141],
          rgbU32: 0x39e58d,
          detailSections: [],
        },
      ],
    },
    stateBundle: {
      state: {
        catalog: {
          fish: [
            { fishId: 10, itemId: 900010, name: "Sea Eel", grade: "general" },
            { fishId: 20, itemId: 900020, name: "Mako Shark", grade: "rare" },
          ],
          layers: [{ layerId: "zone_mask", displayOrder: 20 }],
        },
      },
      inputState: { filters: {} },
    },
    zoneCatalog: [
      { zoneRgb: 0x39e58d, name: "Velia Coast" },
      { zoneRgb: 0x123456, name: "Demi River" },
      { zoneRgb: 0x654321, name: "Balenos River" },
    ],
  });

  assert.deepEqual(
    rows.slice(0, 3).map((row) => [row.kind || "fact", row.fishName || row.value, row.sampleCount || 0]),
    [
      ["point-sample", "Sea Eel", 3],
      ["point-sample", "Mako Shark", 2],
      ["fact", "Velia Coast", 0],
    ],
  );
  assert.equal(rows[0].dateText, "2023-11-14");
  assert.equal(rows[0].zoneKind, "full");
  assert.deepEqual(rows[0].zones.map((zone) => zone.name), []);
  assert.equal(rows[0].iconUrl, "/items/900010.webp");
  assert.equal(rows[1].zoneKind, "partial");
  assert.deepEqual(rows[1].zones.map((zone) => zone.name), ["Balenos River"]);
});

test("buildHoverTooltipRows can suppress ranking sample summaries", () => {
  const rows = buildHoverTooltipRows({
    hover: {
      pointSamples: [
        {
          fishId: 10,
          sampleCount: 3,
          lastTsUtc: 1_700_000_000,
          zoneRgbs: [0x39e58d],
          fullZoneRgbs: [0x39e58d],
        },
      ],
      layerSamples: [],
    },
    pointSamplesEnabled: false,
  });

  assert.deepEqual(rows, []);
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

test("buildLayerPanelHoverFactPreview prefers live hover samples over selection fallback", () => {
  const rows = buildLayerPanelHoverFactPreview({
    layerId: "regions",
    hover: {
      layerSamples: [
        {
          layerId: "regions",
          detailSections: [
            detailSectionFact("origin_region", "Origin", "(R430|Hakoven Islands)", "trade-origin"),
          ],
        },
      ],
    },
    selection: {
      layerSamples: [
        {
          layerId: "regions",
          detailSections: [
            detailSectionFact("origin_region", "Origin", "(R17|Altinova)", "trade-origin"),
          ],
        },
      ],
    },
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
        catalog: {
          fish: [],
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
installMapTestI18n();
