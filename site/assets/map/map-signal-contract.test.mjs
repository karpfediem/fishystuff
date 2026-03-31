import test from "node:test";
import assert from "node:assert/strict";

import {
  DEFAULT_ENABLED_LAYER_IDS,
  DEFAULT_MAP_BRIDGED_SIGNAL_STATE,
  normalizeMapBridgedSignalState,
  normalizeMapControlSignalState,
  normalizeMapUiSignalState,
  normalizeWindowUiState,
} from "./map-signal-contract.js";

test("normalizeWindowUiState applies defaults and normalizes coordinates", () => {
  const state = normalizeWindowUiState({
    search: { open: false, x: "42.8", y: "13.2" },
    settings: { autoAdjustView: false },
    zoneInfo: { tab: " fish " },
  });

  assert.equal(state.search.open, false);
  assert.equal(state.search.collapsed, false);
  assert.equal(state.search.x, 43);
  assert.equal(state.search.y, 13);
  assert.equal(state.settings.autoAdjustView, false);
  assert.equal(state.zoneInfo.tab, "fish");
  assert.equal(state.layers.open, true);
  assert.equal(state.bookmarks.open, false);
});

test("normalizeMapUiSignalState normalizes page-owned local UI state", () => {
  const state = normalizeMapUiSignalState({
    search: { open: true, query: " cron " },
    bookmarks: { placing: true, selectedIds: [" a ", "", "b", "a"] },
    layers: { expandedLayerIds: ["fish_evidence", "", "zone_mask", "fish_evidence"] },
  });

  assert.equal(state.search.open, true);
  assert.equal(state.search.query, " cron ");
  assert.equal(state.bookmarks.placing, true);
  assert.deepEqual(state.bookmarks.selectedIds, ["a", "b", "a"]);
  assert.deepEqual(state.layers.expandedLayerIds, ["fish_evidence", "zone_mask"]);
});

test("normalizeMapControlSignalState keeps only transitional page-owned fields", () => {
  const state = normalizeMapControlSignalState({
    filters: {
      patchId: " 123 ",
      fishIds: [77],
    },
    ui: {
      legendOpen: true,
    },
  });

  assert.equal(state.filters.patchId, "123");
  assert.deepEqual(state.filters.fishIds, [77]);
  assert.equal(state.ui.legendOpen, true);
  assert.equal(state.ui.leftPanelOpen, true);
  assert.equal("layerIdsVisible" in state.filters, false);
  assert.equal("viewMode" in state.ui, false);
});

test("normalizeMapBridgedSignalState keeps the bridge contract explicit and normalized", () => {
  const state = normalizeMapBridgedSignalState({
    filters: {
      layerIdsVisible: ["fish_evidence"],
    },
    ui: {
      viewMode: "3d",
      bookmarks: [{ id: "a", label: "A", worldX: 12.3, worldZ: 45.6 }],
      bookmarkSelectedIds: ["a"],
    },
  });

  assert.equal(state.version, DEFAULT_MAP_BRIDGED_SIGNAL_STATE.version ?? 1);
  assert.deepEqual(state.filters.layerIdsVisible, ["fish_evidence"]);
  assert.deepEqual(state.ui.bookmarkSelectedIds, ["a"]);
  assert.deepEqual(state.ui.bookmarks, [{ id: "a", label: "A", worldX: 12.3, worldZ: 45.6 }]);
  assert.equal(state.ui.viewMode, "3d");
  assert.equal(state.ui.showPointIcons, true);
});

test("normalizeMapBridgedSignalState falls back to default enabled layers", () => {
  const state = normalizeMapBridgedSignalState({});
  assert.deepEqual(state.filters.layerIdsVisible, DEFAULT_ENABLED_LAYER_IDS);
});
