import test from "node:test";
import assert from "node:assert/strict";

import {
  buildLayerClipMaskPatch,
  flattenLayerClipMasks,
  resolveLayerEntries,
} from "./map-layer-state.js";

function layer(layerId, displayOrder) {
  return {
    layerId,
    name: layerId,
    visible: true,
    opacity: 1,
    opacityDefault: 1,
    displayOrder,
  };
}

function stateBundle(filters = {}) {
  return {
    state: {
      catalog: {
        layers: [
          layer("bookmarks", 60),
          layer("fish_evidence", 50),
          layer("regions", 40),
          layer("region_groups", 30),
          layer("zone_mask", 20),
          layer("minimap", 10),
        ],
      },
      filters: {
        layerIdsOrdered: [
          "bookmarks",
          "fish_evidence",
          "regions",
          "region_groups",
          "zone_mask",
          "minimap",
        ],
      },
    },
    inputState: {
      filters,
    },
  };
}

test("resolveLayerEntries prefers live bridged clip-mask overrides", () => {
  const entries = resolveLayerEntries(
    stateBundle({
      layerClipMasks: {
        fish_evidence: "zone_mask",
        region_groups: "zone_mask",
      },
    }),
  );

  assert.deepEqual(
    entries
      .filter((entry) => entry.clipMaskLayerId)
      .map((entry) => [entry.layerId, entry.clipMaskLayerId]),
    [
      ["fish_evidence", "zone_mask"],
      ["region_groups", "zone_mask"],
    ],
  );
});

test("buildLayerClipMaskPatch detaches a layer without disturbing siblings", () => {
  const next = buildLayerClipMaskPatch(
    stateBundle({
      layerClipMasks: {
        fish_evidence: "zone_mask",
        region_groups: "zone_mask",
      },
    }),
    "region_groups",
    "",
  );

  assert.deepEqual(next, {
    fish_evidence: "zone_mask",
  });
});

test("buildLayerClipMaskPatch reattaches the dragged subtree to the new mask root", () => {
  const next = buildLayerClipMaskPatch(
    stateBundle({
      layerClipMasks: {
        regions: "region_groups",
      },
    }),
    "region_groups",
    "zone_mask",
  );

  assert.deepEqual(next, {
    region_groups: "zone_mask",
    regions: "zone_mask",
  });
});

test("flattenLayerClipMasks collapses nested chains to the top-most non-ground mask", () => {
  assert.deepEqual(
    flattenLayerClipMasks({
      regions: "region_groups",
      region_groups: "zone_mask",
      fish_evidence: "zone_mask",
    }),
    {
      fish_evidence: "zone_mask",
      region_groups: "zone_mask",
      regions: "zone_mask",
    },
  );
});
