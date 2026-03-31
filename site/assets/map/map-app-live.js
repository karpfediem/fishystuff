import { DATASTAR_SIGNAL_PATCH_EVENT } from "../js/datastar-signals.js";
import { createMapApp } from "./map-app.js";
import {
  createFishyMapBridge,
  createEmptySnapshot,
  snapshotToRestorePatch,
} from "./map-host.js";
import {
  DEFAULT_MAP_ACTION_SIGNAL_STATE,
  DEFAULT_MAP_BOOKMARKS_SIGNAL_STATE,
  DEFAULT_MAP_BRIDGED_SIGNAL_STATE,
  DEFAULT_MAP_SESSION_SIGNAL_STATE,
  DEFAULT_MAP_UI_SIGNAL_STATE,
} from "./map-signal-contract.js";
import { parseQuerySignalPatch } from "./map-query-state.js";
import { createMapLayerPanelController } from "./map-layer-panel-live.js";
import { combineSignalPatches, dispatchShellSignalPatch } from "./map-signal-patch.js";
import { createMapWindowManager } from "./map-window-manager.js";

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

async function start() {
  const page = window.__fishystuffMap;
  if (!page || typeof page.whenRestored !== "function") {
    return;
  }

  await page.whenRestored();

  const shell = document.getElementById("map-page-shell");
  const canvas = document.getElementById("bevy");
  if (!(shell instanceof HTMLElement) || !(canvas instanceof HTMLCanvasElement)) {
    return;
  }

  const queryPatch = parseQuerySignalPatch(globalThis.location?.href);
  if (queryPatch) {
    dispatchShellSignalPatch(shell, queryPatch);
  }

  const app = createMapApp();
  const bridge = createFishyMapBridge();
  const windowManager = createMapWindowManager({
    shell,
    getSignals: signals,
  });
  const layerPanel = createMapLayerPanelController({
    shell,
    getSignals: signals,
  });
  let syncingFromBridge = false;
  let mounted = false;
  let lastBridgePatchJson = "";
  let actionState = app.readLastActionState();

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
    actionState = app.consumeSignals(signals());
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

  document.addEventListener(DATASTAR_SIGNAL_PATCH_EVENT, (event) => {
    const patch = event?.detail || null;
    if (!patchTouchesLiveBridgeInputs(patch)) {
      if (patch?._map_ui?.windowUi) {
        windowManager.scheduleApplyFromSignals();
      }
      return;
    }

    const nextActionState = signals()?._map_actions || {};
    const resetUiToken = Number(nextActionState.resetUiToken || 0);
    const previousResetUiToken = Number(actionState.resetUiToken || 0);
    if (resetUiToken > previousResetUiToken) {
      syncingFromBridge = true;
      try {
        dispatchShellSignalPatch(shell, buildResetUiPatch());
      } finally {
        syncingFromBridge = false;
      }
    }

    patchBridgeFromSignals();
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
  layerPanel.render();
}

function startWhenDomReady() {
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

if (globalThis.__fishystuffMapAppAutoStart !== false) {
  startWhenDomReady();
}
