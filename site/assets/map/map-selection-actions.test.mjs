import { test } from "bun:test";
import assert from "node:assert/strict";

import { buildFocusWorldPointSignalPatch } from "./map-selection-actions.js";

test("buildFocusWorldPointSignalPatch selects and centers the current map view by default", () => {
  assert.deepEqual(
    buildFocusWorldPointSignalPatch(
      { worldX: 1_100, worldZ: 120, pointKind: "waypoint", pointLabel: "Chunsu" },
      {
        _map_actions: { focusWorldPointToken: 2 },
        _map_session: { view: { viewMode: "2d", camera: { zoom: 512 } } },
      },
    ),
    {
      _map_actions: {
        focusWorldPointToken: 3,
        focusWorldPoint: {
          elementKind: "",
          worldX: 1_100,
          worldZ: 120,
          pointKind: "waypoint",
          pointLabel: "Chunsu",
          historyBehavior: "append",
        },
      },
      _map_session: {
        view: {
          viewMode: "2d",
          camera: {
            zoom: 512,
            centerWorldX: 1_100,
            centerWorldZ: 120,
          },
        },
      },
    },
  );
});

test("buildFocusWorldPointSignalPatch respects disabled auto-adjust view", () => {
  assert.deepEqual(
    buildFocusWorldPointSignalPatch(
      { worldX: 1_100, worldZ: 120, pointKind: "waypoint", pointLabel: "Chunsu" },
      {
        _map_actions: { focusWorldPointToken: 2 },
        _map_ui: { windowUi: { settings: { autoAdjustView: false } } },
        _map_session: { view: { viewMode: "2d", camera: { zoom: 512 } } },
      },
    ),
    {
      _map_actions: {
        focusWorldPointToken: 3,
        focusWorldPoint: {
          elementKind: "",
          worldX: 1_100,
          worldZ: 120,
          pointKind: "waypoint",
          pointLabel: "Chunsu",
          historyBehavior: "append",
        },
      },
    },
  );
});
