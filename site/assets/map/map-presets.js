import { mapText, siteText } from "./map-i18n.js";
import {
  createMapPresetPayload,
  defaultMapPresetPayload,
  mapPresetRestorePatch,
  normalizeMapPresetPayload,
} from "./map-page-state.js";
import { patchMatchesSignalFilter } from "./map-page-signals.js";

export const MAP_PRESET_COLLECTION_KEY = "map-presets";
export const MAP_PRESET_SIGNAL_FILTER =
  /^_(?:map_ui\.(?:windowUi|layers(?:\.|$)|search\.(?:query|selectedTerms|expression))|map_bridged\.ui\.(?:diagnosticsOpen|showPoints|showPointIcons|viewMode|pointIconScale)|map_bridged\.filters\.(?:layerIdsVisible|layerIdsOrdered|layerFilterBindingIdsDisabledByLayer|layerOpacities|layerClipMasks|layerWaypointConnectionsVisible|layerWaypointLabelsVisible|layerPointIconsVisible|layerPointIconScales))(?:\.|$)/;
const MAP_PRESET_TRACK_DELAY_MS = 160;

const boundShells = new WeakSet();

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function trimString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function stableJson(value) {
  return JSON.stringify(value ?? null);
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function cameraIsEmpty(camera) {
  return !isPlainObject(camera) || Object.keys(camera).length === 0;
}

function comparableMapPresetPayload(payload) {
  const normalized = normalizeMapPresetPayload(payload);
  return {
    ...normalized,
    view: {
      ...normalized.view,
      camera: isPlainObject(normalized.view?.camera) ? normalized.view.camera : {},
    },
  };
}

function mapPresetPayloadsEqual(left, right) {
  const leftPayload = comparableMapPresetPayload(left);
  const rightPayload = comparableMapPresetPayload(right);
  if (
    leftPayload.view?.viewMode === rightPayload.view?.viewMode
    && (cameraIsEmpty(leftPayload.view?.camera) || cameraIsEmpty(rightPayload.view?.camera))
  ) {
    leftPayload.view = { ...leftPayload.view, camera: {} };
    rightPayload.view = { ...rightPayload.view, camera: {} };
  }
  return stableJson(leftPayload) === stableJson(rightPayload);
}

function viewFromBridgeState(readBridgeState) {
  if (typeof readBridgeState !== "function") {
    return null;
  }
  try {
    const snapshot = readBridgeState();
    return isPlainObject(snapshot?.view) ? cloneJson(snapshot.view) : null;
  } catch (_error) {
    return null;
  }
}

function mapPresetCapture(readSignals, readBridgeState, options = {}) {
  const signals = readSignals();
  if (!signals || typeof signals !== "object") {
    return null;
  }
  const intent = trimString(options?.intent).toLowerCase();
  const includeCamera =
    options?.includeCamera === true ||
    intent === "save" ||
    intent === "clone";
  if (!includeCamera) {
    return createMapPresetPayload(signals, { includeCamera: false });
  }
  const bridgeView = viewFromBridgeState(readBridgeState);
  const nextSignals = bridgeView
    ? {
        ...signals,
        _map_session: {
          ...(isPlainObject(signals._map_session) ? cloneJson(signals._map_session) : {}),
          view: bridgeView,
        },
      }
    : signals;
  return createMapPresetPayload(nextSignals, { includeCamera: true });
}

function sharedUserPresets() {
  const helper = globalThis.window?.__fishystuffUserPresets;
  return helper
    && typeof helper.registerCollectionAdapter === "function"
    && typeof helper.trackCurrentPayload === "function"
    ? helper
    : null;
}

function toastHelper() {
  return globalThis.window?.__fishystuffToast ?? globalThis.__fishystuffToast ?? null;
}

function presetPreviewHelper() {
  return globalThis.window?.__fishystuffPresetPreviews ?? null;
}

function presetPreviewTitleIconAlias(payload) {
  return presetPreviewHelper()?.titleIconAlias?.(MAP_PRESET_COLLECTION_KEY, { payload }) || "";
}

function renderSharedMapPresetPreview(container, context = {}) {
  presetPreviewHelper()?.render?.(container, {
    ...context,
    collectionKey: MAP_PRESET_COLLECTION_KEY,
  });
}

export function registerMapPresetAdapter({
  readSignals = () => null,
  applyPatch = () => {},
  readBridgeState = () => null,
} = {}) {
  const helper = sharedUserPresets();
  if (!helper) {
    return null;
  }
  return helper.registerCollectionAdapter(MAP_PRESET_COLLECTION_KEY, {
    titleKey: "map.presets.title",
    titleFallback: "Map presets",
    openLabelKey: "map.presets.open",
    openLabelFallback: "Map Presets",
    fileBaseName: "fishystuff-map-presets",
    captureOnClone: true,
    captureOnSave: true,
    defaultPresetName(index) {
      return mapText("presets.default_name", { index: String(index) });
    },
    fixedPresets() {
      return [{
        id: "default",
        name: mapText("presets.default"),
        payload: defaultMapPresetPayload(),
      }];
    },
    normalizePayload: normalizeMapPresetPayload,
    payloadsEqual: mapPresetPayloadsEqual,
    titleIconAlias({ payload }) {
      return presetPreviewTitleIconAlias(payload);
    },
    renderPreview(container, context) {
      renderSharedMapPresetPreview(container, context);
    },
    capture(options = {}) {
      return mapPresetCapture(readSignals, readBridgeState, options);
    },
    apply(payload) {
      const patch = mapPresetRestorePatch(payload);
      applyPatch(patch);
      const signals = readSignals();
      return signals && typeof signals === "object"
        ? createMapPresetPayload(signals, { includeCamera: false })
        : normalizeMapPresetPayload(payload);
    },
  });
}

export function mapPresetCollectionActionSnapshot(
  userPresetsSnapshot,
  collectionKey = MAP_PRESET_COLLECTION_KEY,
) {
  const key = trimString(collectionKey);
  if (!key) {
    return null;
  }
  const collections = userPresetsSnapshot?.collections;
  return collections && typeof collections === "object" ? collections[key] || null : null;
}

export function mapPresetCollectionCanSave(
  userPresetsSnapshot,
  collectionKey = MAP_PRESET_COLLECTION_KEY,
) {
  return Boolean(mapPresetCollectionActionSnapshot(userPresetsSnapshot, collectionKey)?.canSave);
}

export function mapPresetCollectionCanDiscard(
  userPresetsSnapshot,
  collectionKey = MAP_PRESET_COLLECTION_KEY,
) {
  return Boolean(mapPresetCollectionActionSnapshot(userPresetsSnapshot, collectionKey)?.canDiscard);
}

export function saveMapPresetCurrent() {
  const helper = sharedUserPresets();
  if (!helper || typeof helper.saveCurrent !== "function") {
    return null;
  }
  const actionState = helper.currentActionState(MAP_PRESET_COLLECTION_KEY, {
    refresh: true,
    patchDatastar: false,
  });
  if (!actionState.canSave) {
    return null;
  }
  const result = helper.saveCurrent(MAP_PRESET_COLLECTION_KEY);
  helper.refreshDatastar?.();
  return result;
}

export function discardMapPresetCurrent() {
  const helper = sharedUserPresets();
  if (!helper || typeof helper.discardCurrent !== "function") {
    return null;
  }
  const actionState = helper.currentActionState(MAP_PRESET_COLLECTION_KEY, {
    refresh: true,
    patchDatastar: false,
  });
  if (!actionState.canDiscard) {
    return null;
  }
  const result = helper.discardCurrent(MAP_PRESET_COLLECTION_KEY, {
    refreshCurrent: false,
  });
  helper.refreshDatastar?.();
  return result?.current ? null : result;
}

export function showMapPresetSaveToast(result) {
  const savedPreset = result?.preset;
  if (!savedPreset) {
    return null;
  }
  const key = result.action === "created" ? "presets.toast.created" : "presets.toast.saved";
  return toastHelper()?.success?.(siteText(key, { name: savedPreset.name || "" })) || null;
}

export function showMapPresetDiscardToast(result) {
  if (!result) {
    return null;
  }
  return toastHelper()?.info?.(siteText("presets.toast.discarded")) || null;
}

export function showMapPresetActionError(_error, fallbackKey) {
  return toastHelper()?.error?.(siteText(fallbackKey)) || null;
}

export function installMapPresetGlobals(globalRef = globalThis) {
  const target = globalRef?.window ?? globalRef;
  if (!target || typeof target !== "object") {
    return null;
  }
  target.FishyMapPresets = {
    ...(target.FishyMapPresets && typeof target.FishyMapPresets === "object"
      ? target.FishyMapPresets
      : {}),
    presetCollectionCanSave: mapPresetCollectionCanSave,
    presetCollectionCanDiscard: mapPresetCollectionCanDiscard,
    saveCurrent: saveMapPresetCurrent,
    discardCurrent: discardMapPresetCurrent,
  };
  return target.FishyMapPresets;
}

export function applyStoredMapPresetState({
  readSignals = () => null,
  applyPatch = () => {},
} = {}) {
  const helper = sharedUserPresets();
  const storedCollection = helper?.snapshot?.()?.collections?.[MAP_PRESET_COLLECTION_KEY];
  if (!storedCollection?.activeWorkingCopyId) {
    return null;
  }
  const activeWorkingCopy = helper?.activeWorkingCopy?.(MAP_PRESET_COLLECTION_KEY);
  if (!activeWorkingCopy?.payload) {
    return null;
  }
  applyPatch(mapPresetRestorePatch(activeWorkingCopy.payload));
  const signals = readSignals();
  return signals && typeof signals === "object" ? activeWorkingCopy : null;
}

export function trackMapPresetCurrent(readSignals = () => null) {
  const helper = sharedUserPresets();
  const signals = readSignals();
  if (!helper || !signals || typeof signals !== "object") {
    return null;
  }
  return helper.trackCurrentPayload(MAP_PRESET_COLLECTION_KEY, {
    payload: createMapPresetPayload(signals, { includeCamera: false }),
  });
}

function createMapPresetTrackScheduler({
  readSignals = () => null,
  globalRef = globalThis,
  delayMs = MAP_PRESET_TRACK_DELAY_MS,
} = {}) {
  const state = {
    timer: 0,
  };

  function clear() {
    if (!state.timer) {
      return false;
    }
    globalRef.clearTimeout?.(state.timer);
    state.timer = 0;
    return true;
  }

  function flush() {
    clear();
    return trackMapPresetCurrent(readSignals);
  }

  function schedule() {
    clear();
    const timeoutDelayMs = Math.max(0, Number.isFinite(delayMs) ? delayMs : MAP_PRESET_TRACK_DELAY_MS);
    state.timer = globalRef.setTimeout?.(() => {
      state.timer = 0;
      trackMapPresetCurrent(readSignals);
    }, timeoutDelayMs) || 0;
    if (!state.timer) {
      flush();
    }
    return Boolean(state.timer);
  }

  return Object.freeze({
    clear,
    flush,
    schedule,
  });
}

export function patchTouchesMapPreset(patch) {
  return patchMatchesSignalFilter(patch, { include: MAP_PRESET_SIGNAL_FILTER });
}

export function bindMapPresetController({
  shell = null,
  readSignals = () => null,
  applyPatch = () => {},
  readBridgeState = () => null,
  globalRef = globalThis,
  trackDelayMs = MAP_PRESET_TRACK_DELAY_MS,
} = {}) {
  const adapter = registerMapPresetAdapter({ readSignals, applyPatch, readBridgeState });
  if (!adapter) {
    return null;
  }
  const tracker = createMapPresetTrackScheduler({
    readSignals,
    globalRef,
    delayMs: trackDelayMs,
  });
  const applied = applyStoredMapPresetState({ readSignals, applyPatch });
  trackMapPresetCurrent(readSignals);
  if (shell && typeof shell.addEventListener === "function" && !boundShells.has(shell)) {
    shell.addEventListener("fishymap:signal-patched", (event) => {
      if (patchTouchesMapPreset(event?.detail || null)) {
        tracker.schedule();
      }
    });
    boundShells.add(shell);
  }
  return {
    adapter,
    applied,
    flushTrackedCurrent: tracker.flush,
  };
}

installMapPresetGlobals();
