import { createMapApp } from "./map-app.js";
import FishyMapBridge, { createEmptySnapshot, snapshotToRestorePatch } from "./map-host.js";
import { createMapPageLive } from "./map-page-live.js";
import { createMapPagePersistController } from "./map-page-persist.js";
import {
  DEFAULT_MAP_ACTION_SIGNAL_STATE,
  DEFAULT_MAP_BOOKMARKS_SIGNAL_STATE,
  DEFAULT_MAP_BRIDGED_SIGNAL_STATE,
  DEFAULT_MAP_SESSION_SIGNAL_STATE,
  DEFAULT_MAP_UI_SIGNAL_STATE,
} from "./map-signal-contract.js";
import { parseQuerySignalPatch } from "./map-query-state.js";
import {
  createMapBookmarkPanelController,
  patchTouchesBookmarkSignals,
} from "./map-bookmark-panel-live.js";
import { createMapHoverTooltipController } from "./map-hover-tooltip-live.js";
import { createMapInfoPanelController } from "./map-info-panel-live.js";
import {
  createMapLayerPanelController,
  patchTouchesLayerPanelSignals,
} from "./map-layer-panel-live.js";
import {
  createMapPatchPickerController,
  patchTouchesPatchPickerSignals,
} from "./map-patch-picker-live.js";
import {
  createMapSearchPanelController,
  patchTouchesSearchPanelSignals,
} from "./map-search-panel-live.js";
import {
  dispatchShellPatchedSignalEvent,
  FISHYMAP_SIGNAL_PATCHED_EVENT,
  FISHYMAP_SIGNAL_PATCH_EVENT,
  combineSignalPatches,
} from "./map-signal-patch.js";
import { createMapWindowManager } from "./map-window-manager.js";
import { buildSearchProjectionSignalPatch } from "./map-search-projection.js";
import { loadZoneCatalog } from "./map-zone-catalog.js";
import { patchTouchesHoverTooltipSignals } from "./map-hover-facts.js";

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function mergeProjectionPatch(target, patch) {
  if (!isPlainObject(target) || !isPlainObject(patch)) {
    return target;
  }
  for (const [key, value] of Object.entries(patch)) {
    if (Array.isArray(value)) {
      target[key] = cloneJson(value);
      continue;
    }
    if (isPlainObject(value)) {
      const nextTarget = isPlainObject(target[key]) ? target[key] : {};
      target[key] = nextTarget;
      mergeProjectionPatch(nextTarget, value);
      continue;
    }
    target[key] = value;
  }
  return target;
}

export function buildSearchProjectionPatchForSignalPatch(signals, patch) {
  if (patch?._map_ui?.search?.selectedTerms == null) {
    return null;
  }
  const nextSignals = isPlainObject(signals) ? cloneJson(signals) : {};
  mergeProjectionPatch(nextSignals, patch);
  return buildSearchProjectionSignalPatch(nextSignals);
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

function patchTouchesWindowUi(patch) {
  return Boolean(isPlainObject(patch) && isPlainObject(patch._map_ui) && isPlainObject(patch._map_ui.windowUi));
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

export function routeLiveControllerPatch({
  patch,
  windowManager = null,
  patchPicker = null,
  hoverTooltip = null,
  layerPanel = null,
  searchPanel = null,
  bookmarkPanel = null,
  infoPanel = null,
} = {}) {
  if (windowManager && patchTouchesWindowUi(patch)) {
    windowManager.scheduleApplyFromSignals();
  }
  if (patchPicker && patchTouchesPatchPickerSignals(patch)) {
    patchPicker.scheduleRender();
  }
  if (hoverTooltip && patchTouchesHoverTooltipSignals(patch)) {
    hoverTooltip.scheduleRender();
  }
  if (layerPanel && patchTouchesLayerPanelSignals(patch)) {
    layerPanel.scheduleRender();
  }
  if (searchPanel && patchTouchesSearchPanelSignals(patch)) {
    searchPanel.scheduleRender();
  }
  if (bookmarkPanel && patchTouchesBookmarkSignals(patch)) {
    bookmarkPanel.scheduleRender();
  }
  if (infoPanel && typeof infoPanel.handleSignalPatch === "function") {
    infoPanel.handleSignalPatch(patch);
  }
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
    isReady: () => true,
    readSnapshot: () => page.signalObject?.() || null,
  });
  pagePersistor.seed(page.signalObject?.() || null);
  let windowManager = null;
  let patchPicker = null;
  let hoverTooltip = null;
  let bookmarkPanel = null;
  let layerPanel = null;
  let searchPanel = null;
  let zoneInfoPanel = null;

  function dispatchSignalPatch(patch) {
    if (!patch || typeof patch !== "object") {
      return;
    }
    page.patchSignals(patch);
    dispatchShellPatchedSignalEvent(shell, patch);
  }

  const queryPatch = parseQuerySignalPatch(globalThis.location?.href);
  if (queryPatch) {
    dispatchSignalPatch(queryPatch);
  }
  const initialSearchProjectionPatch = buildSearchProjectionSignalPatch(page.signalObject?.() || {});
  if (initialSearchProjectionPatch) {
    dispatchSignalPatch(initialSearchProjectionPatch);
  }

  const app = createMapApp();
  const bridge = FishyMapBridge;
  windowManager = createMapWindowManager({
    shell,
    getSignals: signals,
  });
  bookmarkPanel = createMapBookmarkPanelController({
    shell,
    getSignals: signals,
  });
  hoverTooltip = createMapHoverTooltipController({
    shell,
    getSignals: signals,
  });
  zoneInfoPanel = createMapInfoPanelController({
    shell,
    getSignals: signals,
  });
  patchPicker = createMapPatchPickerController({
    shell,
    getSignals: signals,
  });
  layerPanel = createMapLayerPanelController({
    shell,
    getSignals: signals,
  });
  searchPanel = createMapSearchPanelController({
    shell,
    getSignals: signals,
  });
  let syncingFromBridge = false;
  let applyingInternalSignalPatch = false;
  let mounted = false;
  let lastBridgePatchJson = "";
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
    const patch = app.nextBridgePatch(signals(), {
      currentState: currentBridgeState(),
    });
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
      dispatchSignalPatch(
        combineSignalPatches(app.projectRuntimeSnapshot(snapshot), app.projectSessionSnapshot(snapshot)),
      );
    } finally {
      syncingFromBridge = false;
    }
  }

  function handleBridgeStateEvent(event) {
    const snapshot = resolveBridgeSnapshot(event?.detail, currentBridgeState);
    patchSignalsFromBridge(snapshot);
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
    pagePersistor.handleSignalPatch(patch);
    const searchProjectionPatch = buildSearchProjectionPatchForSignalPatch(signals(), patch);
    const effectivePatch = searchProjectionPatch
      ? combineSignalPatches(patch, searchProjectionPatch)
      : patch;
    routeLiveControllerPatch({
      patch: effectivePatch,
      windowManager,
      patchPicker,
      hoverTooltip,
      layerPanel,
      searchPanel,
      bookmarkPanel,
      infoPanel: zoneInfoPanel,
    });
    if (searchProjectionPatch) {
      applyInternalSignalPatch(searchProjectionPatch);
    }
    if (applyingInternalSignalPatch) {
      return;
    }
    if (!patchTouchesLiveBridgeInputs(effectivePatch)) {
      return;
    }

    const nextActionState = signals()?._map_actions || {};
    const resetUiToken = Number(nextActionState.resetUiToken || 0);
    const previousResetUiToken = Number(actionState.resetUiToken || 0);
    if (resetUiToken > previousResetUiToken) {
      applyInternalSignalPatch(buildResetUiPatch());
    }

    patchBridgeFromSignals();
    if (!syncingFromBridge && effectivePatch?._map_bookmarks?.entries != null) {
      scheduleBookmarkDetailsRefresh();
    }
  });

  const initialPatch = app.nextBridgePatch(signals(), {
    currentState: createEmptySnapshot(),
  });
  const initialRestorePatch = snapshotToRestorePatch(signals()?._map_session || {});
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
  actionState = app.consumeSignals(signals());
  lastBridgePatchJson = JSON.stringify(
    app.nextBridgePatch(signals(), {
      currentState: currentBridgeState(),
    }),
  );
  patchSignalsFromBridge(currentBridgeState());
  windowManager.applyFromSignals();
  bookmarkPanel.render();
  hoverTooltip.render();
  zoneInfoPanel.render();
  void zoneInfoPanel.refreshZoneLootSummary();
  patchPicker.render();
  layerPanel.render();
  searchPanel.render();
  void loadZoneCatalog().then((zoneCatalog) => {
    hoverTooltip.setZoneCatalog(zoneCatalog);
    layerPanel.setZoneCatalog(zoneCatalog);
    bookmarkPanel.setZoneCatalog(zoneCatalog);
    zoneInfoPanel.setZoneCatalog(zoneCatalog);
    void zoneInfoPanel.refreshZoneLootSummary();
    searchPanel.setZoneCatalog(zoneCatalog);
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
