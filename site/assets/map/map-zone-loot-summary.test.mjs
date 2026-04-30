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
        rawDropRateText: "120%",
        rawDropRateTooltip: "Raw General group share",
        normalizedDropRateText: "80%",
        normalizedDropRateTooltip: "Normalized General group share",
        conditionText: "Zone base rate 80%",
        conditionTooltip: "Zone base rate: 80%",
        catchMethods: ["rod"],
        conditionOptions: [
          {
            conditionText: "Default",
            dropRateText: "80%",
            dropRateSourceKind: "database",
            dropRateTooltip: "Default General source",
            presenceText: "Community confirmed×1 · General subgroup",
            presenceSourceKind: "community",
            presenceTooltip: "Community confirmed×1 · General subgroup 11054",
            rawDropRateText: "120%",
            rawDropRateTooltip: "Default raw General source",
            normalizedDropRateText: "80%",
            normalizedDropRateTooltip: "Default normalized General source",
            active: true,
            speciesRows: [
              {
                slotIdx: 4,
                groupLabel: "General",
                label: "Sea Eel",
                dropRateText: "80%",
                rawDropRateText: "120%",
                rawDropRateTooltip: "Raw Sea Eel source",
                normalizedDropRateText: "80%",
                normalizedDropRateTooltip: "Normalized Sea Eel source",
                catchMethods: ["rod"],
              },
            ],
          },
          {
            conditionText: "Fishing Level Guru 1+",
            active: false,
            speciesRows: [
              {
                slotIdx: 4,
                groupLabel: "General",
                label: "Mystical Fish",
                dropRateText: "0.005%",
                catchMethods: ["rod"],
              },
            ],
          },
        ],
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
  assert.equal(summary.groups[0].rawDropRateText, "120%");
  assert.equal(summary.groups[0].normalizedDropRateTooltip, "Normalized General group share");
  assert.equal(summary.groups[0].conditionText, "Zone base rate 80%");
  assert.deepEqual(summary.groups[0].catchMethods, ["rod"]);
  assert.equal(summary.groups[0].conditionOptions.length, 2);
  assert.equal(summary.groups[0].conditionOptions[0].conditionText, "Default");
  assert.equal(summary.groups[0].conditionOptions[0].dropRateText, "80%");
  assert.equal(summary.groups[0].conditionOptions[0].dropRateSourceKind, "database");
  assert.equal(summary.groups[0].conditionOptions[0].dropRateTooltip, "Default General source");
  assert.equal(summary.groups[0].conditionOptions[0].presenceSourceKind, "community");
  assert.match(summary.groups[0].conditionOptions[0].presenceTooltip, /General subgroup 11054/);
  assert.equal(summary.groups[0].conditionOptions[0].rawDropRateTooltip, "Default raw General source");
  assert.equal(summary.groups[0].conditionOptions[0].normalizedDropRateText, "80%");
  assert.equal(summary.groups[0].conditionOptions[0].active, true);
  assert.equal(summary.groups[0].conditionOptions[1].speciesRows[0].label, "Mystical Fish");
  assert.equal(summary.speciesRows[0].groupLabel, "General");
  assert.equal(summary.speciesRows[0].dropRateText, "80%");
  assert.equal(summary.groups[0].conditionOptions[0].speciesRows[0].rawDropRateText, "120%");
  assert.equal(
    summary.groups[0].conditionOptions[0].speciesRows[0].normalizedDropRateTooltip,
    "Normalized Sea Eel source",
  );
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
    normalizeRates: false,
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

  assert.equal(request.url, "http://127.0.0.1:8080/api/v1/zone_loot_summary?lang=en&locale=en-US");
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
    showNormalizedSelectRates: false,
  });
  assert.equal(result.zoneName, "Valencia Sea - Depth 5");
});

test("loadZoneLootSummary posts the normalize rate preference when provided", async () => {
  let body = null;
  await loadZoneLootSummary(0x39e58d, {
    normalizeRates: false,
    locationLike: {
      origin: "http://127.0.0.1:1990",
      protocol: "http:",
      hostname: "127.0.0.1",
    },
    fetchImpl: async (_url, init) => {
      body = JSON.parse(init.body);
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

  assert.equal(body.showNormalizedSelectRates, false);
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
