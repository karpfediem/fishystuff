import { createMapApp } from "./map-app.js";
import FishyMapBridge, { createEmptySnapshot, snapshotToRestorePatch } from "./map-host.js";
import { createMapPageDerivedController } from "./map-page-derived.js";
import { createMapPageLive } from "./map-page-live.js";
import { createMapLifecycleMetrics, createMapOtelMetricsReporter } from "./map-otel-metrics.js";
import { createMapPagePersistController } from "./map-page-persist.js";
import {
  bindMapPresetController,
  discardMapPresetCurrent,
  saveMapPresetCurrent,
  showMapPresetActionError,
  showMapPresetDiscardToast,
  showMapPresetSaveToast,
} from "./map-presets.js";
import { languageReady, mapText } from "./map-i18n.js";
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

function replaceSnapshotBranch(baseValue, patchValue) {
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
    selection: replaceSnapshotBranch(baseSnapshot.selection, patchSnapshot.selection),
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

export function createBridgeInputPatchCoordinator({ patchBridgeFromSignals = () => {} } = {}) {
  let syncingFromBridge = false;
  let pendingBridgePatchAfterSync = false;

  function requestBridgePatch() {
    if (syncingFromBridge) {
      pendingBridgePatchAfterSync = true;
      return false;
    }
    patchBridgeFromSignals();
    return true;
  }

  function runBridgeSync(callback) {
    syncingFromBridge = true;
    try {
      return callback();
    } finally {
      syncingFromBridge = false;
      if (pendingBridgePatchAfterSync) {
        pendingBridgePatchAfterSync = false;
        patchBridgeFromSignals();
      }
    }
  }

  return Object.freeze({
    isSyncingFromBridge: () => syncingFromBridge,
    requestBridgePatch,
    runBridgeSync,
  });
}

function sharedUserPresets() {
  return globalThis.window?.__fishystuffUserPresets ?? globalThis.__fishystuffUserPresets ?? null;
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

const RUNTIME_STATUS_FIELDS = Object.freeze([
  ["metaStatus", "Meta"],
  ["layersStatus", "Layers"],
  ["zonesStatus", "Zones"],
  ["pointsStatus", "Points"],
  ["fishStatus", "Fish"],
  ["zoneStatsStatus", "Zone stats"],
]);

const API_STATUS_FIELDS = Object.freeze(["metaStatus", "zonesStatus", "pointsStatus", "fishStatus"]);
const API_FAILURE_STATUS_PATTERN =
  /\b(?:request failed|retrying|request closed|failed to fetch|connection refused|http\s+[45]\d{2})\b/i;
const API_STATUS_DETAIL_PATTERN = /\bapi="([^"]*)"/i;

function trimStatus(value) {
  return String(value ?? "").trim();
}

export function runtimeStatusLines(statuses) {
  if (!isPlainObject(statuses)) {
    return [];
  }
  return RUNTIME_STATUS_FIELDS.flatMap(([key, label]) => {
    const status = trimStatus(statuses[key]);
    return status ? [{ key, label, status }] : [];
  });
}

const MAP_LOADING_STAGE_TEXT_KEYS = Object.freeze({
  starting: "loading.starting",
  manifest: "loading.manifest",
  "module-import": "loading.module_import",
  "wasm-fetch": "loading.wasm_fetch",
  "wasm-compile": "loading.wasm_compile",
  "renderer-start": "loading.renderer_start",
  "runtime-bootstrap": "loading.runtime_bootstrap",
  "first-paint": "loading.first_paint",
  ready: "loading.ready",
  error: "error.renderer_unavailable",
});

const ACTIVE_RUNTIME_STATUS_PATTERN =
  /\b(?:pending|waiting|loading|retrying|request failed|request closed|failed to fetch|http\s+[45]\d{2})\b/i;

export function formatMapLoadingBytes(bytes) {
  const value = Number(bytes);
  if (!Number.isFinite(value) || value <= 0) {
    return "";
  }
  const units = ["B", "KB", "MB", "GB"];
  let scaled = value;
  let unitIndex = 0;
  while (scaled >= 1024 && unitIndex < units.length - 1) {
    scaled /= 1024;
    unitIndex += 1;
  }
  const precision = unitIndex === 0 || scaled >= 10 ? 0 : 1;
  return `${scaled.toFixed(precision)} ${units[unitIndex]}`;
}

function loadingStageText(stage) {
  return mapText(MAP_LOADING_STAGE_TEXT_KEYS[stage] || MAP_LOADING_STAGE_TEXT_KEYS.starting);
}

function runtimeStatusLoadingDetail(statuses) {
  const statusLines = runtimeStatusLines(statuses);
  const activeLine =
    statusLines.find((line) => ACTIVE_RUNTIME_STATUS_PATTERN.test(line.status)) ||
    statusLines.find((line) => line.status);
  return activeLine ? `${activeLine.label}: ${activeLine.status}` : "";
}

function loadingDetailText(detail) {
  const loadedBytes = Number(detail?.loadedBytes);
  const totalBytes = Number(detail?.totalBytes);
  if (Number.isFinite(loadedBytes) && loadedBytes > 0) {
    const loaded = formatMapLoadingBytes(loadedBytes);
    const total = formatMapLoadingBytes(totalBytes);
    if (loaded && total) {
      return mapText("loading.fetch_bytes", { loaded, total });
    }
    if (loaded) {
      return mapText("loading.fetch_bytes_unknown", { loaded });
    }
  }
  return runtimeStatusLoadingDetail(detail?.statuses) || mapText("loading.initial_detail");
}

export function createMapLoadingOverlayController(shell) {
  const overlay = shell?.querySelector?.("#fishymap-loading-overlay") || null;
  if (!overlay) {
    return Object.freeze({
      update() {},
      updateFromSnapshot() {},
      finishAfterFirstPaint() {},
      fail() {},
    });
  }

  const stageElement = overlay.querySelector("#fishymap-loading-stage");
  const detailElement = overlay.querySelector("#fishymap-loading-detail");
  const progressElement = overlay.querySelector("#fishymap-loading-progress");
  const percentElement = overlay.querySelector("#fishymap-loading-percent");
  let hidden = false;
  let finished = false;
  let lastProgress = 0;

  function setProgress(progress) {
    const normalized = Number(progress);
    if (!Number.isFinite(normalized)) {
      progressElement?.removeAttribute?.("value");
      if (percentElement) {
        percentElement.hidden = true;
        percentElement.textContent = "";
      }
      return;
    }
    lastProgress = Math.max(lastProgress, Math.min(1, Math.max(0, normalized)));
    const percent = Math.round(lastProgress * 100);
    progressElement?.setAttribute?.("value", String(percent));
    progressElement?.setAttribute?.("max", "100");
    if (percentElement) {
      percentElement.hidden = false;
      percentElement.textContent = `${percent}%`;
    }
  }

  function update(detail = {}) {
    if (hidden) {
      return;
    }
    shell.dataset.mapLoading = "loading";
    overlay.hidden = false;
    overlay.classList.remove("is-hidden");
    const stage = String(detail?.stage || "starting");
    overlay.dataset.stage = stage;
    if (stageElement) {
      stageElement.textContent = loadingStageText(stage);
    }
    if (detailElement) {
      detailElement.textContent = loadingDetailText(detail);
    }
    setProgress(detail?.progress);
  }

  function updateFromSnapshot(snapshot) {
    if (hidden || finished) {
      return;
    }
    if (snapshot?.ready === true) {
      update({
        stage: "first-paint",
        progress: 0.98,
        statuses: snapshot?.statuses,
      });
      return;
    }
    update({
      stage: "runtime-bootstrap",
      progress: 0.96,
      statuses: snapshot?.statuses,
    });
  }

  function hide() {
    hidden = true;
    overlay.classList.add("is-hidden");
    shell.dataset.mapLoading = "hidden";
    globalThis.setTimeout?.(() => {
      overlay.hidden = true;
    }, 220);
  }

  function finishAfterFirstPaint() {
    if (hidden || finished) {
      return;
    }
    finished = true;
    update({
      stage: "first-paint",
      progress: 0.98,
    });
    deferAfterAnimationFrames(
      () => {
        update({
          stage: "ready",
          progress: 1,
        });
        globalThis.setTimeout?.(hide, 120);
      },
      { frames: 2 },
    );
  }

  function fail(error) {
    hidden = false;
    finished = true;
    shell.dataset.mapLoading = "error";
    overlay.hidden = false;
    overlay.classList.remove("is-hidden");
    overlay.dataset.stage = "error";
    progressElement?.classList?.remove("progress-primary");
    progressElement?.classList?.add("progress-error");
    progressElement?.removeAttribute?.("value");
    if (percentElement) {
      percentElement.hidden = true;
      percentElement.textContent = "";
    }
    if (stageElement) {
      stageElement.textContent = loadingStageText("error");
    }
    if (detailElement) {
      detailElement.textContent =
        error && typeof error === "object" && "message" in error
          ? String(error.message)
          : mapText("error.renderer_start_failed");
    }
  }

  update({
    stage: "starting",
    progress: null,
  });

  return Object.freeze({
    update,
    updateFromSnapshot,
    finishAfterFirstPaint,
    fail,
  });
}

export function apiFailureStatusLines(statuses) {
  if (!isPlainObject(statuses)) {
    return [];
  }
  return runtimeStatusLines(statuses).flatMap((line) => {
    if (!API_STATUS_FIELDS.includes(line.key)) {
      return [];
    }
    const attrMatch = line.status.match(API_STATUS_DETAIL_PATTERN);
    const rawStatus = trimStatus(attrMatch ? attrMatch[1] : line.status);
    const status = rawStatus.replace(/^[a-z][a-z0-9 _-]*:\s*/i, "");
    if (!API_FAILURE_STATUS_PATTERN.test(status)) {
      return [];
    }
    return [{ ...line, status }];
  });
}

function syncApiHealthIssues(apiFailures) {
  const health = globalThis.window?.__fishystuffPageHealth ?? globalThis.__fishystuffPageHealth;
  if (!health || typeof health.syncSourceIssues !== "function") {
    return;
  }
  health.syncSourceIssues("map-api", apiFailures.map((line) => ({
    id: `map-api:${line.key}`,
    severity: "warning",
    source: "map-api",
    category: "api",
    title: `${line.label} API request failed`,
    detail: line.status,
    context: {
      statusKey: line.key,
      status: line.status,
    },
  })));
}

export function renderRuntimeStatusSurface(root, statuses) {
  if (!root || typeof root.querySelector !== "function") {
    return;
  }

  const statusLines = runtimeStatusLines(statuses);
  const statusLinesElement = root.querySelector("#fishymap-status-lines");
  if (statusLinesElement) {
    const ownerDocument = root.ownerDocument || document;
    statusLinesElement.replaceChildren(
      ...statusLines.map((line) => {
        const row = ownerDocument.createElement("div");
        row.className = "flex min-w-0 items-start gap-2";

        const label = ownerDocument.createElement("span");
        label.className = "shrink-0 font-semibold text-base-content/55";
        label.textContent = `${line.label}:`;
        row.appendChild(label);

        const status = ownerDocument.createElement("span");
        status.className = "min-w-0 break-words";
        status.textContent = line.status;
        row.appendChild(status);

        return row;
      }),
    );
  }

  const apiFailures = apiFailureStatusLines(statuses);
  syncApiHealthIssues(apiFailures);
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
  const loadingOverlay = createMapLoadingOverlayController(shell);

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
  sharedUserPresets()?.bindDatastar?.(signals());
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
  let applyingInternalSignalPatch = false;
  let mounted = false;
  let lastBridgePatchJson = "";
  let pendingBridgeRestoreView = null;
  let actionState = app.readLastActionState();
  let consumingMapPresetAction = false;
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
    loadingOverlay.finishAfterFirstPaint();
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

  const bridgeInputPatchCoordinator = createBridgeInputPatchCoordinator({
    patchBridgeFromSignals,
  });

  function patchBridgeFromSignals() {
    if (!mounted || bridgeInputPatchCoordinator.isSyncingFromBridge()) {
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
    bridgeInputPatchCoordinator.runBridgeSync(() => {
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
    });
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

  function consumeMapPresetActionTokens(nextActionState, previousActionState) {
    if (consumingMapPresetAction) {
      return;
    }
    const saveToken = Number(nextActionState.saveMapPresetToken || 0);
    const previousSaveToken = Number(previousActionState.saveMapPresetToken || 0);
    const discardToken = Number(nextActionState.discardMapPresetToken || 0);
    const previousDiscardToken = Number(previousActionState.discardMapPresetToken || 0);
    if (saveToken <= previousSaveToken && discardToken <= previousDiscardToken) {
      return;
    }

    actionState = {
      ...actionState,
      saveMapPresetToken: Math.max(previousSaveToken, saveToken),
      discardMapPresetToken: Math.max(previousDiscardToken, discardToken),
    };
    consumingMapPresetAction = true;
    try {
      if (saveToken > previousSaveToken) {
        try {
          showMapPresetSaveToast(saveMapPresetCurrent());
        } catch (error) {
          showMapPresetActionError(error, "presets.error.save");
        }
      }

      if (discardToken > previousDiscardToken) {
        try {
          showMapPresetDiscardToast(discardMapPresetCurrent());
        } catch (error) {
          showMapPresetActionError(error, "presets.error.discard");
        }
      }
    } finally {
      consumingMapPresetAction = false;
    }
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
    loadingOverlay.updateFromSnapshot(snapshot);
    if (event?.type === "fishymap:ready" || snapshot?.ready === true) {
      recordRuntimeReady();
    }
    renderRuntimeStatusSurface(shell, snapshot?.statuses);
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
    const restoredBridgeView = bridgeInputPatchCoordinator.isSyncingFromBridge()
      ? false
      : restoreBridgeViewFromSignalPatch(patch);
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
    consumeMapPresetActionTokens(nextActionState, actionState);
    if (resetUiToken > previousResetUiToken) {
      applyInternalSignalPatch(buildResetUiPatch());
    }

    if (touchesLiveBridgeInputs) {
      bridgeInputPatchCoordinator.requestBridgePatch();
    }
    if (
      !bridgeInputPatchCoordinator.isSyncingFromBridge() &&
      patch?._map_bookmarks?.entries != null
    ) {
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
    onLoadProgress: (detail) => loadingOverlay.update(detail),
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
  loadingOverlay.updateFromSnapshot(currentBridgeState());
  renderRuntimeStatusSurface(
    shell,
    currentBridgeState()?.statuses || signals()?._map_runtime?.statuses,
  );
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
      createMapLoadingOverlayController(document.getElementById("map-page-shell"))?.fail?.(error);
    });
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", run, { once: true });
    return;
  }

  run();
}
