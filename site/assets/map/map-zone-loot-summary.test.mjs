import { test } from "bun:test";
import assert from "node:assert/strict";
import { installMapTestI18n } from "./test-i18n.js";

import {
  loadZoneLootSummary,
  normalizeZoneLootSummary,
  zoneRgbFromSelection,
} from "./map-zone-loot-summary.js";

test("zoneRgbFromSelection prefers zone stats rgb and falls back to zone mask samples", () => {
  assert.equal(zoneRgbFromSelection({ zoneStats: { zoneRgb: 0x39e58d } }), 0x39e58d);
  assert.equal(
    zoneRgbFromSelection({
      layerSamples: [{ layerId: "zone_mask", rgbU32: 0x39e58d }],
    }),
    0x39e58d,
  );
  assert.equal(zoneRgbFromSelection({ layerSamples: [] }), null);
});

test("normalizeZoneLootSummary keeps grouped species rows intact", () => {
  const summary = normalizeZoneLootSummary({
    available: true,
    zoneName: "Valencia Sea - Depth 5",
    dataQualityNote:
      "Expected loot uses average session casts, the current Fish multiplier, normalized group shares, and actual source-backed item prices.",
    note: "Zone loot uses calculator defaults.",
    profileLabel: "Calculator defaults",
    groups: [
      {
        slotIdx: 4,
        label: "General",
        dropRateText: "80%",
        dropRateSourceKind: "database",
        dropRateTooltip: "Source-backed General group share",
        conditionText: "Zone base rate 80%",
        conditionTooltip: "Zone base rate: 80%",
        catchMethods: ["rod"],
      },
    ],
    speciesRows: [
      {
        slotIdx: 4,
        groupLabel: "General",
        label: "Sea Eel",
        dropRateText: "80%",
        presenceText: "Community confirmed×2 · General subgroup",
        presenceTooltip:
          "Community confirmed×2 · General subgroup 11054 · source community_presence_sheet",
        catchMethods: ["rod"],
      },
    ],
  });

  assert.equal(summary.available, true);
  assert.match(summary.dataQualityNote, /Expected loot uses average session casts/);
  assert.equal(summary.groups[0].slotIdx, 4);
  assert.equal(summary.groups[0].dropRateText, "80%");
  assert.equal(summary.groups[0].dropRateSourceKind, "database");
  assert.equal(summary.groups[0].conditionText, "Zone base rate 80%");
  assert.deepEqual(summary.groups[0].catchMethods, ["rod"]);
  assert.equal(summary.speciesRows[0].groupLabel, "General");
  assert.equal(summary.speciesRows[0].dropRateText, "80%");
  assert.equal(
    summary.speciesRows[0].presenceText,
    "Community confirmed×2 · General subgroup",
  );
  assert.deepEqual(summary.speciesRows[0].catchMethods, ["rod"]);
  assert.match(summary.speciesRows[0].presenceTooltip, /community_presence_sheet/);
});

test("loadZoneLootSummary posts rgb triplets to the zone loot summary endpoint", async () => {
  let request = null;
  const result = await loadZoneLootSummary(0x39e58d, {
    overlaySignals: {
      zones: {
        "57,229,141": {
          groups: {
            4: { rawRatePercent: 82 },
          },
          items: {},
        },
      },
    },
    locationLike: {
      origin: "http://127.0.0.1:1990",
      protocol: "http:",
      hostname: "127.0.0.1",
    },
    fetchImpl: async (url, init) => {
      request = { url, init };
      return {
        ok: true,
        async json() {
          return {
            available: true,
            zoneName: "Valencia Sea - Depth 5",
            groups: [],
            speciesRows: [],
          };
        },
      };
    },
  });

  assert.equal(request.url, "http://127.0.0.1:8080/api/v1/zone_loot_summary");
  assert.deepEqual(JSON.parse(request.init.body), {
    rgb: "57,229,141",
    overlay: {
      zones: {
        "57,229,141": {
          groups: {
            4: { rawRatePercent: 82 },
          },
          items: {},
        },
      },
    },
  });
  assert.equal(result.zoneName, "Valencia Sea - Depth 5");
});

test("loadZoneLootSummary reports a clear message when the API build lacks the endpoint", async () => {
  await assert.rejects(
    () =>
      loadZoneLootSummary(0x39e58d, {
        locationLike: {
          origin: "http://127.0.0.1:1990",
          protocol: "http:",
          hostname: "127.0.0.1",
        },
        fetchImpl: async () => ({
          ok: false,
          status: 404,
        }),
      }),
    /Zone loot summary endpoint is unavailable on the current API build\./,
  );
});
installMapTestI18n();
