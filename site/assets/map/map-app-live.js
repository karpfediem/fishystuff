import { createMapApp } from "./map-app.js";
import FishyMapBridge, { createEmptySnapshot, snapshotToRestorePatch } from "./map-host.js";
import { createMapPageDerivedController } from "./map-page-derived.js";
import { createMapPageLive } from "./map-page-live.js";
import { createMapLifecycleMetrics, createMapOtelMetricsReporter } from "./map-otel-metrics.js";
import { createMapPagePersistController } from "./map-page-persist.js";
import { bindMapPresetController } from "./map-presets.js";
import { languageReady } from "./map-i18n.js";
import {
  DEFAULT_MAP_ACTION_SIGNAL_STATE,
  DEFAULT_MAP_BOOKMARKS_SIGNAL_STATE,
  DEFAULT_MAP_BRIDGED_SIGNAL_STATE,
  DEFAULT_MAP_SESSION_SIGNAL_STATE,
  DEFAULT_MAP_UI_SIGNAL_STATE,
} from "./map-signal-contract.js";
import "./map-bookmark-panel-element.js";
import "./map-hover-tooltip-element.js";
import "./map-info-panel-element.js";
import "./map-layer-panel-element.js";
import "./map-search-panel-element.js";
import "./map-window-manager-element.js";
import {
  FISHYMAP_SIGNAL_PATCHED_EVENT,
  FISHYMAP_SIGNAL_PATCH_EVENT,
  combineSignalPatches,
} from "./map-signal-patch.js";
import { loadZoneCatalog } from "./map-zone-catalog.js";
import { dispatchShellZoneCatalogReadyEvent } from "./map-zone-catalog-live.js";

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function mergeSnapshotBranch(baseValue, patchValue) {
  if (isPlainObject(baseValue) || isPlainObject(patchValue)) {
    return {
      ...(isPlainObject(baseValue) ? cloneJson(baseValue) : {}),
      ...(isPlainObject(patchValue) ? cloneJson(patchValue) : {}),
    };
  }
  if (Array.isArray(patchValue)) {
    return cloneJson(patchValue);
  }
  if (patchValue !== undefined) {
    return cloneJson(patchValue);
  }
  return baseValue === undefined ? undefined : cloneJson(baseValue);
}

export function resolveBridgeSnapshot(eventDetail, readCurrentState) {
  const currentSnapshot =
    typeof readCurrentState === "function" ? readCurrentState() : createEmptySnapshot();
  const baseSnapshot = isPlainObject(currentSnapshot) ? currentSnapshot : createEmptySnapshot();
  const patchSnapshot = isPlainObject(eventDetail?.state) ? eventDetail.state : null;
  if (!patchSnapshot) {
    return cloneJson(baseSnapshot);
  }
  return {
    ...cloneJson(baseSnapshot),
    ...cloneJson(patchSnapshot),
    theme: mergeSnapshotBranch(baseSnapshot.theme, patchSnapshot.theme),
    ui: mergeSnapshotBranch(baseSnapshot.ui, patchSnapshot.ui),
    view: mergeSnapshotBranch(baseSnapshot.view, patchSnapshot.view),
    selection: mergeSnapshotBranch(baseSnapshot.selection, patchSnapshot.selection),
    hover: mergeSnapshotBranch(baseSnapshot.hover, patchSnapshot.hover),
    catalog: mergeSnapshotBranch(baseSnapshot.catalog, patchSnapshot.catalog),
    statuses: mergeSnapshotBranch(baseSnapshot.statuses, patchSnapshot.statuses),
    effectiveFilters: mergeSnapshotBranch(
      baseSnapshot.effectiveFilters,
      patchSnapshot.effectiveFilters,
    ),
  };
}

function patchTouchesLiveBridgeInputs(patch) {
  if (!isPlainObject(patch)) {
    return false;
  }
  return (
    "_map_bridged" in patch ||
    "_map_actions" in patch ||
    "_map_bookmarks" in patch ||
    "_shared_fish" in patch
  );
}

function buildResetUiPatch() {
  return {
    _map_ui: cloneJson(DEFAULT_MAP_UI_SIGNAL_STATE),
    _map_bridged: cloneJson(DEFAULT_MAP_BRIDGED_SIGNAL_STATE),
    _map_bookmarks: cloneJson(DEFAULT_MAP_BOOKMARKS_SIGNAL_STATE),
    _map_session: cloneJson(DEFAULT_MAP_SESSION_SIGNAL_STATE),
    _map_actions: cloneJson(DEFAULT_MAP_ACTION_SIGNAL_STATE),
  };
}

function stableJson(value) {
  return JSON.stringify(value ?? null);
}

export function changedSignalPatch(projectedPatch, currentSignals) {
  if (!isPlainObject(projectedPatch)) {
    return null;
  }
  const nextPatch = {};
  for (const [rootKey, branch] of Object.entries(projectedPatch)) {
    if (!isPlainObject(branch)) {
      if (!Object.is(branch, currentSignals?.[rootKey])) {
        nextPatch[rootKey] = cloneJson(branch);
      }
      continue;
    }
    const currentBranch = isPlainObject(currentSignals?.[rootKey])
      ? currentSignals[rootKey]
      : {};
    const nextBranch = {};
    for (const [key, value] of Object.entries(branch)) {
      if (stableJson(value) === stableJson(currentBranch[key])) {
        continue;
      }
      nextBranch[key] = cloneJson(value);
    }
    if (Object.keys(nextBranch).length) {
      nextPatch[rootKey] = nextBranch;
    }
  }
  return Object.keys(nextPatch).length ? nextPatch : null;
}

function restoreViewPatchFromSessionView(view) {
  const patch = snapshotToRestorePatch({ view });
  return patch?.commands?.restoreView ? patch : null;
}

export function restoreViewPatchFromSignalPatch(patch) {
  const view = patch?._map_session?.view;
  return isPlainObject(view) ? restoreViewPatchFromSessionView(view) : null;
}

export function bridgeSnapshotMatchesRestoreView(snapshot, restoreView) {
  if (!restoreView) {
    return true;
  }
  const snapshotRestoreView = restoreViewPatchFromSessionView(snapshot?.view)?.commands?.restoreView;
  if (!snapshotRestoreView) {
    return false;
  }
  if (stableJson(snapshotRestoreView) === stableJson(restoreView)) {
    return true;
  }
  if (snapshotRestoreView.viewMode !== restoreView.viewMode) {
    return false;
  }
  const snapshotCamera = isPlainObject(snapshotRestoreView.camera) ? snapshotRestoreView.camera : {};
  const restoreCamera = isPlainObject(restoreView.camera) ? restoreView.camera : {};
  const stableCameraKeys = Object.keys(restoreCamera).filter((key) => key !== "zoom" && key !== "distance");
  return stableCameraKeys.length > 0
    && stableCameraKeys.every((key) => Object.is(snapshotCamera[key], restoreCamera[key]));
}

export function deferAfterAnimationFrames(
  callback,
  {
    frames = 1,
    requestAnimationFrameImpl = globalThis.requestAnimationFrame?.bind(globalThis),
    setTimeoutImpl = globalThis.setTimeout?.bind(globalThis),
  } = {},
) {
  const remainingFrames = Math.max(0, Number.parseInt(frames, 10) || 0);
  const invoke = () => {
    if (typeof callback === "function") {
      callback();
    }
  };

  const step = (remaining) => {
    if (remaining <= 0) {
      invoke();
      return;
    }
    const next = () => step(remaining - 1);
    if (typeof requestAnimationFrameImpl === "function") {
      requestAnimationFrameImpl(next);
      return;
    }
    if (typeof setTimeoutImpl === "function") {
      setTimeoutImpl(next, 16);
      return;
    }
    next();
  };

  step(remainingFrames);
}

export function createDeferredBridgeStateRefresher({
  bridge,
  onSnapshot,
  requestAnimationFrameImpl = globalThis.requestAnimationFrame?.bind(globalThis),
  cancelAnimationFrameImpl = globalThis.cancelAnimationFrame?.bind(globalThis),
} = {}) {
  let frameId = 0;

  function cancel() {
    if (!frameId || typeof cancelAnimationFrameImpl !== "function") {
      frameId = 0;
      return;
    }
    cancelAnimationFrameImpl(frameId);
    frameId = 0;
  }

  function run() {
    frameId = 0;
    const snapshot =
      typeof bridge?.refreshCurrentStateNow === "function"
        ? bridge.refreshCurrentStateNow()
        : typeof bridge?.getCurrentState === "function"
          ? bridge.getCurrentState()
          : null;
    if (!snapshot || typeof onSnapshot !== "function") {
      return;
    }
    onSnapshot(snapshot);
  }

  return Object.freeze({
    schedule() {
      cancel();
      if (typeof requestAnimationFrameImpl !== "function") {
        run();
        return;
      }
      frameId = requestAnimationFrameImpl(run) || 0;
      if (!frameId) {
        run();
      }
    },
    cancel,
  });
}

export async function start() {
  const shell = document.getElementById("map-page-shell");
  const canvas = document.getElementById("bevy");
  if (!(shell instanceof HTMLElement) || !(canvas instanceof HTMLCanvasElement)) {
    return;
  }

  const page = createMapPageLive();
  page.start();
  await page.whenRestored();
  const pagePersistor = createMapPagePersistController({
    globalRef: globalThis,
    shell,
    isReady: () => true,
    readSnapshot: () => page.signalObject?.() || null,
  });
  pagePersistor.seed(page.signalObject?.() || null);

  function dispatchSignalPatch(patch) {
    if (!patch || typeof patch !== "object") {
      return;
    }
    page.patchSignals(patch);
  }
  const mapPresetController = bindMapPresetController({
    shell,
    readSignals: signals,
    applyPatch: dispatchSignalPatch,
    readBridgeState: currentBridgeState,
  });
  void mapPresetController;
  const pageDerived = createMapPageDerivedController({
    globalRef: globalThis,
    shell,
    readSignals: signals,
    dispatchPatch: dispatchSignalPatch,
  });
  pageDerived.applyInitialPatches();
  pageDerived.start(shell);

  const app = createMapApp();
  const bridge = FishyMapBridge;
  const otelMetricsReporter = createMapOtelMetricsReporter({ bridge });
  const lifecycleMetrics = createMapLifecycleMetrics();
  const nowMs = () => {
    const performanceNow = globalThis.performance?.now?.();
    if (Number.isFinite(performanceNow)) {
      return performanceNow;
    }
    return Date.now();
  };
  const runtimeStartedAtMs = nowMs();
  let syncingFromBridge = false;
  let applyingInternalSignalPatch = false;
  let mounted = false;
  let lastBridgePatchJson = "";
  let pendingBridgeRestoreView = null;
  let actionState = app.readLastActionState();
  const bridgeStateRefresher = createDeferredBridgeStateRefresher({
    bridge,
    onSnapshot: patchSignalsFromBridge,
  });
  const scheduleBookmarkDetailsRefresh = () => {
    deferAfterAnimationFrames(
      () => {
        bridgeStateRefresher.schedule();
      },
      { frames: 2 },
    );
  };
  const recordRuntimeReady = () => {
    lifecycleMetrics.recordRuntimeReady(Math.max(0, nowMs() - runtimeStartedAtMs));
  };

  function signals() {
    return page.signalObject?.() || null;
  }

  function currentBridgeState() {
    try {
      return bridge.getCurrentState?.() || createEmptySnapshot();
    } catch (_error) {
      return createEmptySnapshot();
    }
  }

  function patchBridgeFromSignals() {
    if (!mounted || syncingFromBridge) {
      return;
    }
    const patch = app.nextBridgePatch(signals());
    const patchJson = JSON.stringify(patch);
    if (patchJson === lastBridgePatchJson) {
      return;
    }
    lastBridgePatchJson = patchJson;
    bridge.setState(patch);
    bridge.flushPendingPatchNow?.();
    actionState = app.consumeSignals(signals());
    bridgeStateRefresher.schedule();
  }

  function applyInternalSignalPatch(patch) {
    if (applyingInternalSignalPatch) {
      return;
    }
    applyingInternalSignalPatch = true;
    try {
      dispatchSignalPatch(patch);
    } finally {
      applyingInternalSignalPatch = false;
    }
  }

  function patchSignalsFromBridge(snapshot) {
    syncingFromBridge = true;
    try {
      const currentSignals = signals();
      const sessionPatch = pendingBridgeRestoreView
        ? (
            bridgeSnapshotMatchesRestoreView(snapshot, pendingBridgeRestoreView)
              ? app.projectSessionSnapshot(snapshot)
              : null
          )
        : app.projectSessionSnapshot(snapshot);
      if (pendingBridgeRestoreView && sessionPatch) {
        pendingBridgeRestoreView = null;
      }
      const projectedPatch = combineSignalPatches(app.projectRuntimeSnapshot(snapshot), sessionPatch);
      const changedPatch = changedSignalPatch(projectedPatch, currentSignals);
      if (changedPatch) {
        dispatchSignalPatch(changedPatch);
      }
    } finally {
      syncingFromBridge = false;
    }
  }

  function restoreBridgeViewFromSignalPatch(patch) {
    const restorePatch = restoreViewPatchFromSignalPatch(patch);
    const restoreView = restorePatch?.commands?.restoreView;
    if (!restoreView) {
      return false;
    }
    pendingBridgeRestoreView = cloneJson(restoreView);
    bridge.setState(restorePatch);
    return true;
  }

  function handleBridgeStateEvent(event) {
    const snapshot = resolveBridgeSnapshot(event?.detail, currentBridgeState);
    if (event?.type === "fishymap:view-changed") {
      if (
        pendingBridgeRestoreView &&
        bridgeSnapshotMatchesRestoreView(snapshot, pendingBridgeRestoreView)
      ) {
        pendingBridgeRestoreView = null;
      }
      return;
    }
    patchSignalsFromBridge(snapshot);
    if (event?.type === "fishymap:ready" || snapshot?.ready === true) {
      recordRuntimeReady();
    }
  }

  shell.addEventListener("fishymap:ready", handleBridgeStateEvent);
  shell.addEventListener("fishymap:state-changed", handleBridgeStateEvent);
  shell.addEventListener("fishymap:view-changed", handleBridgeStateEvent);
  shell.addEventListener("fishymap:selection-changed", handleBridgeStateEvent);
  shell.addEventListener("fishymap:diagnostic", handleBridgeStateEvent);
  shell.addEventListener(FISHYMAP_SIGNAL_PATCH_EVENT, (event) => {
    dispatchSignalPatch(event?.detail || null);
  });
  shell.addEventListener(FISHYMAP_SIGNAL_PATCHED_EVENT, (event) => {
    const patch = event?.detail || null;
    if (applyingInternalSignalPatch) {
      return;
    }
    const restoredBridgeView = syncingFromBridge ? false : restoreBridgeViewFromSignalPatch(patch);
    const touchesLiveBridgeInputs = patchTouchesLiveBridgeInputs(patch);
    if (!restoredBridgeView && !touchesLiveBridgeInputs) {
      return;
    }

    const nextActionState = {
      ...(signals()?._map_actions || {}),
      ...(patch?._map_actions || {}),
    };
    const resetUiToken = Number(nextActionState.resetUiToken || 0);
    const previousResetUiToken = Number(actionState.resetUiToken || 0);
    if (resetUiToken > previousResetUiToken) {
      applyInternalSignalPatch(buildResetUiPatch());
    }

    if (touchesLiveBridgeInputs) {
      patchBridgeFromSignals();
    }
    if (!syncingFromBridge && patch?._map_bookmarks?.entries != null) {
      scheduleBookmarkDetailsRefresh();
    }
  });

  const initialPatch = app.nextBridgePatch(signals());
  const initialRestorePatch = snapshotToRestorePatch(signals()?._map_session || {});
  pendingBridgeRestoreView = initialRestorePatch?.commands?.restoreView
    ? cloneJson(initialRestorePatch.commands.restoreView)
    : null;
  await languageReady();
  await bridge.mount(shell, {
    canvas,
    initialState: {
      ...cloneJson(initialPatch),
      ...(initialRestorePatch || {}),
      ...(initialPatch.filters || initialRestorePatch?.filters
        ? {
            filters: {
              ...(isPlainObject(initialRestorePatch?.filters)
                ? cloneJson(initialRestorePatch.filters)
                : {}),
              ...(isPlainObject(initialPatch.filters) ? cloneJson(initialPatch.filters) : {}),
            },
          }
        : {}),
      ...(initialPatch.ui || initialRestorePatch?.ui
        ? {
            ui: {
              ...(isPlainObject(initialRestorePatch?.ui) ? cloneJson(initialRestorePatch.ui) : {}),
              ...(isPlainObject(initialPatch.ui) ? cloneJson(initialPatch.ui) : {}),
            },
          }
        : {}),
      ...(initialPatch.commands || initialRestorePatch?.commands
        ? {
            commands: {
              ...(isPlainObject(initialRestorePatch?.commands)
                ? cloneJson(initialRestorePatch.commands)
                : {}),
              ...(isPlainObject(initialPatch.commands) ? cloneJson(initialPatch.commands) : {}),
            },
          }
        : {}),
    },
  });
  mounted = true;
  void otelMetricsReporter;
  actionState = app.consumeSignals(signals());
  lastBridgePatchJson = JSON.stringify(initialPatch);
  patchSignalsFromBridge(currentBridgeState());
  if (currentBridgeState()?.ready === true) {
    recordRuntimeReady();
  }
  patchBridgeFromSignals();
  void loadZoneCatalog().then((loadedZoneCatalog) => {
    dispatchShellZoneCatalogReadyEvent(
      shell,
      Array.isArray(loadedZoneCatalog) ? cloneJson(loadedZoneCatalog) : [],
    );
  });
}

export function startWhenDomReady() {
  const run = () => {
    start().catch((error) => {
      console.error("Fishy map app bootstrap failed", error);
    });
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", run, { once: true });
    return;
  }

  run();
}
