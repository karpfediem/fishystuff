import { test } from "bun:test";
import assert from "node:assert/strict";
import { installMapTestI18n } from "./test-i18n.js";

import {
  buildFocusBookmarkPatch,
  buildBookmarkPlacementSelectionResult,
  readMapBookmarkPanelShellSignals,
  registerFishyMapBookmarkPanelElement,
} from "./map-bookmark-panel-element.js";

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

test("readMapBookmarkPanelShellSignals prefers live shell signals over initial signals", () => {
  const initialSignals = { _map_bookmarks: { entries: [{ id: "a" }] } };
  const liveSignals = { _map_bookmarks: { entries: [{ id: "b" }] } };
  const shell = {
    __fishymapInitialSignals: initialSignals,
    __fishymapLiveSignals: liveSignals,
  };

  assert.equal(readMapBookmarkPanelShellSignals(shell), liveSignals);
});

test("registerFishyMapBookmarkPanelElement defines the custom element once", () => {
  const registry = {
    definitions: new Map(),
    get(name) {
      return this.definitions.get(name) || null;
    },
    define(name, constructor) {
      this.definitions.set(name, constructor);
    },
  };

  assert.equal(registerFishyMapBookmarkPanelElement(registry), true);
  assert.equal(registerFishyMapBookmarkPanelElement(registry), true);
  assert.equal(registry.definitions.size, 1);
  assert.ok(registry.get("fishymap-bookmark-panel"));
});

test("buildFocusBookmarkPatch selects bookmarks through the generic selected-element path", () => {
  assert.deepEqual(
    buildFocusBookmarkPatch(
      { id: "bookmark-a", label: "Saved Hotspot", worldX: "12.5", worldZ: "-7.25" },
      {
        _map_actions: { focusWorldPointToken: 4 },
        _map_session: { view: { viewMode: "2d", camera: { zoom: 768 } } },
      },
    ),
    {
      _map_actions: {
        focusWorldPointToken: 5,
        focusWorldPoint: {
          elementKind: "bookmark",
          worldX: 12.5,
          worldZ: -7.25,
          pointKind: "bookmark",
          pointLabel: "Saved Hotspot",
          historyBehavior: "append",
        },
      },
      _map_session: {
        view: {
          viewMode: "2d",
          camera: {
            zoom: 768,
            centerWorldX: 12.5,
            centerWorldZ: -7.25,
          },
        },
      },
    },
  );
});

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
installMapTestI18n();
