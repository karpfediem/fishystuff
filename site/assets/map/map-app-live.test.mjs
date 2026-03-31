import test from "node:test";
import assert from "node:assert/strict";

globalThis.__fishystuffMapAppAutoStart = false;
const {
  createDeferredBridgeStateRefresher,
  resolveBridgeSnapshot,
  waitForMapPageBootstrap,
} = await import("./map-app-live.js");
delete globalThis.__fishystuffMapAppAutoStart;

test("resolveBridgeSnapshot preserves coarse runtime fields on partial bridge events", () => {
  const currentSnapshot = {
    ready: true,
    theme: { name: "fishy" },
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
    catalog: {
      layers: [{ layerId: "zone_mask" }],
    },
  };

  assert.deepEqual(resolveBridgeSnapshot({}, () => currentSnapshot), currentSnapshot);
});

test("waitForMapPageBootstrap waits for page bootstrap globals to appear", async () => {
  const previousMap = globalThis.window?.__fishystuffMap;
  if (!globalThis.window) {
    globalThis.window = globalThis;
  }
  try {
    delete globalThis.window.__fishystuffMap;

    const bootstrapPromise = waitForMapPageBootstrap({
      timeoutMs: 200,
      pollIntervalMs: 1,
    });

    setTimeout(() => {
      globalThis.window.__fishystuffMap = {
        whenRestored() {
          return Promise.resolve();
        },
      };
    }, 5);

    const bootstrap = await bootstrapPromise;
    assert.equal(typeof bootstrap.page.whenRestored, "function");
  } finally {
    if (previousMap === undefined) {
      delete globalThis.window.__fishystuffMap;
    } else {
      globalThis.window.__fishystuffMap = previousMap;
    }
  }
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
