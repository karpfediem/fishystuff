import { test } from "bun:test";
import assert from "node:assert/strict";
import { installMapTestI18n } from "./test-i18n.js";

import {
  formatTradeDistanceBonus,
  loadTradeNpcMapCatalog,
  selectedTradeOriginFromLayerSamples,
  tradeManagerFactsForOrigin,
  tradeManagerRowsForOrigin,
} from "./map-trade-summary.js";

function originLayerSample() {
  return {
    layerId: "regions",
    fieldId: 430,
    detailSections: [
      {
        id: "trade-origin",
        kind: "facts",
        title: "Trade Origin",
        facts: [
          {
            key: "origin_region",
            label: "Origin",
            value: "Hakoven Islands (R430)",
            icon: "trade-origin",
          },
        ],
      },
    ],
    targets: [
      {
        key: "origin_node",
        label: "Origin: Hakoven Islands (R430)",
        worldX: 10,
        worldZ: 20,
      },
    ],
  };
}

function tradeNpcCatalog() {
  return {
    type: "FeatureCollection",
    metadata: { layerId: "trade_npcs" },
    features: [
      {
        type: "Feature",
        properties: {
          id: "near",
          npcKey: 1,
          npcName: "Near Trader",
          sellOriginLabel: "Velia (R5)",
          sellDestinationTradeOrigin: {
            region_id: 5,
            region_name: "Velia",
            world_x: 1_000,
            world_z: 20,
          },
        },
        geometry: { type: "Point", coordinates: [1_000, 20] },
      },
      {
        type: "Feature",
        properties: {
          id: "far",
          npcKey: 2,
          npcName: "Far Trader",
          sellOriginLabel: "Valencia City (R42)",
          sellDestinationTradeOrigin: {
            region_id: 42,
            region_name: "Valencia City",
            world_x: 20_000,
            world_z: 20,
          },
        },
        geometry: { type: "Point", coordinates: [20_000, 20] },
      },
    ],
  };
}

test("selectedTradeOriginFromLayerSamples reads origin labels and hover target coordinates", () => {
  assert.deepEqual(selectedTradeOriginFromLayerSamples([originLayerSample()]), {
    regionId: 430,
    label: "Hakoven Islands (R430)",
    worldX: 10,
    worldZ: 20,
  });
});

test("tradeManagerRowsForOrigin sorts destination traders by highest distance first", () => {
  const rows = tradeManagerRowsForOrigin([originLayerSample()], tradeNpcCatalog());

  assert.equal(rows.length, 2);
  assert.equal(rows[0].npcName, "Far Trader");
  assert.equal(rows[1].npcName, "Near Trader");
  assert.equal(formatTradeDistanceBonus(rows[0].distanceBonus), "1.4%");
});

test("tradeManagerFactsForOrigin includes a manager count and sorted distance rows", () => {
  const facts = tradeManagerFactsForOrigin([originLayerSample()], tradeNpcCatalog(), {
    status: "loaded",
  });

  assert.deepEqual(
    facts.map((fact) => [fact.key, fact.label, fact.value]),
    [
      ["trade_manager_count", "Trade Managers", "2 destination traders"],
      ["trade_manager:far", "Far Trader", "1.4% · Valencia City (R42)"],
      ["trade_manager:near", "Near Trader", "0.1% · Velia (R5)"],
    ],
  );
});

test("loadTradeNpcMapCatalog normalizes fetched trade NPC map features", async () => {
  const requests = [];
  const catalog = await loadTradeNpcMapCatalog({
    force: true,
    locationLike: { protocol: "http:", hostname: "127.0.0.1" },
    fetchImpl: async (url) => {
      requests.push(url);
      return {
        ok: true,
        async json() {
          return tradeNpcCatalog();
        },
      };
    },
  });

  assert.deepEqual(requests, ["http://127.0.0.1:8080/api/v1/trade_npcs/map"]);
  assert.equal(catalog.features[0].npcName, "Near Trader");
});

installMapTestI18n();
