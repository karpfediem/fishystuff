import test from "node:test";
import assert from "node:assert/strict";

import {
  buildLayerPanelStateBundle,
  patchTouchesLayerPanelSignals,
  toggleExpandedLayerIds,
} from "./map-layer-panel-state.js";

test("buildLayerPanelStateBundle keeps only runtime layer catalog and bridged filters", () => {
  assert.deepEqual(
    buildLayerPanelStateBundle({
      _map_runtime: {
        ready: true,
        catalog: {
          layers: [{ layerId: "fish_evidence", name: "Fish Evidence" }],
          patches: [{ patchId: "ignored" }],
        },
      },
      _map_bridged: {
        filters: {
          layerIdsVisible: ["bookmarks", "fish_evidence"],
          layerOpacities: { fish_evidence: 0.6 },
        },
      },
      _map_ui: {
        layers: {
          expandedLayerIds: ["fish_evidence"],
        },
      },
    }),
    {
      state: {
        ready: true,
        catalog: {
          layers: [{ layerId: "fish_evidence", name: "Fish Evidence" }],
        },
      },
      inputState: {
        filters: {
          layerIdsVisible: ["bookmarks", "fish_evidence"],
          layerOpacities: { fish_evidence: 0.6 },
        },
      },
    },
  );
});

test("toggleExpandedLayerIds toggles a layer id without duplicates", () => {
  assert.deepEqual(toggleExpandedLayerIds(["bookmarks"], "fish_evidence"), [
    "bookmarks",
    "fish_evidence",
  ]);
  assert.deepEqual(toggleExpandedLayerIds(["bookmarks", "fish_evidence"], "fish_evidence"), [
    "bookmarks",
  ]);
});

test("patchTouchesLayerPanelSignals only reacts to layer-relevant branches", () => {
  assert.equal(
    patchTouchesLayerPanelSignals({
      _map_runtime: {
        catalog: {
          layers: [],
        },
      },
    }),
    true,
  );
  assert.equal(
    patchTouchesLayerPanelSignals({
      _map_bridged: {
        filters: {
          layerIdsVisible: ["zone_mask"],
        },
      },
    }),
    true,
  );
  assert.equal(
    patchTouchesLayerPanelSignals({
      _map_ui: {
        layers: {
          expandedLayerIds: [],
        },
      },
    }),
    true,
  );
  assert.equal(
    patchTouchesLayerPanelSignals({
      _map_runtime: {
        selection: {
          pointKind: "clicked",
        },
      },
    }),
    true,
  );
  assert.equal(
    patchTouchesLayerPanelSignals({
      _map_ui: {
        windowUi: {
          layers: { open: false },
        },
      },
    }),
    false,
  );
});
