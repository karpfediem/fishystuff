import { createMapApp } from "./map-app.js";
import FishyMapBridge, { createEmptySnapshot, snapshotToRestorePatch } from "./map-host.js";
import {
  DEFAULT_MAP_ACTION_SIGNAL_STATE,
  DEFAULT_MAP_BOOKMARKS_SIGNAL_STATE,
  DEFAULT_MAP_BRIDGED_SIGNAL_STATE,
  DEFAULT_MAP_SESSION_SIGNAL_STATE,
  DEFAULT_MAP_UI_SIGNAL_STATE,
} from "./map-signal-contract.js";
import { parseQuerySignalPatch } from "./map-query-state.js";
import { createMapBookmarkPanelController } from "./map-bookmark-panel-live.js";
import { createMapHoverTooltipController } from "./map-hover-tooltip-live.js";
import { createMapInfoPanelController } from "./map-info-panel-live.js";
import { createMapLayerPanelController, patchTouchesLayerPanelSignals } from "./map-layer-panel-live.js";
import { createMapSearchPanelController } from "./map-search-panel-live.js";
import { combineSignalPatches, dispatchShellSignalPatch } from "./map-signal-patch.js";
import { createMapWindowManager } from "./map-window-manager.js";
import { patchTouchesBookmarkSignals } from "./map-bookmark-state.js";
import { patchTouchesHoverTooltipSignals } from "./map-hover-facts.js";
import { patchTouchesInfoSignals } from "./map-info-state.js";
import { patchTouchesSearchPanelSignals } from "./map-search-state.js";
import { loadZoneCatalog } from "./map-zone-catalog.js";

const FISHYMAP_DATASTAR_SIGNAL_PATCH_EVENT = "fishymap:datastar-signal-patch";

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

function currentPageBootstrap(shell) {
  const page = shell?.__fishystuffMapPage;
  if (
    !page ||
    typeof page.whenRestored !== "function" ||
    typeof page.signalObject !== "function"
  ) {
    return null;
  }
  return { shell, page };
}

function wait(delayMs) {
  return new Promise((resolve) => {
    globalThis.setTimeout(resolve, delayMs);
  });
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

export async function waitForMapPageBootstrap({
  shell = globalThis.document?.getElementById?.("map-page-shell") || null,
  timeoutMs = 5000,
  pollIntervalMs = 16,
} = {}) {
  const deadline = Date.now() + timeoutMs;
  let bootstrap = currentPageBootstrap(shell);
  while (!bootstrap) {
    if (Date.now() >= deadline) {
      throw new Error("timed out waiting for map shell signal bootstrap");
    }
    await wait(pollIntervalMs);
    bootstrap = currentPageBootstrap(
      shell || globalThis.document?.getElementById?.("map-page-shell") || null,
    );
  }
  return bootstrap;
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

  const { page } = await waitForMapPageBootstrap({ shell });

  await page.whenRestored();

  const queryPatch = parseQuerySignalPatch(globalThis.location?.href);
  if (queryPatch) {
    dispatchShellSignalPatch(shell, queryPatch);
  }

  const app = createMapApp();
  const bridge = FishyMapBridge;
  const windowManager = createMapWindowManager({
    shell,
    getSignals: signals,
    listenToSignalPatches: false,
  });
  const bookmarkPanel = createMapBookmarkPanelController({
    shell,
    getSignals: signals,
    listenToSignalPatches: false,
  });
  const hoverTooltip = createMapHoverTooltipController({
    shell,
    getSignals: signals,
    listenToSignalPatches: false,
  });
  const zoneInfoPanel = createMapInfoPanelController({
    shell,
    getSignals: signals,
    listenToSignalPatches: false,
  });
  const layerPanel = createMapLayerPanelController({
    shell,
    getSignals: signals,
    listenToSignalPatches: false,
  });
  const searchPanel = createMapSearchPanelController({
    shell,
    getSignals: signals,
    listenToSignalPatches: false,
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
      dispatchShellSignalPatch(shell, patch);
    } finally {
      applyingInternalSignalPatch = false;
    }
  }

  function patchSignalsFromBridge(snapshot) {
    syncingFromBridge = true;
    try {
      dispatchShellSignalPatch(
        shell,
        combineSignalPatches(
          app.projectRuntimeSnapshot(snapshot),
          app.projectSessionSnapshot(snapshot),
        ),
      );
    } finally {
      syncingFromBridge = false;
    }
    scheduleShellControllers();
  }

  function scheduleShellControllers() {
    windowManager.scheduleApplyFromSignals();
    bookmarkPanel.scheduleRender();
    hoverTooltip.scheduleRender();
    zoneInfoPanel.scheduleRender();
    layerPanel.scheduleRender();
    searchPanel.scheduleRender();
  }

  function scheduleControllersForPatch(patch) {
    if (!patch || typeof patch !== "object") {
      return;
    }
    if (patch._map_ui?.windowUi) {
      windowManager.scheduleApplyFromSignals();
    }
    if (patchTouchesBookmarkSignals(patch)) {
      bookmarkPanel.scheduleRender();
    }
    if (patchTouchesHoverTooltipSignals(patch)) {
      hoverTooltip.scheduleRender();
    }
    if (patchTouchesInfoSignals(patch)) {
      if (patch._map_runtime?.selection != null) {
        void zoneInfoPanel.refreshZoneLootSummary();
      }
      zoneInfoPanel.scheduleRender();
    }
    if (patchTouchesLayerPanelSignals(patch)) {
      layerPanel.scheduleRender();
    }
    if (patchTouchesSearchPanelSignals(patch)) {
      searchPanel.scheduleRender();
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

  shell.addEventListener(FISHYMAP_DATASTAR_SIGNAL_PATCH_EVENT, (event) => {
    const patch = event?.detail || null;
    if (applyingInternalSignalPatch) {
      scheduleControllersForPatch(patch);
      return;
    }
    if (!patchTouchesLiveBridgeInputs(patch)) {
      scheduleControllersForPatch(patch);
      return;
    }

    const nextActionState = signals()?._map_actions || {};
    const resetUiToken = Number(nextActionState.resetUiToken || 0);
    const previousResetUiToken = Number(actionState.resetUiToken || 0);
    if (resetUiToken > previousResetUiToken) {
      applyInternalSignalPatch(buildResetUiPatch());
      scheduleShellControllers();
    }

    patchBridgeFromSignals();
    if (!syncingFromBridge && patch?._map_bookmarks?.entries != null) {
      scheduleBookmarkDetailsRefresh();
    }
    scheduleControllersForPatch(patch);
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
