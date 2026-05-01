import { test } from "bun:test";
import assert from "node:assert/strict";
import { installMapTestI18n } from "./test-i18n.js";

import {
  buildOverviewRowsForLayerSamples,
  preferredPointLabelForLayerSamples,
  buildTradePaneFacts,
  buildTerritoryPaneFacts,
  buildZonePaneFacts,
  preferredOverviewRow,
} from "./map-overview-facts.js";

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

test("buildOverviewRowsForLayerSamples follows semantic layer order from lowest layer first", () => {
  const rows = buildOverviewRowsForLayerSamples(
    [
      {
        layerId: "regions",
        detailSections: [detailSectionFact("origin_region", "Origin", "(R430|Hakoven Islands)", "trade-origin")],
      },
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
    ],
    {
      runtimeLayers: [
        { layerId: "zone_mask", displayOrder: 20 },
        { layerId: "region_groups", displayOrder: 30 },
        { layerId: "regions", displayOrder: 40 },
      ],
    },
  );

  assert.deepEqual(
    rows.map((row) => [row.key, row.value]),
    [
      ["zone", "Valencia Sea - Depth 5"],
      ["resources", "(RG212|Arehaza)"],
      ["origin", "(R430|Hakoven Islands)"],
    ],
  );
});

test("buildZonePaneFacts includes RGB and bite time from zone catalog", () => {
  const rows = buildZonePaneFacts(
    [
      {
        layerId: "zone_mask",
        rgbU32: 0x39e58d,
        rgb: [57, 229, 141],
        detailSections: [detailSectionFact("zone", "Zone", "Valencia Sea - Depth 5", "hover-zone")],
      },
    ],
    {
      zoneCatalog: [
        {
          zoneRgb: 0x39e58d,
          name: "Valencia Sea - Depth 5",
          biteTimeMin: 5,
          biteTimeMax: 7,
        },
      ],
    },
  );

  assert.deepEqual(
    rows.map((row) => [row.key, row.value, row.swatchRgb || ""]),
    [
      ["zone", "Valencia Sea - Depth 5", ""],
      ["rgb", "57,229,141", "57 229 141"],
      ["bite_time", "5-7 s", ""],
    ],
  );
});

test("buildTerritoryPaneFacts and buildTradePaneFacts normalize semantic labels", () => {
  const layerSamples = [
    {
      layerId: "region_groups",
      detailSections: [detailSectionFact("resource_region", "Region", "(RG212|Arehaza)", "hover-resources")],
    },
    {
      layerId: "regions",
      detailSections: [detailSectionFact("origin_region", "Region", "(R430|Hakoven Islands)", "trade-origin")],
    },
  ];

  assert.deepEqual(buildTerritoryPaneFacts(layerSamples), [
    {
      key: "resources",
      icon: "hover-resources",
      label: "Resources",
      value: "(RG212|Arehaza)",
    },
  ]);
  assert.deepEqual(buildTradePaneFacts(layerSamples), [
    {
      key: "origin",
      icon: "trade-origin",
      label: "Origin",
      value: "(R430|Hakoven Islands)",
    },
  ]);
});

test("buildTradePaneFacts enriches selected origins with trade managers sorted by distance", () => {
  const layerSamples = [
    {
      layerId: "regions",
      detailSections: [detailSectionFact("origin_region", "Origin", "Hakoven Islands (R430)", "trade-origin")],
      targets: [{ key: "origin_node", label: "Origin: Hakoven Islands (R430)", worldX: 10, worldZ: 20 }],
    },
  ];
  const tradeNpcMapCatalog = {
    features: [
      {
        properties: {
          id: "near",
          npcName: "Near Trader",
          sellOriginLabel: "Velia (R5)",
          sellDestinationTradeOrigin: { region_id: 5, world_x: 1_000, world_z: 20 },
        },
        geometry: { coordinates: [1_000, 20] },
      },
      {
        properties: {
          id: "far",
          npcName: "Far Trader",
          sellOriginLabel: "Valencia City (R42)",
          sellDestinationTradeOrigin: { region_id: 42, world_x: 20_000, world_z: 20 },
        },
        geometry: { coordinates: [20_000, 20] },
      },
    ],
  };

  assert.deepEqual(
    buildTradePaneFacts(layerSamples, { tradeNpcMapCatalog, tradeNpcMapStatus: "loaded" }).map(
      (row) => [row.key, row.label, row.value],
    ),
    [
      ["origin", "Origin", "Hakoven Islands (R430)"],
      ["trade_manager_count", "Trade Managers", "2 destination traders"],
      ["trade_manager:far", "Far Trader", "1.4% · Valencia City (R42)"],
      ["trade_manager:near", "Near Trader", "0.1% · Velia (R5)"],
    ],
  );
});

test("preferredOverviewRow prefers zone over territory and trade facts", () => {
  const preferred = preferredOverviewRow([
    {
      layerId: "region_groups",
      detailSections: [detailSectionFact("resource_group", "Resources", "(RG212|Arehaza)", "hover-resources")],
    },
    {
      layerId: "zone_mask",
      detailSections: [detailSectionFact("zone", "Zone", "Valencia Sea - Depth 5", "hover-zone")],
    },
  ]);

  assert.equal(preferred?.key, "zone");
  assert.equal(preferred?.value, "Valencia Sea - Depth 5");
});

test("preferredPointLabelForLayerSamples follows layer order and zone fallback names", () => {
  const label = preferredPointLabelForLayerSamples(
    [
      {
        layerId: "region_groups",
        detailSections: [detailSectionFact("resource_group", "Resources", "(RG218|Margoria)", "hover-resources")],
      },
      {
        layerId: "zone_mask",
        rgbU32: 0x3c963c,
        rgb: [60, 150, 60],
        detailSections: [],
      },
    ],
    {
      zoneCatalog: [{ zoneRgb: 0x3c963c, name: "Margoria South" }],
      runtimeLayers: [
        { layerId: "zone_mask", displayOrder: 10 },
        { layerId: "region_groups", displayOrder: 20 },
      ],
    },
  );

  assert.equal(label, "Margoria South");
});
installMapTestI18n();
