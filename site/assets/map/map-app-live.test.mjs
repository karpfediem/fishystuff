import test from "node:test";
import assert from "node:assert/strict";

const {
  createDeferredBridgeStateRefresher,
  deferAfterAnimationFrames,
  resolveBridgeSnapshot,
  shouldRefreshBridgeFromRuntimeEvent,
  startWhenDomReady,
  start,
} = await import("./map-app-live.js");
const { buildSearchProjectionPatchForSignalPatch } = await import("./map-page-derived.js");

test("resolveBridgeSnapshot preserves coarse runtime fields on partial bridge events", () => {
  const currentSnapshot = {
    ready: true,
    theme: { name: "fishy" },
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

test("resolveBridgeSnapshot falls back to the current full snapshot when event state is missing", () => {
  const currentSnapshot = {
    ready: true,
    ui: {
      bookmarks: [{ id: "bookmark-a", worldX: 1, worldZ: 2 }],
    },
    catalog: {
      layers: [{ layerId: "zone_mask" }],
    },
  };

  assert.deepEqual(resolveBridgeSnapshot({}, () => currentSnapshot), currentSnapshot);
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
      },
    },
  });
});

test("shouldRefreshBridgeFromRuntimeEvent reruns bridge projection when runtime catalog changes under fish filter terms", () => {
  assert.equal(
    shouldRefreshBridgeFromRuntimeEvent(
      {
        _map_bridged: {
          filters: {
            fishFilterTerms: ["missing"],
          },
        },
      },
      {
        state: {
          catalog: {
            fish: [{ fishId: 77 }],
          },
        },
      },
    ),
    true,
  );
  assert.equal(
    shouldRefreshBridgeFromRuntimeEvent(
      {
        _map_bridged: {
          filters: {
            fishFilterTerms: [],
          },
        },
      },
      {
        state: {
          catalog: {
            fish: [{ fishId: 77 }],
          },
        },
      },
    ),
    false,
  );
  assert.equal(
    shouldRefreshBridgeFromRuntimeEvent(
      {
        _map_bridged: {
          filters: {
            fishFilterTerms: [],
          },
        },
      },
      {
        state: {
          ready: true,
        },
      },
    ),
    false,
  );
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
