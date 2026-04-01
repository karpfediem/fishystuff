import test from "node:test";
import assert from "node:assert/strict";

import {
  buildLayerSearchEffects,
  layerSearchTermKindLabels,
} from "./map-layer-search-effects.js";

test("layerSearchTermKindLabels exposes human-readable layer support", () => {
  assert.deepEqual(layerSearchTermKindLabels("zone_mask"), ["Fish", "Fish filters", "Zones"]);
  assert.deepEqual(layerSearchTermKindLabels("regions"), ["Semantic fields"]);
});

test("buildLayerSearchEffects derives search-driven clipping from actual layer attachments", () => {
  assert.deepEqual(
    buildLayerSearchEffects({
      fishIds: [121],
      layerClipMasks: {
        fish_evidence: "zone_mask",
        regions: "zone_mask",
        minimap: "manual-mask",
        bookmarks: "bookmarks",
      },
    }),
    {
      activeZoneSearch: true,
      effectiveLayerClipMasks: {
        fish_evidence: "zone_mask",
        regions: "zone_mask",
        minimap: "manual-mask",
      },
      zoneMembershipLayerIds: ["fish_evidence"],
    },
  );
});

test("buildLayerSearchEffects leaves attachment-driven clipping idle without active zone search", () => {
  assert.deepEqual(
    buildLayerSearchEffects({
      fishIds: [],
      zoneRgbs: [],
      semanticFieldIdsByLayer: {},
      fishFilterTerms: [],
      layerClipMasks: {
        fish_evidence: "zone_mask",
        regions: "zone_mask",
      },
    }),
    {
      activeZoneSearch: false,
      effectiveLayerClipMasks: {
        fish_evidence: "zone_mask",
        regions: "zone_mask",
      },
      zoneMembershipLayerIds: [],
    },
  );
});
