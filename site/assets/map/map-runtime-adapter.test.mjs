import test from "node:test";
import assert from "node:assert/strict";

import {
  buildBridgeCommandPatchFromSignals,
  buildBridgeInputPatchFromSignals,
  normalizeMapActionState,
  projectRuntimeSnapshotToSignals,
  projectSessionSnapshotToSignals,
} from "./map-runtime-adapter.js";
import { createEmptySnapshot } from "./map-host.js";

test("buildBridgeInputPatchFromSignals projects only bridge-relevant state", () => {
  const patch = buildBridgeInputPatchFromSignals(
    {
      _map_ui: {
        windowUi: {
          search: { open: false },
        },
        bookmarks: {
          selectedIds: ["bookmark-a", "missing"],
        },
      },
      _map_controls: {
        filters: {
          searchText: "cron",
          fishFilterTerms: ["favourite"],
        },
        ui: {
          legendOpen: true,
          leftPanelOpen: false,
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [77],
          zoneRgbs: [123456],
          semanticFieldIdsByLayer: { regions: [11] },
          patchId: "p1",
          fromPatchId: "a",
          toPatchId: "b",
          layerIdsVisible: ["bookmarks", "fish_evidence"],
          layerIdsOrdered: ["fish_evidence", "bookmarks"],
          layerOpacities: { fish_evidence: 0.5 },
          layerClipMasks: { minimap: "manual-mask" },
          layerWaypointConnectionsVisible: { bookmarks: true },
          layerWaypointLabelsVisible: { bookmarks: false },
          layerPointIconsVisible: { fish_evidence: true },
          layerPointIconScales: { fish_evidence: 1.4 },
        },
        ui: {
          diagnosticsOpen: true,
          showPoints: false,
          showPointIcons: true,
          viewMode: "3d",
          pointIconScale: 1.8,
        },
      },
      _map_bookmarks: {
        entries: [
          {
            id: "bookmark-a",
            label: "Cron",
            pointLabel: "Cron Islands",
            worldX: 12.5,
            worldZ: 34.5,
            layerSamples: [{ nope: 1 }],
          },
          { id: "bookmark-b", worldX: 1, worldZ: 2 },
          { id: "", worldX: 9, worldZ: 9 },
        ],
      },
      _shared_fish: {
        caughtIds: [5],
        favouriteIds: [77],
      },
    },
    {
      currentState: {
        ...createEmptySnapshot(),
        catalog: {
          fish: [{ fishId: 912 }],
        },
      },
    },
  );

  assert.equal(patch.version, 1);
  assert.deepEqual(patch.filters.fishIds, [77]);
  assert.deepEqual(patch.filters.zoneRgbs, [123456]);
  assert.deepEqual(patch.filters.semanticFieldIdsByLayer, {
    regions: [11],
    zone_mask: [123456],
  });
  assert.deepEqual(patch.filters.fishFilterTerms, []);
  assert.equal(patch.filters.patchId, "p1");
  assert.equal(patch.filters.fromPatchId, "a");
  assert.equal(patch.filters.toPatchId, "b");
  assert.deepEqual(patch.filters.layerIdsVisible, ["bookmarks", "fish_evidence"]);
  assert.deepEqual(patch.filters.zoneMembershipLayerIds, []);
  assert.deepEqual(patch.filters.layerClipMasks, {
    minimap: "manual-mask",
  });
  assert.deepEqual(patch.ui.bookmarkSelectedIds, ["bookmark-a"]);
  assert.deepEqual(patch.ui.bookmarks, [
    { id: "bookmark-a", label: "Cron", worldX: 12.5, worldZ: 34.5 },
    { id: "bookmark-b", worldX: 1, worldZ: 2 },
  ]);
  assert.deepEqual(patch.ui.sharedFishState, {
    caughtIds: [5],
    favouriteIds: [77],
  });
  assert.equal("searchText" in patch.filters, false);
  assert.equal("legendOpen" in patch.ui, false);
  assert.equal("leftPanelOpen" in patch.ui, false);
  assert.equal("windowUi" in patch.ui, false);
});

test("buildBridgeInputPatchFromSignals derives search filters from selected terms", () => {
  const patch = buildBridgeInputPatchFromSignals(
    {
      _map_ui: {
        search: {
          selectedTerms: [
            { kind: "zone", zoneRgb: 123456 },
            { kind: "semantic", layerId: "regions", fieldId: 11 },
            { kind: "fish-filter", term: "favorite" },
          ],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [77],
          zoneRgbs: [],
          semanticFieldIdsByLayer: {},
          fishFilterTerms: [],
        },
      },
      _shared_fish: {
        caughtIds: [],
        favouriteIds: [912],
      },
    },
    {
      currentState: {
        ...createEmptySnapshot(),
        catalog: {
          fish: [{ fishId: 912 }],
        },
      },
    },
  );

  assert.deepEqual(patch.filters.zoneRgbs, [123456]);
  assert.deepEqual(patch.filters.semanticFieldIdsByLayer, {
    regions: [11],
    zone_mask: [123456],
  });
  assert.deepEqual(patch.filters.fishIds, [912]);
  assert.deepEqual(patch.filters.fishFilterTerms, ["favourite"]);
  assert.deepEqual(patch.filters.zoneMembershipLayerIds, []);
  assert.deepEqual(patch.ui.sharedFishState, {
    caughtIds: [],
    favouriteIds: [912],
  });
});

test("buildBridgeInputPatchFromSignals derives fish ids from the boolean search expression tree", () => {
  const patch = buildBridgeInputPatchFromSignals(
    {
      _map_ui: {
        search: {
          expression: {
            type: "group",
            operator: "or",
            children: [
              {
                type: "group",
                operator: "and",
                children: [
                  { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                  { type: "term", term: { kind: "fish-filter", term: "missing" } },
                ],
              },
              { type: "term", term: { kind: "fish-filter", term: "red" } },
            ],
          },
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [],
          zoneRgbs: [],
          semanticFieldIdsByLayer: {},
          fishFilterTerms: ["favourite", "missing", "red"],
        },
      },
      _shared_fish: {
        caughtIds: [912],
        favouriteIds: [77],
      },
    },
    {
      currentState: {
        ...createEmptySnapshot(),
        catalog: {
          fish: [
            { fishId: 61, itemId: 6100, name: "Ancient Relic Crystal Shard", grade: "Prize", isPrize: true },
            { fishId: 77, itemId: 77, name: "Serendia Carp", grade: "General", isPrize: false },
            { fishId: 912, itemId: 912, name: "Cron Dart", grade: "Rare", isPrize: false },
          ],
        },
      },
    },
  );

  assert.deepEqual(patch.filters.fishIds, [61, 77]);
  assert.deepEqual(patch.filters.fishFilterTerms, ["favourite", "missing", "red"]);
});

test("buildBridgeInputPatchFromSignals derives zone-membership clipping from attached layers", () => {
  const patch = buildBridgeInputPatchFromSignals(
    {
      _map_ui: {
        search: {
          selectedTerms: [{ kind: "zone", zoneRgb: 123456 }],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [],
          zoneRgbs: [],
          semanticFieldIdsByLayer: {},
          fishFilterTerms: [],
          layerClipMasks: {
            fish_evidence: "zone_mask",
            regions: "zone_mask",
          },
        },
      },
      _shared_fish: {
        caughtIds: [],
        favouriteIds: [],
      },
    },
    { currentState: createEmptySnapshot() },
  );

  assert.deepEqual(patch.filters.zoneMembershipLayerIds, ["fish_evidence"]);
  assert.deepEqual(patch.filters.layerClipMasks, {
    fish_evidence: "zone_mask",
    regions: "zone_mask",
  });
  assert.deepEqual(patch.ui.sharedFishState, {
    caughtIds: [],
    favouriteIds: [],
  });
});

test("buildBridgeInputPatchFromSignals ignores transitional control filters", () => {
  const patch = buildBridgeInputPatchFromSignals(
    {
      _map_controls: {
        filters: {
          fishIds: [912],
          zoneRgbs: [654321],
          semanticFieldIdsByLayer: { region_groups: [22] },
          fishFilterTerms: ["uncaught"],
          patchId: "legacy-patch",
        },
      },
    },
    { currentState: createEmptySnapshot() },
  );

  assert.deepEqual(patch.filters.fishIds, []);
  assert.deepEqual(patch.filters.zoneRgbs, []);
  assert.deepEqual(patch.filters.semanticFieldIdsByLayer, {});
  assert.deepEqual(patch.filters.fishFilterTerms, []);
  assert.equal(patch.filters.patchId, null);
});

test("buildBridgeCommandPatchFromSignals only emits resetView on token increase", () => {
  assert.equal(
    buildBridgeCommandPatchFromSignals(
      { _map_actions: { resetViewToken: 3, resetUiToken: 7, focusWorldPointToken: 0 } },
      { resetViewToken: 3, resetUiToken: 6, focusWorldPointToken: 0 },
    ),
    null,
  );

  assert.deepEqual(
    buildBridgeCommandPatchFromSignals(
      { _map_actions: { resetViewToken: 4, resetUiToken: 7, focusWorldPointToken: 0 } },
      { resetViewToken: 3, resetUiToken: 7, focusWorldPointToken: 0 },
    ),
    { version: 1, commands: { resetView: true } },
  );
});

test("normalizeMapActionState defaults missing tokens to zero", () => {
  assert.deepEqual(normalizeMapActionState(null), {
    resetViewToken: 0,
    resetUiToken: 0,
    focusWorldPointToken: 0,
    focusWorldPoint: null,
  });
});

test("buildBridgeCommandPatchFromSignals emits selectWorldPoint on focus token increase", () => {
  assert.deepEqual(
    buildBridgeCommandPatchFromSignals(
      {
        _map_actions: {
          resetViewToken: 0,
          resetUiToken: 0,
          focusWorldPointToken: 2,
          focusWorldPoint: {
            worldX: 12,
            worldZ: 34,
            pointKind: "bookmark",
            pointLabel: "Cron",
          },
        },
      },
      {
        resetViewToken: 0,
        resetUiToken: 0,
        focusWorldPointToken: 1,
        focusWorldPoint: null,
      },
    ),
    {
      version: 1,
      commands: {
        selectWorldPoint: {
          worldX: 12,
          worldZ: 34,
          pointKind: "bookmark",
          pointLabel: "Cron",
        },
      },
    },
  );
});

test("projectRuntimeSnapshotToSignals keeps only coarse runtime fields", () => {
  const patch = projectRuntimeSnapshotToSignals({
    ready: true,
    theme: { name: "night" },
    view: { viewMode: "3d" },
    selection: { pointKind: "clicked" },
    catalog: { layers: [{ layerId: "zone_mask" }] },
    statuses: { layersStatus: "ready" },
    lastDiagnostic: { note: "ok" },
    hover: { shouldNotLeak: true },
    filters: { shouldNotLeak: true },
  });

  assert.deepEqual(patch, {
    _map_runtime: {
      ready: true,
      theme: { name: "night" },
      ui: { bookmarks: [] },
      view: { viewMode: "3d" },
      selection: { pointKind: "clicked" },
      catalog: { layers: [{ layerId: "zone_mask" }] },
      statuses: { layersStatus: "ready" },
      lastDiagnostic: { note: "ok" },
    },
  });
});

test("projectSessionSnapshotToSignals keeps only restorable session fields", () => {
  const patch = projectSessionSnapshotToSignals({
    view: { viewMode: "2d", camera: { zoom: 2 } },
    selection: { pointKind: "bookmark" },
    hover: { shouldNotLeak: true },
  });

  assert.deepEqual(patch, {
    _map_session: {
      view: { viewMode: "2d", camera: { zoom: 2 } },
      selection: { pointKind: "bookmark" },
    },
  });
});

test("projectRuntimeSnapshotToSignals keeps runtime bookmark details ephemeral", () => {
  const patch = projectRuntimeSnapshotToSignals({
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
  });

  assert.deepEqual(patch, {
    _map_runtime: {
      ready: false,
      theme: {},
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
      view: {},
      selection: {},
      catalog: {},
      statuses: {},
      lastDiagnostic: null,
    },
  });
});
