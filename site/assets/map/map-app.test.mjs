import test from "node:test";
import assert from "node:assert/strict";

import { createMapApp, mergeBridgePatches } from "./map-app.js";

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

  const firstPatch = app.nextBridgePatch(signals);
  assert.deepEqual(firstPatch.commands, { resetView: true });

  app.consumeSignals(signals);

  const secondPatch = app.nextBridgePatch(signals);
  assert.equal("commands" in secondPatch, false);
  assert.deepEqual(secondPatch.filters.fishIds, []);
  assert.equal(secondPatch.filters.searchExpression.type, "group");
  assert.equal(secondPatch.filters.searchExpression.children.length, 0);
});

test("createMapApp exposes coarse runtime and session projections", () => {
  const app = createMapApp();
  assert.deepEqual(
    app.projectRuntimeSnapshot({
      ready: true,
      effectiveFilters: {
        searchExpression: { type: "group", operator: "or", children: [] },
        sharedFishState: { caughtIds: [77], favouriteIds: [912] },
        zoneMembershipByLayer: { fish_evidence: { active: true, zoneRgbs: [0x39e58d], revision: 4 } },
        semanticFieldFiltersByLayer: {},
      },
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
        effectiveFilters: {
          searchExpression: { type: "group", operator: "or", children: [] },
          sharedFishState: { caughtIds: [77], favouriteIds: [912] },
          zoneMembershipByLayer: {
            fish_evidence: { active: true, zoneRgbs: [0x39e58d], revision: 4 },
          },
          semanticFieldFiltersByLayer: {},
        },
        ui: { bookmarks: [] },
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
