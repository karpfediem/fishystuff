import { test } from "bun:test";
import assert from "node:assert/strict";

const {
  createDeferredBridgeStateRefresher,
  createBridgeInputPatchCoordinator,
  apiFailureStatusLines,
  deferAfterAnimationFrames,
  bridgeSnapshotMatchesRestoreView,
  changedSignalPatch,
  runtimeStatusLines,
  restoreViewPatchFromSignalPatch,
  renderRuntimeStatusSurface,
  resolveBridgeSnapshot,
  startWhenDomReady,
  start,
} = await import("./map-app-live.js");
const { buildSearchProjectionPatchForSignalPatch } = await import("./map-page-derived.js");

test("changedSignalPatch omits unchanged runtime branches", () => {
  const currentSignals = {
    _map_runtime: {
      ready: true,
      catalog: {
        layers: [{ layerId: "zone_mask" }],
        fish: [{ itemId: 1, name: "Yellow Corvina" }],
      },
      effectiveFilters: {
        layerOpacities: { zone_mask: 0.4 },
      },
      selection: { zoneRgb: "1,2,3" },
    },
    _map_session: {
      view: { viewMode: "2d", camera: { zoom: 3 } },
      selection: { zoneRgb: "1,2,3" },
    },
  };

  assert.deepEqual(
    changedSignalPatch({
      _map_runtime: {
        ready: true,
        catalog: {
          layers: [{ layerId: "zone_mask" }],
          fish: [{ itemId: 1, name: "Yellow Corvina" }],
        },
        effectiveFilters: {
          layerOpacities: { zone_mask: 0.6 },
        },
        selection: { zoneRgb: "1,2,3" },
      },
      _map_session: {
        view: { viewMode: "2d", camera: { zoom: 3 } },
        selection: { zoneRgb: "1,2,3" },
      },
    }, currentSignals),
    {
      _map_runtime: {
        effectiveFilters: {
          layerOpacities: { zone_mask: 0.6 },
        },
      },
    },
  );

  assert.equal(
    changedSignalPatch({
      _map_runtime: {
        ready: true,
        catalog: {
          layers: [{ layerId: "zone_mask" }],
          fish: [{ itemId: 1, name: "Yellow Corvina" }],
        },
        effectiveFilters: {
          layerOpacities: { zone_mask: 0.4 },
        },
        selection: { zoneRgb: "1,2,3" },
      },
    }, currentSignals),
    null,
  );
});

test("resolveBridgeSnapshot preserves coarse runtime fields on partial bridge events", () => {
  const currentSnapshot = {
    ready: true,
    theme: { name: "fishy" },
    effectiveFilters: {
      searchExpression: { type: "group", operator: "or", children: [] },
      sharedFishState: { caughtIds: [77], favouriteIds: [912] },
      zoneMembershipByLayer: {},
      semanticFieldFiltersByLayer: {},
    },
    ui: {
      bookmarks: [{ id: "bookmark-a", worldX: 1, worldZ: 2 }],
      bookmarkSelectedIds: ["bookmark-a"],
    },
    view: {
      viewMode: "2d",
      camera: { zoom: 2 },
    },
    selection: { pointKind: "clicked" },
    catalog: {
      layers: [{ layerId: "fish_evidence" }],
      patches: [{ patchId: "p1" }],
    },
    statuses: {
      layersStatus: "layers: ready",
    },
    lastDiagnostic: { note: "ok" },
  };

  const resolved = resolveBridgeSnapshot(
    {
      state: {
        view: {
          viewMode: "3d",
          camera: { distance: 900000 },
        },
      },
    },
    () => currentSnapshot,
  );

  assert.equal(resolved.ready, true);
  assert.deepEqual(resolved.theme, { name: "fishy" });
  assert.deepEqual(resolved.effectiveFilters, {
    searchExpression: { type: "group", operator: "or", children: [] },
    sharedFishState: { caughtIds: [77], favouriteIds: [912] },
    zoneMembershipByLayer: {},
    semanticFieldFiltersByLayer: {},
  });
  assert.deepEqual(resolved.ui, {
    bookmarks: [{ id: "bookmark-a", worldX: 1, worldZ: 2 }],
    bookmarkSelectedIds: ["bookmark-a"],
  });
  assert.deepEqual(resolved.catalog, {
    layers: [{ layerId: "fish_evidence" }],
    patches: [{ patchId: "p1" }],
  });
  assert.deepEqual(resolved.statuses, {
    layersStatus: "layers: ready",
  });
  assert.deepEqual(resolved.view, {
    viewMode: "3d",
    camera: { distance: 900000 },
  });
});

test("runtimeStatusLines returns stable labels for bridge statuses", () => {
  assert.deepEqual(runtimeStatusLines({
    metaStatus: "meta: loaded",
    zonesStatus: "zones: 287",
    fishStatus: "",
  }), [
    { key: "metaStatus", label: "Meta", status: "meta: loaded" },
    { key: "zonesStatus", label: "Zones", status: "zones: 287" },
  ]);
});

test("apiFailureStatusLines selects retrying API statuses", () => {
  assert.deepEqual(apiFailureStatusLines({
    metaStatus: "meta: request failed; retrying in 2s (Failed to fetch)",
    layersStatus: "layers: waiting for API",
    zonesStatus: "zones: 287",
    pointsStatus: "points: mode=grid_aggregate rev=events-1 snapshot=network-initial api=\"request failed; retrying in 4s (Failed to fetch)\"",
    fishStatus: "fish: request closed",
  }), [
    {
      key: "metaStatus",
      label: "Meta",
      status: "request failed; retrying in 2s (Failed to fetch)",
    },
    {
      key: "pointsStatus",
      label: "Points",
      status: "request failed; retrying in 4s (Failed to fetch)",
    },
    { key: "fishStatus", label: "Fish", status: "request closed" },
  ]);
});

test("apiFailureStatusLines ignores normal network snapshot diagnostics", () => {
  assert.deepEqual(apiFailureStatusLines({
    metaStatus: "meta: loaded",
    pointsStatus: "points: mode=grid_aggregate rev=events-69c20788fafa75e8 snapshot_events=47549 idx_bucket=128 cluster_bucket=256 candidates=47549 represented=47549 rendered_points=0 rendered_clusters=613 snapshot=network-initial",
  }), []);
});

test("renderRuntimeStatusSurface syncs map API failures to page health", () => {
  const calls = [];
  const previousHealth = globalThis.__fishystuffPageHealth;
  globalThis.__fishystuffPageHealth = {
    syncSourceIssues(source, issues) {
      calls.push({ source, issues });
    },
  };
  try {
    renderRuntimeStatusSurface({
      querySelector() {
        return null;
      },
    }, {
      pointsStatus: "points: mode=grid_aggregate rev=events-1 snapshot=network-initial api=\"request failed; retrying in 4s (Failed to fetch)\"",
    });

    assert.equal(calls.length, 1);
    assert.equal(calls[0].source, "map-api");
    assert.equal(calls[0].issues.length, 1);
    assert.equal(calls[0].issues[0].id, "map-api:pointsStatus");
    assert.equal(calls[0].issues[0].detail, "request failed; retrying in 4s (Failed to fetch)");

    renderRuntimeStatusSurface({
      querySelector() {
        return null;
      },
    }, {
      pointsStatus: "points: mode=grid_aggregate snapshot=network-initial",
    });
    assert.equal(calls.length, 2);
    assert.deepEqual(calls[1], { source: "map-api", issues: [] });
  } finally {
    if (previousHealth === undefined) {
      delete globalThis.__fishystuffPageHealth;
    } else {
      globalThis.__fishystuffPageHealth = previousHealth;
    }
  }
});

test("resolveBridgeSnapshot falls back to the current full snapshot when event state is missing", () => {
  const currentSnapshot = {
    ready: true,
    effectiveFilters: {
      searchExpression: { type: "group", operator: "or", children: [] },
      sharedFishState: { caughtIds: [], favouriteIds: [] },
      zoneMembershipByLayer: {},
      semanticFieldFiltersByLayer: {},
    },
    ui: {
      bookmarks: [{ id: "bookmark-a", worldX: 1, worldZ: 2 }],
    },
    catalog: {
      layers: [{ layerId: "zone_mask" }],
    },
  };

  assert.deepEqual(resolveBridgeSnapshot({}, () => currentSnapshot), currentSnapshot);
});

test("resolveBridgeSnapshot replaces selection as an atomic selected-element branch", () => {
  const resolved = resolveBridgeSnapshot(
    {
      state: {
        selection: {
          detailsGeneration: 2,
          pointKind: "waypoint",
          pointLabel: "Chunsu",
          worldX: 300,
          worldZ: 400,
        },
      },
    },
    () => ({
      selection: {
        detailsGeneration: 1,
        pointKind: "clicked",
        pointLabel: "Serendia Terrain",
        worldX: 100,
        worldZ: 200,
        layerSamples: [{ label: "Serendia Terrain" }],
      },
    }),
  );

  assert.deepEqual(resolved.selection, {
    detailsGeneration: 2,
    pointKind: "waypoint",
    pointLabel: "Chunsu",
    worldX: 300,
    worldZ: 400,
  });
});

test("restore view helpers normalize signal patches and detect matching bridge snapshots", () => {
  const restorePatch = restoreViewPatchFromSignalPatch({
    _map_session: {
      view: {
        viewMode: "2d",
        camera: { centerWorldX: 100, centerWorldZ: 200, zoom: 3 },
      },
    },
  });

  assert.deepEqual(restorePatch?.commands?.restoreView, {
    viewMode: "2d",
    camera: { centerWorldX: 100, centerWorldZ: 200, zoom: 3 },
  });
  assert.equal(
    bridgeSnapshotMatchesRestoreView(
      { view: { viewMode: "2d", camera: { centerWorldX: 100, centerWorldZ: 200, zoom: 3 } } },
      restorePatch.commands.restoreView,
    ),
    true,
  );
  assert.equal(
    bridgeSnapshotMatchesRestoreView(
      { view: { viewMode: "2d", camera: { centerWorldX: 100, centerWorldZ: 200, zoom: 9.2 } } },
      restorePatch.commands.restoreView,
    ),
    true,
  );
  assert.equal(
    bridgeSnapshotMatchesRestoreView(
      { view: { viewMode: "2d", camera: { centerWorldX: -1, centerWorldZ: 200, zoom: 3 } } },
      restorePatch.commands.restoreView,
    ),
    false,
  );
});

test("buildSearchProjectionPatchForSignalPatch projects selected search terms against the patched signal state", () => {
  const patch = buildSearchProjectionPatchForSignalPatch(
    {
      _map_ui: {
        search: {
          selectedTerms: [],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [],
          zoneRgbs: [],
          semanticFieldIdsByLayer: {},
          fishFilterTerms: [],
        },
      },
    },
    {
      _map_ui: {
        search: {
          selectedTerms: [{ kind: "zone", zoneRgb: 123456 }],
        },
      },
    },
  );

  assert.deepEqual(patch, {
    _map_bridged: {
      filters: {
        fishIds: [],
        zoneRgbs: [123456],
        semanticFieldIdsByLayer: {
          zone_mask: [123456],
        },
        fishFilterTerms: [],
        patchId: null,
        fromPatchId: null,
        toPatchId: null,
        searchExpression: {
          type: "group",
          operator: "or",
          children: [{ type: "term", term: { kind: "zone", zoneRgb: 123456 } }],
        },
      },
    },
  });
});

test("createBridgeInputPatchCoordinator defers signal-derived bridge patches until bridge sync completes", () => {
  const calls = [];
  const coordinator = createBridgeInputPatchCoordinator({
    patchBridgeFromSignals() {
      calls.push("patch");
    },
  });

  assert.equal(coordinator.requestBridgePatch(), true);
  assert.deepEqual(calls, ["patch"]);

  coordinator.runBridgeSync(() => {
    assert.equal(coordinator.isSyncingFromBridge(), true);
    assert.equal(coordinator.requestBridgePatch(), false);
    assert.equal(coordinator.requestBridgePatch(), false);
    assert.deepEqual(calls, ["patch"]);
  });

  assert.equal(coordinator.isSyncingFromBridge(), false);
  assert.deepEqual(calls, ["patch", "patch"]);
});

test("createDeferredBridgeStateRefresher refreshes once on the next frame", () => {
  const snapshots = [];
  const scheduled = [];
  const cancelled = [];
  const refresher = createDeferredBridgeStateRefresher({
    bridge: {
      refreshCurrentStateNow() {
        return { ready: true, filters: { layerIdsVisible: ["zone_mask"] } };
      },
    },
    onSnapshot(snapshot) {
      snapshots.push(snapshot);
    },
    requestAnimationFrameImpl(callback) {
      scheduled.push(callback);
      return scheduled.length;
    },
    cancelAnimationFrameImpl(frameId) {
      cancelled.push(frameId);
    },
  });

  refresher.schedule();
  refresher.schedule();

  assert.equal(cancelled.length, 1);
  assert.equal(snapshots.length, 0);

  const nextFrame = scheduled.at(-1);
  nextFrame();

  assert.deepEqual(snapshots, [{ ready: true, filters: { layerIdsVisible: ["zone_mask"] } }]);
});

test("deferAfterAnimationFrames waits for the requested number of animation frames", () => {
  const scheduled = [];
  const calls = [];

  deferAfterAnimationFrames(
    () => {
      calls.push("done");
    },
    {
      frames: 2,
      requestAnimationFrameImpl(callback) {
        scheduled.push(callback);
        return scheduled.length;
      },
    },
  );

  assert.deepEqual(calls, []);
  assert.equal(scheduled.length, 1);
  scheduled.shift()();
  assert.deepEqual(calls, []);
  assert.equal(scheduled.length, 1);
  scheduled.shift()();
  assert.deepEqual(calls, ["done"]);
});

test("map-app-live exports explicit start hooks", () => {
  assert.equal(typeof start, "function");
  assert.equal(typeof startWhenDomReady, "function");
});
