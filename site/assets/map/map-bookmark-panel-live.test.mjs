import test from "node:test";
import assert from "node:assert/strict";

import { buildBookmarkPlacementSelectionResult } from "./map-bookmark-panel-live.js";

function clickedSelection(overrides = {}) {
  return {
    worldX: 123,
    worldZ: 456,
    pointKind: "clicked",
    pointLabel: "Zenato Sea - Depth 4",
    layerSamples: [
      {
        layerId: "zone_mask",
        rgbU32: 7382977,
      },
    ],
    ...overrides,
  };
}

test("buildBookmarkPlacementSelectionResult allows explicit clicked-point placement even for the current selection key", () => {
  const selection = clickedSelection();
  const result = buildBookmarkPlacementSelectionResult({
    selection,
    bookmarks: [],
    placing: true,
    lastPlacementKey: JSON.stringify({
      worldX: 123,
      worldZ: 456,
      pointKind: "clicked",
      pointLabel: "Zenato Sea - Depth 4",
    }),
    allowSameSelection: true,
    requireClickedPoint: true,
  });

  assert.ok(result);
  assert.equal(
    result.placementKey,
    JSON.stringify({
      worldX: 123,
      worldZ: 456,
      pointKind: "clicked",
      pointLabel: "Zenato Sea - Depth 4",
    }),
  );
  assert.equal(result.bookmark.worldX, 123);
  assert.equal(result.bookmark.worldZ, 456);
  assert.equal(result.bookmark.label, "Zenato Sea - Depth 4");
});

test("buildBookmarkPlacementSelectionResult ignores unchanged selection keys on passive signal patches", () => {
  const result = buildBookmarkPlacementSelectionResult({
    selection: clickedSelection(),
    bookmarks: [],
    placing: true,
    lastPlacementKey: JSON.stringify({
      worldX: 123,
      worldZ: 456,
      pointKind: "clicked",
      pointLabel: "Zenato Sea - Depth 4",
    }),
  });

  assert.equal(result, null);
});

test("buildBookmarkPlacementSelectionResult ignores non-clicked focus selections for placement mode", () => {
  const result = buildBookmarkPlacementSelectionResult({
    selection: clickedSelection({ pointKind: "bookmark" }),
    bookmarks: [],
    placing: true,
    allowSameSelection: true,
    requireClickedPoint: true,
  });

  assert.equal(result, null);
});

test("buildBookmarkPlacementSelectionResult keeps the runtime point label for bookmark titles", () => {
  const result = buildBookmarkPlacementSelectionResult({
    selection: clickedSelection({
      pointLabel: "Margoria (RG218)",
      layerSamples: [
        {
          layerId: "zone_mask",
          rgbU32: 0x3c963c,
          rgb: [60, 150, 60],
        },
      ],
    }),
    bookmarks: [],
    placing: true,
    allowSameSelection: true,
    requireClickedPoint: true,
  });

  assert.ok(result);
  assert.equal(result.bookmark.label, "Margoria (RG218)");
});
