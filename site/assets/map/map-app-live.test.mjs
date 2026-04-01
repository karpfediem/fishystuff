import test from "node:test";
import assert from "node:assert/strict";

const {
  buildSearchProjectionPatchForSignalPatch,
  createDeferredBridgeStateRefresher,
  deferAfterAnimationFrames,
  resolveBridgeSnapshot,
  routeLiveControllerPatch,
  startWhenDomReady,
  start,
} = await import("./map-app-live.js");

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

test("routeLiveControllerPatch schedules only the relevant live controllers", () => {
  const calls = [];
  const controller = (name, method) => ({
    [method]() {
      calls.push(name);
    },
  });

  routeLiveControllerPatch({
    patch: {
      _map_ui: {
        windowUi: {
          settings: { open: false },
        },
        search: {
          open: true,
        },
      },
      _map_bridged: {
        filters: {
          layerIdsVisible: ["zone_mask"],
        },
      },
      _map_bookmarks: {
        entries: [{ id: "bookmark-a" }],
      },
      _map_runtime: {
        catalog: {
          layers: [{ layerId: "zone_mask" }],
        },
      },
    },
    windowManager: controller("window", "scheduleApplyFromSignals"),
    patchPicker: controller("patch-picker", "scheduleRender"),
    hoverTooltip: controller("hover", "scheduleRender"),
    layerPanel: controller("layer", "scheduleRender"),
    searchPanel: controller("search", "scheduleRender"),
    bookmarkPanel: controller("bookmark", "scheduleRender"),
    infoPanel: controller("info", "handleSignalPatch"),
  });

  assert.deepEqual(calls, ["window", "hover", "layer", "search", "bookmark", "info"]);
});

test("map-app-live exports explicit start hooks", () => {
  assert.equal(typeof start, "function");
  assert.equal(typeof startWhenDomReady, "function");
});
