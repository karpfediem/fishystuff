import { mapText } from "./map-i18n.js";
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

function renderMapPresetPreview(container, context = {}) {
  if (!(container instanceof HTMLElement)) {
    return;
  }
  const payload = normalizeMapPresetPayload(context.payload);
  container.replaceChildren();
  const root = document.createElement("div");
  root.className = "fishy-preset-manager__summary-preview";
  const visibleLayers = Array.isArray(payload.bridgedFilters?.layerIdsVisible)
    ? payload.bridgedFilters.layerIdsVisible.length
    : 0;
  const query = trimString(payload.search?.query);
  const viewMode = payload.bridgedUi?.viewMode === "3d" ? "3D" : "2D";
  const rows = [
    [viewMode, `${visibleLayers} layers`],
    [payload.bridgedUi?.showPoints === false ? "Points off" : "Points on"],
    [query || "No search"],
  ];
  for (const row of rows) {
    const rowElement = document.createElement("div");
    rowElement.className = "fishy-preset-manager__summary-preview-row";
    for (const part of row) {
      const chip = document.createElement("span");
      chip.className = "fishy-preset-manager__summary-preview-chip";
      chip.textContent = part;
      rowElement.append(chip);
    }
    root.append(rowElement);
  }
  container.append(root);
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
      return normalizeMapPresetPayload(payload).bridgedUi.viewMode === "3d" ? "cube-view" : "map-view";
    },
    renderPreview(container, context) {
      renderMapPresetPreview(container, context);
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

export function applyStoredMapPresetState({
  readSignals = () => null,
  applyPatch = () => {},
} = {}) {
  const helper = sharedUserPresets();
  const currentPreset = helper?.current?.(MAP_PRESET_COLLECTION_KEY);
  if (currentPreset?.payload) {
    applyPatch(mapPresetRestorePatch(currentPreset.payload));
    return currentPreset;
  }
  const selectedPreset = helper?.selectedPreset?.(MAP_PRESET_COLLECTION_KEY);
  if (selectedPreset?.payload) {
    applyPatch(mapPresetRestorePatch(selectedPreset.payload));
    const signals = readSignals();
    return signals && typeof signals === "object" ? selectedPreset : null;
  }
  const selectedFixedId = helper?.selectedFixedId?.(MAP_PRESET_COLLECTION_KEY);
  const selectedFixedPreset = selectedFixedId
    ? helper?.fixedPreset?.(MAP_PRESET_COLLECTION_KEY, selectedFixedId)
    : null;
  if (!selectedFixedPreset?.payload) {
    return null;
  }
  applyPatch(mapPresetRestorePatch(selectedFixedPreset.payload));
  const signals = readSignals();
  return signals && typeof signals === "object" ? selectedFixedPreset : null;
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

export function patchTouchesMapPreset(patch) {
  return patchMatchesSignalFilter(patch, { include: MAP_PRESET_SIGNAL_FILTER });
}

export function bindMapPresetController({
  shell = null,
  readSignals = () => null,
  applyPatch = () => {},
  readBridgeState = () => null,
} = {}) {
  const adapter = registerMapPresetAdapter({ readSignals, applyPatch, readBridgeState });
  if (!adapter) {
    return null;
  }
  const applied = applyStoredMapPresetState({ readSignals, applyPatch });
  trackMapPresetCurrent(readSignals);
  if (shell && typeof shell.addEventListener === "function" && !boundShells.has(shell)) {
    shell.addEventListener("fishymap:signal-patched", (event) => {
      if (patchTouchesMapPreset(event?.detail || null)) {
        trackMapPresetCurrent(readSignals);
      }
    });
    boundShells.add(shell);
  }
  return {
    adapter,
    applied,
  };
}
