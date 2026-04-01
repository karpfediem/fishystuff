import test from "node:test";
import assert from "node:assert/strict";

import { createMapApp, mergeBridgePatches } from "./map-app.js";
import { createEmptySnapshot } from "./map-host.js";

test("mergeBridgePatches merges input and commands without losing nested fields", () => {
  assert.deepEqual(
    mergeBridgePatches(
      {
        version: 1,
        filters: { fishIds: [77] },
        ui: { viewMode: "2d" },
      },
      {
        version: 1,
        commands: { resetView: true },
      },
    ),
    {
      version: 1,
      filters: { fishIds: [77] },
      ui: { viewMode: "2d" },
      commands: { resetView: true },
    },
  );
});

test("createMapApp emits bridge commands once per consumed action token state", () => {
  const app = createMapApp();
  const signals = {
    _map_bridged: {
      filters: {
        fishIds: [77],
      },
      ui: {
        viewMode: "2d",
      },
    },
    _map_actions: {
      resetViewToken: 1,
    },
  };

  const firstPatch = app.nextBridgePatch(signals, {
    currentState: createEmptySnapshot(),
  });
  assert.deepEqual(firstPatch.commands, { resetView: true });

  app.consumeSignals(signals);

  const secondPatch = app.nextBridgePatch(signals, {
    currentState: createEmptySnapshot(),
  });
  assert.equal("commands" in secondPatch, false);
  assert.deepEqual(secondPatch.filters.fishIds, [77]);
});

test("createMapApp exposes coarse runtime and session projections", () => {
  const app = createMapApp();
  assert.deepEqual(
    app.projectRuntimeSnapshot({
      ready: true,
      view: { viewMode: "3d" },
      selection: { pointKind: "clicked" },
      catalog: { layers: [] },
      statuses: {},
      lastDiagnostic: null,
      hover: { ignored: true },
    }),
    {
      _map_runtime: {
        ready: true,
        theme: {},
        view: { viewMode: "3d" },
        selection: { pointKind: "clicked" },
        catalog: { layers: [] },
        statuses: {},
        lastDiagnostic: null,
      },
    },
  );

  assert.deepEqual(
    app.projectSessionSnapshot({
      view: { viewMode: "2d" },
      selection: { pointKind: "bookmark" },
      hover: { ignored: true },
    }),
    {
      _map_session: {
        view: { viewMode: "2d" },
        selection: { pointKind: "bookmark" },
      },
    },
  );
});

test("createMapApp projects runtime bookmark enrichment without leaking full ui state", () => {
  const app = createMapApp();

  assert.deepEqual(
    app.projectRuntimeBookmarkDetails(
      {
        ui: {
          bookmarks: [
            {
              id: "bookmark-a",
              label: "Imported",
              worldX: 12,
              worldZ: 34,
              zoneRgb: 0x39e58d,
              layerSamples: [{ layerId: "zone_mask" }],
            },
          ],
        },
      },
      {
        _map_bookmarks: {
          entries: [{ id: "bookmark-a", label: "Imported", worldX: 12, worldZ: 34 }],
        },
      },
    ),
    {
      _map_bookmarks: {
        entries: [
          {
            id: "bookmark-a",
            label: "Imported",
            worldX: 12,
            worldZ: 34,
            zoneRgb: 0x39e58d,
            layerSamples: [{ layerId: "zone_mask" }],
          },
        ],
      },
    },
  );
});
