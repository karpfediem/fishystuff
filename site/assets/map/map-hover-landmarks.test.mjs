import { test } from "bun:test";
import assert from "node:assert/strict";
import { installMapTestI18n } from "./test-i18n.js";

import { buildLandmarkHoverRows } from "./map-hover-landmarks.js";

const stateBundle = {
  state: {
    catalog: {
      fish: [{ fishId: 10, itemId: 123, name: "Grunt", grade: "white" }],
    },
  },
};

test("buildLandmarkHoverRows builds explicit landmark rows for samples and targets", () => {
  const rows = buildLandmarkHoverRows({
    stateBundle,
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
      layerSamples: [
        {
          layerId: "zone_mask",
          rgb: [57, 229, 141],
          rgbU32: 0x39e58d,
          targets: [],
          detailSections: [
            {
              id: "zone",
              kind: "facts",
              title: "Zone",
              facts: [{ key: "zone", label: "Zone", value: "Velia Coast", icon: "hover-zone" }],
            },
          ],
        },
        {
          layerId: "trade_npcs",
          targets: [
            { key: "trade_npc", label: "Bahar", worldX: 10, worldZ: 20 },
            { key: "trade_npc", label: "Bahar", worldX: 10, worldZ: 20 },
          ],
          detailSections: [],
        },
        {
          layerId: "bookmarks",
          targets: [{ key: "bookmark", label: "Saved Hotspot", worldX: 30, worldZ: 40 }],
          detailSections: [],
        },
        {
          layerId: "fishing_hotspots",
          targets: [{ key: "fishing_hotspot", label: "Fishing Hotspot #2", worldX: 50, worldZ: 60 }],
          detailSections: [],
        },
      ],
    },
  });

  assert.deepEqual(
    rows.map((row) =>
      row.kind === "point-sample"
        ? [row.kind, row.fishId, row.fishName, row.sampleCount]
        : [row.kind, row.layerId, row.targetKey, row.label, row.value, row.icon],
    ),
    [
      ["point-sample", 10, "Grunt", 3],
      ["landmark-hover", "trade_npcs", "trade_npc", "NPC", "Bahar", "trade-origin"],
      ["landmark-hover", "bookmarks", "bookmark", "Bookmark", "Saved Hotspot", "bookmark"],
      ["landmark-hover", "fishing_hotspots", "fishing_hotspot", "Hotspot", "Fishing Hotspot #2", "map-pin"],
    ],
  );
});

test("buildLandmarkHoverRows returns no rows for ordinary layer fact samples", () => {
  const rows = buildLandmarkHoverRows({
    hover: {
      layerSamples: [
        {
          layerId: "zone_mask",
          rgb: [57, 229, 141],
          rgbU32: 0x39e58d,
          targets: [],
          detailSections: [
            {
              id: "zone",
              kind: "facts",
              title: "Zone",
              facts: [{ key: "zone", label: "Zone", value: "Velia Coast", icon: "hover-zone" }],
            },
          ],
        },
        {
          layerId: "regions",
          targets: [{ key: "origin_node", label: "Origin: Margoria (R829)", worldX: 1, worldZ: 2 }],
          detailSections: [],
        },
        {
          layerId: "region_groups",
          targets: [{ key: "resource_node", label: "Resources: Margoria (RG218)", worldX: 1, worldZ: 2 }],
          detailSections: [],
        },
      ],
    },
  });

  assert.deepEqual(rows, []);
});

test("buildLandmarkHoverRows can hide fish sample landmarks through the sample hover setting", () => {
  const rows = buildLandmarkHoverRows({
    stateBundle,
    pointSamplesEnabled: false,
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
    },
  });

  assert.deepEqual(rows, []);
});

installMapTestI18n();
