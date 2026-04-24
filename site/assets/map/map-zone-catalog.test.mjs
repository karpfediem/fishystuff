import { test } from "bun:test";
import assert from "node:assert/strict";

import {
  findZoneMatches,
  loadZoneCatalog,
  normalizeZoneCatalog,
  zoneDisplayNameFromCatalog,
} from "./map-zone-catalog.js";

test("normalizeZoneCatalog accepts common zone payload shapes", () => {
  const catalog = normalizeZoneCatalog({
    zones: [
      {
        rgb: { r: 112, g: 167, b: 193 },
        name: "Zenato Sea - Depth 4",
        confirmed: 1,
        order: 4,
      },
    ],
  });

  assert.deepEqual(catalog, [
    {
      kind: "zone",
      zoneRgb: 7382977,
      r: 112,
      g: 167,
      b: 193,
      name: "Zenato Sea - Depth 4",
      confirmed: true,
      active: null,
      order: 4,
      biteTimeMin: null,
      biteTimeMax: null,
      rgbKey: "112,167,193",
      rgbSpaced: "112 167 193",
      normalizedKey: "0.439216,0.654902,0.756863",
      normalizedSpaced: "0.439216 0.654902 0.756863",
      hexKey: "0x70a7c1",
      hashHexKey: "#70a7c1",
      bareHexKey: "70a7c1",
      _nameSearch: "zenato sea - depth 4",
      _nameSearchCompact: "zenatoseadepth4",
    },
  ]);
});

test("findZoneMatches matches by name and rgb formats", () => {
  const catalog = normalizeZoneCatalog([
    {
      r: 112,
      g: 167,
      b: 193,
      name: "Zenato Sea - Depth 4",
      confirmed: true,
      order: 1,
    },
    {
      r: 193,
      g: 127,
      b: 127,
      name: "Cron Islands - Depth 2",
      confirmed: true,
      order: 2,
    },
  ]);

  assert.equal(findZoneMatches(catalog, "Depth 4")[0]?.name, "Zenato Sea - Depth 4");
  assert.equal(findZoneMatches(catalog, "#70a7c1")[0]?.name, "Zenato Sea - Depth 4");
  assert.equal(zoneDisplayNameFromCatalog(catalog, 7382977), "Zenato Sea - Depth 4");
  assert.deepEqual(
    findZoneMatches(catalog, "Depth 4").map((zone) => zone.name),
    ["Zenato Sea - Depth 4"],
  );
});

test("findZoneMatches supports fuzzy zone-name matching", () => {
  const catalog = normalizeZoneCatalog([
    {
      r: 0,
      g: 0,
      b: 1,
      name: "Valencia Sea - Depth 5",
      confirmed: true,
      order: 1,
    },
    {
      r: 0,
      g: 0,
      b: 2,
      name: "O'draxxia (Leaf Spot)",
      confirmed: true,
      order: 2,
    },
    {
      r: 0,
      g: 0,
      b: 3,
      name: "O'dyllita Waters",
      confirmed: true,
      order: 3,
    },
  ]);

  const byCompactAlias = findZoneMatches(catalog, "Val D5");
  assert.equal(byCompactAlias[0]?.name, "Valencia Sea - Depth 5");

  assert.equal(findZoneMatches(catalog, "odraxia")[0]?.name, "O'draxxia (Leaf Spot)");
  assert.equal(findZoneMatches(catalog, "ody")[0]?.name, "O'dyllita Waters");
});

test("loadZoneCatalog normalizes the fetched payload", async () => {
  const catalog = await loadZoneCatalog(
    async () => ({
      ok: true,
      async json() {
        return [
          {
            r: 112,
            g: 167,
            b: 193,
            name: "Zenato Sea - Depth 4",
            confirmed: true,
          },
        ];
      },
    }),
    { protocol: "http:", hostname: "127.0.0.1" },
  );

  assert.equal(catalog[0]?.zoneRgb, 7382977);
  assert.equal(catalog[0]?.name, "Zenato Sea - Depth 4");
});
