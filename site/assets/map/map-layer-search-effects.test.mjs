import test from "node:test";
import assert from "node:assert/strict";

import {
  buildLayerSearchClipsPatch,
  buildLayerSearchEffects,
  layerSearchClipRows,
  layerSearchTermKindLabels,
  normalizeLayerSearchClips,
} from "./map-layer-search-effects.js";

test("normalizeLayerSearchClips keeps only supported layer clip modes", () => {
  assert.deepEqual(
    normalizeLayerSearchClips({
      fish_evidence: "zone-membership",
      minimap: "mask-sample",
      bookmarks: "mask-sample",
      bad: "zone-membership",
    }),
    {
      fish_evidence: "zone-membership",
      minimap: "mask-sample",
    },
  );
});

test("layerSearchTermKindLabels exposes human-readable layer support", () => {
  assert.deepEqual(layerSearchTermKindLabels("zone_mask"), ["Fish", "Fish filters", "Zones"]);
  assert.deepEqual(layerSearchTermKindLabels("regions"), ["Semantic fields"]);
});

test("layerSearchClipRows reflects enabled rows for a layer", () => {
  assert.deepEqual(layerSearchClipRows("fish_evidence", { fish_evidence: "zone-membership" }), [
    {
      layerId: "fish_evidence",
      clipMode: "zone-membership",
      label: "Clip to visible Zone Mask",
      enabled: true,
    },
  ]);
});

test("buildLayerSearchClipsPatch toggles page-owned layer clip preferences", () => {
  assert.deepEqual(
    buildLayerSearchClipsPatch({}, "fish_evidence", "zone-membership", true),
    { fish_evidence: "zone-membership" },
  );
  assert.deepEqual(
    buildLayerSearchClipsPatch(
      { fish_evidence: "zone-membership", minimap: "mask-sample" },
      "fish_evidence",
      "zone-membership",
      false,
    ),
    { minimap: "mask-sample" },
  );
});

test("buildLayerSearchEffects derives low-level bridge clip masks from page-owned prefs", () => {
  assert.deepEqual(
    buildLayerSearchEffects({
      fishIds: [121],
      layerClipMasks: { minimap: "manual-mask" },
      layerSearchClips: {
        fish_evidence: "zone-membership",
        regions: "mask-sample",
      },
    }),
    {
      activeZoneSearch: true,
      layerSearchClips: {
        fish_evidence: "zone-membership",
        regions: "mask-sample",
      },
      effectiveLayerClipMasks: {
        minimap: "manual-mask",
        regions: "zone_mask",
      },
      zoneMembershipLayerIds: ["fish_evidence"],
    },
  );
});
