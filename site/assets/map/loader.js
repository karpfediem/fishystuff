import FishyMapBridge, {
  FISHYMAP_EVENTS,
  FISHYMAP_POINT_ICON_SCALE_MAX,
  FISHYMAP_POINT_ICON_SCALE_MIN,
  resolveCdnBaseUrl,
} from "./map-host.js";

const FIXED_GROUND_LAYER_IDS = new Set(["minimap"]);
const DEFAULT_ZONE_CATALOG_URL = new URL("../data/zones.json", import.meta.url).toString();
const WINDOW_DRAG_THRESHOLD_PX = 8;
const WINDOW_TITLEBAR_FALLBACK_HEIGHT_PX = 52;
const DEFAULT_WINDOW_UI_STATE = Object.freeze({
  search: Object.freeze({ open: true, collapsed: false, x: null, y: null }),
  settings: Object.freeze({ open: true, collapsed: false, x: null, y: null }),
  zoneInfo: Object.freeze({ open: true, collapsed: false, x: null, y: null }),
  layers: Object.freeze({ open: true, collapsed: false, x: null, y: null }),
});

function dispatchMapEvent(target, type, detail) {
  target.dispatchEvent(new CustomEvent(type, { detail }));
}

function dispatchMapState(target, patch) {
  dispatchMapEvent(target, FISHYMAP_EVENTS.setState, patch);
}

function dispatchMapCommand(target, command) {
  dispatchMapEvent(target, FISHYMAP_EVENTS.command, command);
}

function supportsWebgl2(doc = document) {
  const probe = doc?.createElement?.("canvas");
  if (!probe?.getContext) {
    return false;
  }
  try {
    return !!probe.getContext("webgl2");
  } catch (_) {
    return false;
  }
}

function formatLoaderError(error) {
  if (!error) {
    return "Unknown renderer error.";
  }
  if (typeof error === "string") {
    return error;
  }
  if (typeof error === "object") {
    if (typeof error.stack === "string" && error.stack.trim()) {
      return error.stack;
    }
    if (typeof error.message === "string" && error.message.trim()) {
      return error.message;
    }
    if (typeof error.reason === "object" || typeof error.reason === "string") {
      return formatLoaderError(error.reason);
    }
  }
  return String(error);
}

function shouldHandleRendererError(error, fallbackMessage = "") {
  const text = `${formatLoaderError(error)} ${fallbackMessage}`.toLowerCase();
  return (
    text.includes("fishystuff_ui_bevy") ||
    text.includes("wgpu surface") ||
    text.includes("webgl2") ||
    text.includes("renderer/mod.rs") ||
    text.includes("canvas.getcontext")
  );
}

function setMapError(elements, error) {
  const message = formatLoaderError(error);
  elements.readyPill.textContent = "Error";
  elements.readyPill.className = "badge badge-error badge-sm";
  elements.statusLines.innerHTML = "";
  const status = document.createElement("p");
  status.textContent = "The map renderer failed to start.";
  elements.statusLines.appendChild(status);
  elements.diagnosticJson.textContent = message;
  if (elements.errorMessage) {
    elements.errorMessage.textContent = message;
  }
  if (elements.errorOverlay) {
    elements.errorOverlay.hidden = false;
  }
  if (elements.canvas) {
    elements.canvas.hidden = true;
  }
}

function installRendererErrorHandlers(elements) {
  const onError = (event) => {
    if (!shouldHandleRendererError(event?.error, event?.message || event?.filename || "")) {
      return;
    }
    FishyMapBridge.destroy?.();
    setMapError(elements, event?.error || event?.message || event);
  };
  const onRejection = (event) => {
    if (!shouldHandleRendererError(event?.reason)) {
      return;
    }
    FishyMapBridge.destroy?.();
    setMapError(elements, event?.reason || event);
  };
  window.addEventListener("error", onError);
  window.addEventListener("unhandledrejection", onRejection);
}

function requestBridgeState(target) {
  const detail = {};
  dispatchMapEvent(target, FISHYMAP_EVENTS.requestState, detail);
  return {
    state: detail.state || FishyMapBridge.getCurrentState(),
    inputState:
      detail.inputState ||
      (typeof FishyMapBridge.getCurrentInputState === "function"
        ? FishyMapBridge.getCurrentInputState()
        : {}),
  };
}

function applyThemeToShell(shell) {
  if (!shell) {
    return;
  }
  const background =
    window.__fishystuffTheme?.colors?.base200 ||
    window.__fishystuffTheme?.colors?.base100 ||
    window.getComputedStyle(document.documentElement).getPropertyValue("--color-base-200") ||
    window.getComputedStyle(document.documentElement).getPropertyValue("--color-base-100");
  const nextBackground = String(background || "").trim();
  if (nextBackground && shell.dataset.themeBackground !== nextBackground) {
    shell.dataset.themeBackground = nextBackground;
    shell.style.backgroundColor = nextBackground;
  }
}

function setTextContent(element, text) {
  if (!element) {
    return;
  }
  const nextText = String(text ?? "");
  if (element.textContent !== nextText) {
    element.textContent = nextText;
  }
}

function setClassName(element, className) {
  if (!element) {
    return;
  }
  if (element.className !== className) {
    element.className = className;
  }
}

function setAttributeValue(element, name, value) {
  if (!element) {
    return;
  }
  const nextValue = String(value ?? "");
  if (element.getAttribute(name) !== nextValue) {
    element.setAttribute(name, nextValue);
  }
}

function setBooleanProperty(element, propertyName, value) {
  if (!element) {
    return;
  }
  const nextValue = Boolean(value);
  if (element[propertyName] !== nextValue) {
    element[propertyName] = nextValue;
  }
}

function setMarkup(element, renderKey, html) {
  if (!element) {
    return false;
  }
  const nextKey = String(renderKey ?? "");
  if (element.dataset.markupKey === nextKey) {
    return false;
  }
  element.dataset.markupKey = nextKey;
  element.innerHTML = html;
  return true;
}

function clamp(value, min, max) {
  return Math.min(Math.max(value, min), max);
}

function normalizeWindowCoordinate(value) {
  if (value == null || value === "") {
    return null;
  }
  const number = Number(value);
  return Number.isFinite(number) ? Math.round(number) : null;
}

function normalizeWindowUiEntry(rawEntry, fallbackEntry) {
  const baseEntry = isPlainObject(rawEntry) ? rawEntry : {};
  return {
    open: hasOwnKey(baseEntry, "open") ? baseEntry.open !== false : fallbackEntry.open !== false,
    collapsed: hasOwnKey(baseEntry, "collapsed")
      ? Boolean(baseEntry.collapsed)
      : Boolean(fallbackEntry.collapsed),
    x: hasOwnKey(baseEntry, "x") ? normalizeWindowCoordinate(baseEntry.x) : fallbackEntry.x,
    y: hasOwnKey(baseEntry, "y") ? normalizeWindowCoordinate(baseEntry.y) : fallbackEntry.y,
  };
}

export function normalizeWindowUiState(rawState) {
  const source = isPlainObject(rawState) ? rawState : {};
  return {
    search: {
      ...normalizeWindowUiEntry(source.search, DEFAULT_WINDOW_UI_STATE.search),
      collapsed: false,
    },
    settings: normalizeWindowUiEntry(source.settings, DEFAULT_WINDOW_UI_STATE.settings),
    zoneInfo: normalizeWindowUiEntry(source.zoneInfo, DEFAULT_WINDOW_UI_STATE.zoneInfo),
    layers: normalizeWindowUiEntry(source.layers, DEFAULT_WINDOW_UI_STATE.layers),
  };
}

export function parseWindowUiState(serializedState) {
  if (typeof serializedState !== "string" || !serializedState.trim()) {
    return normalizeWindowUiState(null);
  }
  try {
    return normalizeWindowUiState(JSON.parse(serializedState));
  } catch (_) {
    return normalizeWindowUiState(null);
  }
}

export function serializeWindowUiState(windowUiState) {
  return JSON.stringify(normalizeWindowUiState(windowUiState));
}

function buildFishLookup(catalogFish) {
  const map = new Map();
  for (const fish of catalogFish || []) {
    map.set(fish.fishId, fish);
    if (Number.isFinite(fish.itemId)) {
      map.set(fish.itemId, fish);
    }
  }
  return map;
}

const FISH_GRADE_ORDER = ["Prize", "Rare", "HighQuality", "General", "Trash"];

function normalizeFishGrade(fish) {
  if (!fish || typeof fish !== "object") {
    return "Unknown";
  }
  if (fish.isPrize === true || fish.grade === "Prize") {
    return "Prize";
  }
  const grade = String(fish.grade || "").trim();
  return FISH_GRADE_ORDER.includes(grade) ? grade : "Unknown";
}

function slugifyFishGrade(value) {
  return String(value || "Unknown").toLowerCase();
}

function fishGradeFrameClass(fish) {
  return `grade-${slugifyFishGrade(normalizeFishGrade(fish))}`;
}

function isPlainObject(value) {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

function hasOwnKey(object, key) {
  return !!object && Object.prototype.hasOwnProperty.call(object, key);
}

function rgbTripletToU32(r, g, b) {
  return ((((r & 0xff) << 16) | ((g & 0xff) << 8) | (b & 0xff)) >>> 0);
}

function parseCatalogRgbByte(value) {
  const number = Number.parseFloat(value);
  if (!Number.isFinite(number)) {
    return null;
  }
  if (number >= 0 && number <= 255 && Math.abs(number - Math.round(number)) < 1e-6) {
    return Math.round(number);
  }
  return null;
}

function formatRgbKey(r, g, b, separator = ",") {
  return `${r}${separator}${g}${separator}${b}`;
}

function formatNormalizedRgbComponent(value) {
  return (value / 255).toFixed(6);
}

export function normalizeZoneCatalog(rawCatalog) {
  const entries = Array.isArray(rawCatalog)
    ? rawCatalog
    : Array.isArray(rawCatalog?.zones)
      ? rawCatalog.zones
      : [];
  const normalized = [];
  for (const entry of entries) {
    const r = parseCatalogRgbByte(entry?.r ?? entry?.rgb?.r);
    const g = parseCatalogRgbByte(entry?.g ?? entry?.rgb?.g);
    const b = parseCatalogRgbByte(entry?.b ?? entry?.rgb?.b);
    if (![r, g, b].every(Number.isInteger)) {
      continue;
    }
    const zoneRgb = rgbTripletToU32(r, g, b);
    const rgbKey = formatRgbKey(r, g, b);
    const normalizedParts = [
      formatNormalizedRgbComponent(r),
      formatNormalizedRgbComponent(g),
      formatNormalizedRgbComponent(b),
    ];
    const hex = Number(zoneRgb).toString(16).padStart(6, "0");
    const name = String(entry?.name || "").trim() || `Zone ${rgbKey}`;
    const confirmedRaw = entry?.confirmed;
    const confirmed =
      confirmedRaw === true ||
      confirmedRaw === 1 ||
      String(confirmedRaw || "").trim() === "1" ||
      String(confirmedRaw || "").trim().toLowerCase() === "true";
    const order = Number.parseInt(entry?.order, 10);
    normalized.push({
      kind: "zone",
      zoneRgb,
      r,
      g,
      b,
      name,
      confirmed,
      order: Number.isFinite(order) ? order : Number.MAX_SAFE_INTEGER,
      rgbKey,
      rgbSpaced: formatRgbKey(r, g, b, " "),
      normalizedKey: normalizedParts.join(","),
      normalizedSpaced: normalizedParts.join(" "),
      hexKey: `0x${hex}`,
      hashHexKey: `#${hex}`,
      bareHexKey: hex,
      _nameSearch: name.toLowerCase(),
    });
  }
  return normalized;
}

async function loadZoneCatalog(fetchImpl = globalThis.fetch, url = DEFAULT_ZONE_CATALOG_URL) {
  if (typeof fetchImpl !== "function") {
    return [];
  }
  try {
    const response = await fetchImpl(url);
    if (!response?.ok) {
      throw new Error(`zone catalog request failed with status ${response?.status ?? "unknown"}`);
    }
    return normalizeZoneCatalog(await response.json());
  } catch (error) {
    console.warn("Failed to load zone search catalog", error);
    return [];
  }
}

function mergeZoneEvidenceIntoFishLookup(fishLookup, zoneStats) {
  const distribution = Array.isArray(zoneStats?.distribution) ? zoneStats.distribution : [];
  for (const entry of distribution) {
    const fishId = Number.parseInt(entry?.fishId, 10);
    if (!Number.isFinite(fishId)) {
      continue;
    }
    const existing = fishLookup.get(fishId) || {};
    fishLookup.set(fishId, {
      fishId,
      itemId: Number.isFinite(entry?.itemId) ? entry.itemId : existing.itemId,
      encyclopediaKey: Number.isFinite(entry?.encyclopediaKey)
        ? entry.encyclopediaKey
        : existing.encyclopediaKey,
      encyclopediaId: Number.isFinite(entry?.encyclopediaId)
        ? entry.encyclopediaId
        : existing.encyclopediaId,
      name: entry.fishName || existing.name || `Fish ${fishId}`,
      grade: existing.grade || null,
      isPrize: existing.isPrize || false,
    });
  }
  return fishLookup;
}

function escapeHtml(value) {
  return String(value ?? "").replace(
    /[&<>"']/g,
    (char) =>
      (
        {
          "&": "&amp;",
          "<": "&lt;",
          ">": "&gt;",
          '"': "&quot;",
          "'": "&#39;",
        }[char] || char
      ),
  );
}

function fishIconUrl(fish) {
  if (typeof globalThis.window?.__fishystuffResolveFishItemIconUrl === "function") {
    const itemUrl = globalThis.window.__fishystuffResolveFishItemIconUrl(fish?.itemId);
    if (itemUrl) {
      return itemUrl;
    }
  }
  if (typeof globalThis.window?.__fishystuffResolveFishEncyclopediaIconUrl === "function") {
    const encyclopediaUrl =
      globalThis.window.__fishystuffResolveFishEncyclopediaIconUrl(fish?.encyclopediaId);
    if (encyclopediaUrl) {
      return encyclopediaUrl;
    }
  }
  const assetPath = fishItemIconPath(fish?.itemId) || fishEncyclopediaIconPath(fish?.encyclopediaId);
  return assetPath ? `${resolveCdnBaseUrl(window.location)}${assetPath}` : "";
}

function zeroPad(value, width) {
  const numeric = Number.parseInt(value, 10);
  if (!Number.isFinite(numeric) || numeric <= 0) {
    return "";
  }
  return String(numeric).padStart(width, "0");
}

function fishItemIconPath(itemId) {
  const digits = zeroPad(itemId, 8);
  if (!digits) {
    return "";
  }
  return `/images/FishIcons/${digits}.png`;
}

function fishEncyclopediaIconPath(encyclopediaId) {
  const numeric = Number.parseInt(encyclopediaId, 10);
  if (!Number.isFinite(numeric) || numeric <= 0) {
    return "";
  }
  return `/images/FishIcons/IC_0${numeric}.png`;
}

function clampPointIconScale(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return FISHYMAP_POINT_ICON_SCALE_MIN;
  }
  return Math.min(FISHYMAP_POINT_ICON_SCALE_MAX, Math.max(FISHYMAP_POINT_ICON_SCALE_MIN, number));
}

function pointIconScaleValue(scale) {
  return String(Math.round(clampPointIconScale(scale) * 100) / 100);
}

function pointIconScaleLabel(scale) {
  return `${Math.round(clampPointIconScale(scale) * 100)}%`;
}

function clampLayerOpacity(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return 1;
  }
  return Math.min(1, Math.max(0, number));
}

function layerOpacityValue(opacity) {
  return String(Math.round(clampLayerOpacity(opacity) * 100) / 100);
}

function layerOpacityLabel(opacity) {
  return `${Math.round(clampLayerOpacity(opacity) * 100)}%`;
}

function isFixedGroundLayer(layerId) {
  return FIXED_GROUND_LAYER_IDS.has(String(layerId || "").trim());
}

function layerKindLabel(kind) {
  if (kind === "vector-geojson") {
    return "Vector";
  }
  if (kind === "tiled-raster") {
    return "Raster";
  }
  return "Layer";
}

function spriteIcon(name) {
  return `<svg class="fishy-icon size-5" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="/img/icons.svg?v=20260320-1#fishy-${name}"></use></svg>`;
}

function dragHandleIcon() {
  return spriteIcon("drag-handle");
}

function eyeIcon(visible) {
  if (visible) {
    return spriteIcon("eye");
  }
  return spriteIcon("eye-slash");
}

function mapViewIcon() {
  return spriteIcon("map-view");
}

function cubeViewIcon() {
  return spriteIcon("cube-view");
}

function resolveLayerEntries(stateBundle) {
  const layers = Array.isArray(stateBundle.state?.catalog?.layers)
    ? stateBundle.state.catalog.layers.slice()
    : [];
  const orderedIds = Array.isArray(stateBundle.inputState?.filters?.layerIdsOrdered)
    ? stateBundle.inputState.filters.layerIdsOrdered
    : Array.isArray(stateBundle.state?.filters?.layerIdsOrdered)
      ? stateBundle.state.filters.layerIdsOrdered
      : [];
  const visibleOverride = Array.isArray(stateBundle.inputState?.filters?.layerIdsVisible)
    ? new Set(stateBundle.inputState.filters.layerIdsVisible)
    : null;
  const inputOpacityOverride = isPlainObject(stateBundle.inputState?.filters?.layerOpacities)
    ? stateBundle.inputState.filters.layerOpacities
    : null;
  const stateOpacityOverride = isPlainObject(stateBundle.state?.filters?.layerOpacities)
    ? stateBundle.state.filters.layerOpacities
    : null;
  const inputClipMaskOverride = isPlainObject(stateBundle.inputState?.filters?.layerClipMasks)
    ? stateBundle.inputState.filters.layerClipMasks
    : null;
  const stateClipMaskOverride = isPlainObject(stateBundle.state?.filters?.layerClipMasks)
    ? stateBundle.state.filters.layerClipMasks
    : null;
  const byId = new Map(layers.map((layer) => [layer.layerId, layer]));
  const seen = new Set();
  const movable = [];
  const pinned = [];

  const pushLayer = (layer) => {
    if (!layer || seen.has(layer.layerId)) {
      return;
    }
    seen.add(layer.layerId);
    const visible = visibleOverride ? visibleOverride.has(layer.layerId) : Boolean(layer.visible);
    const opacityDefault = clampLayerOpacity(layer.opacityDefault ?? 1);
    let opacity = clampLayerOpacity(layer.opacity);
    if (inputOpacityOverride) {
      opacity = hasOwnKey(inputOpacityOverride, layer.layerId)
        ? clampLayerOpacity(inputOpacityOverride[layer.layerId])
        : opacityDefault;
    } else if (stateOpacityOverride && hasOwnKey(stateOpacityOverride, layer.layerId)) {
      opacity = clampLayerOpacity(stateOpacityOverride[layer.layerId]);
    }
    let clipMaskLayerId = null;
    if (inputClipMaskOverride) {
      clipMaskLayerId = hasOwnKey(inputClipMaskOverride, layer.layerId)
        ? String(inputClipMaskOverride[layer.layerId] || "").trim() || null
        : null;
    } else if (stateClipMaskOverride && hasOwnKey(stateClipMaskOverride, layer.layerId)) {
      clipMaskLayerId = String(stateClipMaskOverride[layer.layerId] || "").trim() || null;
    }
    const entry = {
      ...layer,
      visible,
      opacity,
      opacityDefault,
      clipMaskLayerId,
      locked: isFixedGroundLayer(layer.layerId),
    };
    if (entry.locked) {
      pinned.push(entry);
    } else {
      movable.push(entry);
    }
  };

  for (const layerId of orderedIds) {
    pushLayer(byId.get(layerId));
  }

  const fallback = layers.slice().sort((left, right) => {
    const leftOrder = Number.isFinite(left?.displayOrder) ? left.displayOrder : 0;
    const rightOrder = Number.isFinite(right?.displayOrder) ? right.displayOrder : 0;
    return rightOrder - leftOrder || String(left?.layerId || "").localeCompare(String(right?.layerId || ""));
  });
  for (const layer of fallback) {
    pushLayer(layer);
  }

  return movable.concat(pinned);
}

function resolveVisibleLayerIds(stateBundle) {
  return resolveLayerEntries(stateBundle)
    .filter((layer) => layer.visible)
    .map((layer) => layer.layerId);
}

function moveLayerIdBefore(entries, draggedLayerId, targetLayerId, position) {
  const movableIds = entries.filter((layer) => !layer.locked).map((layer) => layer.layerId);
  const fromIndex = movableIds.indexOf(draggedLayerId);
  const targetIndex = movableIds.indexOf(targetLayerId);
  if (fromIndex < 0 || targetIndex < 0) {
    return movableIds;
  }
  const [dragged] = movableIds.splice(fromIndex, 1);
  const insertIndex = position === "after" ? targetIndex + (fromIndex < targetIndex ? 0 : 1) : targetIndex + (fromIndex < targetIndex ? -1 : 0);
  movableIds.splice(Math.max(0, insertIndex), 0, dragged);
  const pinnedIds = entries.filter((layer) => layer.locked).map((layer) => layer.layerId);
  return movableIds.concat(pinnedIds);
}

function resolveTopClipMaskLayerId(clipMasks, layerId) {
  const normalizedLayerId = String(layerId || "").trim();
  if (!normalizedLayerId) {
    return "";
  }
  const seen = new Set([normalizedLayerId]);
  let cursor = String(clipMasks[normalizedLayerId] || "").trim();
  while (cursor) {
    if (seen.has(cursor) || cursor === normalizedLayerId || isFixedGroundLayer(cursor)) {
      return "";
    }
    seen.add(cursor);
    const nextMaskLayerId = String(clipMasks[cursor] || "").trim();
    if (!nextMaskLayerId || nextMaskLayerId === cursor || isFixedGroundLayer(nextMaskLayerId)) {
      return cursor;
    }
    cursor = nextMaskLayerId;
  }
  return "";
}

function flattenLayerClipMasks(clipMasks) {
  const flattened = {};
  for (const [layerId, maskLayerId] of Object.entries(clipMasks || {})) {
    const normalizedLayerId = String(layerId || "").trim();
    const normalizedMaskLayerId = String(maskLayerId || "").trim();
    if (
      !normalizedLayerId ||
      !normalizedMaskLayerId ||
      normalizedLayerId === normalizedMaskLayerId ||
      isFixedGroundLayer(normalizedLayerId) ||
      isFixedGroundLayer(normalizedMaskLayerId)
    ) {
      continue;
    }
    const topMaskLayerId = resolveTopClipMaskLayerId(
      { ...clipMasks, [normalizedLayerId]: normalizedMaskLayerId },
      normalizedLayerId,
    );
    if (!topMaskLayerId || topMaskLayerId === normalizedLayerId) {
      continue;
    }
    flattened[normalizedLayerId] = topMaskLayerId;
  }
  return flattened;
}

function collectAttachedLayerIds(clipMasks, rootLayerId) {
  const normalizedRootLayerId = String(rootLayerId || "").trim();
  if (!normalizedRootLayerId) {
    return new Set();
  }
  const attachedLayerIds = new Set([normalizedRootLayerId]);
  let changed = true;
  while (changed) {
    changed = false;
    for (const [layerId, maskLayerId] of Object.entries(clipMasks || {})) {
      const normalizedLayerId = String(layerId || "").trim();
      const normalizedMaskLayerId = String(maskLayerId || "").trim();
      if (
        !normalizedLayerId ||
        !normalizedMaskLayerId ||
        attachedLayerIds.has(normalizedLayerId) ||
        !attachedLayerIds.has(normalizedMaskLayerId)
      ) {
        continue;
      }
      attachedLayerIds.add(normalizedLayerId);
      changed = true;
    }
  }
  return attachedLayerIds;
}

function buildLayerOpacityPatch(stateBundle, targetLayerId, opacity) {
  const nextOpacities = {};
  for (const layer of resolveLayerEntries(stateBundle)) {
    if (layer.locked) {
      continue;
    }
    const effectiveOpacity =
      layer.layerId === targetLayerId ? clampLayerOpacity(opacity) : clampLayerOpacity(layer.opacity);
    if (Math.abs(effectiveOpacity - clampLayerOpacity(layer.opacityDefault)) <= 0.0001) {
      continue;
    }
    nextOpacities[layer.layerId] = effectiveOpacity;
  }
  return nextOpacities;
}

function buildLayerClipMaskPatch(stateBundle, targetLayerId, maskLayerId) {
  const nextClipMasks = {};
  for (const layer of resolveLayerEntries(stateBundle)) {
    if (layer.locked) {
      continue;
    }
    const currentMaskLayerId = String(layer.clipMaskLayerId || "").trim();
    if (
      !currentMaskLayerId ||
      currentMaskLayerId === layer.layerId ||
      isFixedGroundLayer(currentMaskLayerId)
    ) {
      continue;
    }
    nextClipMasks[layer.layerId] = currentMaskLayerId;
  }
  const normalizedTargetLayerId = String(targetLayerId || "").trim();
  const normalizedMaskLayerId = String(maskLayerId || "").trim();
  if (!normalizedTargetLayerId || isFixedGroundLayer(normalizedTargetLayerId)) {
    return flattenLayerClipMasks(nextClipMasks);
  }
  if (!normalizedMaskLayerId || isFixedGroundLayer(normalizedMaskLayerId)) {
    delete nextClipMasks[normalizedTargetLayerId];
    return flattenLayerClipMasks(nextClipMasks);
  }

  const draggedSubtree = collectAttachedLayerIds(nextClipMasks, normalizedTargetLayerId);
  const targetSubtree = collectAttachedLayerIds(nextClipMasks, normalizedMaskLayerId);

  delete nextClipMasks[normalizedMaskLayerId];
  for (const layerId of targetSubtree) {
    if (layerId === normalizedMaskLayerId) {
      continue;
    }
    nextClipMasks[layerId] = normalizedMaskLayerId;
  }
  for (const layerId of draggedSubtree) {
    if (layerId === normalizedMaskLayerId) {
      continue;
    }
    nextClipMasks[layerId] = normalizedMaskLayerId;
  }
  return flattenLayerClipMasks(nextClipMasks);
}

function renderViewState(elements, state) {
  const viewMode = state?.view?.viewMode === "3d" ? "3d" : "2d";
  if (elements.viewReadout) {
    setTextContent(elements.viewReadout, viewMode === "3d" ? "3D" : "2D");
  }
  if (elements.viewToggle) {
    const nextMode = viewMode === "3d" ? "2D" : "3D";
    const currentMode = viewMode === "3d" ? "3D" : "2D";
    const label = `View mode: ${currentMode}. Click to switch to ${nextMode}.`;
    setAttributeValue(elements.viewToggle, "aria-label", label);
    setAttributeValue(elements.viewToggle, "title", label);
  }
  if (elements.viewToggleIcon) {
    setMarkup(
      elements.viewToggleIcon,
      viewMode,
      viewMode === "3d" ? cubeViewIcon() : mapViewIcon(),
    );
  }
}

function setHoverTooltipPosition(elements, clientX, clientY) {
  if (!elements.hoverTooltip) {
    return;
  }
  elements.hoverTooltip.style.setProperty("--fishymap-hover-x", `${clientX}px`);
  elements.hoverTooltip.style.setProperty("--fishymap-hover-y", `${clientY}px`);
}

function renderHoverTooltip(elements, hover) {
  if (!elements.hoverTooltip || !elements.hoverSummary || !elements.hoverLayers) {
    return;
  }
  const label = hover?.zoneName || (hover?.zoneRgb != null ? formatZone(hover.zoneRgb) : null);
  const layerSamples = Array.isArray(hover?.layerSamples) ? hover.layerSamples : [];
  if ((!label && layerSamples.length === 0) || !elements.hoverPointerActive) {
    setBooleanProperty(elements.hoverTooltip, "hidden", true);
    return;
  }
  if (label) {
    setTextContent(elements.hoverSummary, label);
  }
  setBooleanProperty(elements.hoverSummary, "hidden", !label);
  setMarkup(
    elements.hoverLayers,
    layerSamples
      .map((sample) => {
        const rgb = Array.isArray(sample?.rgb) ? sample.rgb : [];
        return `${sample?.layerId ?? ""}:${rgb.join(",")}`;
      })
      .join("|"),
    layerSamples
      .map((sample) => {
        const rgb = Array.isArray(sample?.rgb) ? sample.rgb : [0, 0, 0];
        const red = Number.parseInt(rgb[0], 10) || 0;
        const green = Number.parseInt(rgb[1], 10) || 0;
        const blue = Number.parseInt(rgb[2], 10) || 0;
        const layerName = String(sample?.layerName || sample?.layerId || "Layer").trim() || "Layer";
        const rgbLabel = `rgb(${red}, ${green}, ${blue})`;
        return `
          <div class="fishymap-hover-layer-row">
            <span class="fishymap-hover-layer-name">${escapeHtml(layerName)}</span>
            <span class="fishymap-hover-layer-value">
              <span class="fishymap-hover-layer-swatch" style="background-color: ${escapeHtml(rgbLabel)};"></span>
              <code>${escapeHtml(rgbLabel)}</code>
            </span>
          </div>
        `;
      })
      .join(""),
  );
  setBooleanProperty(elements.hoverLayers, "hidden", layerSamples.length === 0);
  setBooleanProperty(elements.hoverTooltip, "hidden", false);
}

function hoverFromEventDetail(detail) {
  if (detail?.hover && typeof detail.hover === "object") {
    return {
      ...detail.hover,
      layerSamples: Array.isArray(detail.hover.layerSamples) ? detail.hover.layerSamples : [],
    };
  }
  return {
    worldX: detail?.worldX ?? null,
    worldZ: detail?.worldZ ?? null,
    zoneRgb: detail?.zoneRgb ?? null,
    zoneName: detail?.zoneName ?? null,
    layerSamples: Array.isArray(detail?.layerSamples) ? detail.layerSamples : [],
  };
}

function renderFishAvatar(fish, sizeClass = "size-6", options = {}) {
  const name = fish?.name || `Fish ${fish?.fishId ?? "?"}`;
  const iconUrl = fishIconUrl(fish);
  const frameClass = options.gradeFrame
    ? `fishymap-item-icon-frame ${fishGradeFrameClass(fish)}`
    : "overflow-hidden rounded-full bg-base-200 ring-1 ring-base-300/80";
  if (iconUrl) {
    return `
      <span class="${sizeClass} shrink-0 ${frameClass}">
        <img
          class="${options.gradeFrame ? "fishymap-item-icon" : "h-full w-full object-cover"}"
          src="${escapeHtml(iconUrl)}"
          alt="${escapeHtml(name)}"
          loading="lazy"
          decoding="async"
        />
      </span>
    `;
  }
  const fallback = escapeHtml(String(name).trim().charAt(0).toUpperCase() || "?");
  return `
    <span class="${sizeClass} inline-flex shrink-0 items-center justify-center ${
      options.gradeFrame
        ? `fishymap-item-icon-frame ${fishGradeFrameClass(fish)} fishymap-item-icon-fallback`
        : "rounded-full bg-base-300 text-[11px] font-semibold text-base-content/70"
    }">
      ${fallback}
    </span>
  `;
}

function renderFishItemIcon(fish, sizeClass = "size-5") {
  const name = fish?.name || `Fish ${fish?.fishId ?? "?"}`;
  const iconUrl = fishIconUrl(fish);
  if (iconUrl) {
    return `
      <span class="${sizeClass} fishymap-item-icon-frame ${fishGradeFrameClass(fish)}">
        <img
          class="fishymap-item-icon"
          src="${escapeHtml(iconUrl)}"
          alt="${escapeHtml(name)}"
          loading="lazy"
          decoding="async"
        />
      </span>
    `;
  }
  const fallback = escapeHtml(String(name).trim().charAt(0).toUpperCase() || "?");
  return `
    <span class="${sizeClass} fishymap-item-icon-frame ${fishGradeFrameClass(fish)} fishymap-item-icon-fallback">
      ${fallback}
    </span>
  `;
}

function resolveSelectedFishIds(stateBundle) {
  const inputFishIds = stateBundle.inputState?.filters?.fishIds;
  if (Array.isArray(inputFishIds)) {
    return inputFishIds;
  }
  const stateFishIds = stateBundle.state?.filters?.fishIds;
  if (Array.isArray(stateFishIds)) {
    return stateFishIds;
  }
  return [];
}

function resolveSelectedZoneRgbs(stateBundle) {
  const inputZoneRgbs = stateBundle.inputState?.filters?.zoneRgbs;
  if (Array.isArray(inputZoneRgbs)) {
    return inputZoneRgbs;
  }
  const stateZoneRgbs = stateBundle.state?.filters?.zoneRgbs;
  if (Array.isArray(stateZoneRgbs)) {
    return stateZoneRgbs;
  }
  return [];
}

function scoreFishMatch(fish, queryTerms) {
  if (!queryTerms.length) {
    return 0;
  }
  const name = String(fish.name || "").toLowerCase();
  const id = String(fish.fishId || "");
  let score = 0;
  for (const term of queryTerms) {
    if (id === term) {
      score += 200;
      continue;
    }
    const idIndex = id.indexOf(term);
    if (idIndex >= 0) {
      score += 120 - idIndex;
      continue;
    }
    const nameIndex = name.indexOf(term);
    if (nameIndex >= 0) {
      score += 90 - Math.min(nameIndex, 60);
      continue;
    }
    return Number.NEGATIVE_INFINITY;
  }
  return score;
}

function scoreTermMatch(haystack, term, baseScore) {
  const index = String(haystack || "").indexOf(term);
  if (index < 0) {
    return Number.NEGATIVE_INFINITY;
  }
  return baseScore - Math.min(index, baseScore - 1);
}

function findFishMatches(catalogFish, searchText) {
  const query = String(searchText || "").trim().toLowerCase();
  const terms = query ? query.split(/\s+/g).filter(Boolean) : [];
  if (!terms.length) {
    return [];
  }
  const filtered = [];
  for (const fish of catalogFish || []) {
    const score = scoreFishMatch(fish, terms);
    if (!terms.length || Number.isFinite(score)) {
      filtered.push({
        ...fish,
        _score: Number.isFinite(score) ? score : 0,
      });
    }
  }
  filtered.sort((left, right) => {
    if (terms.length && right._score !== left._score) {
      return right._score - left._score;
    }
    return String(left.name || "").localeCompare(String(right.name || ""));
  });
  return filtered;
}

export function parseZoneRgbSearch(searchText) {
  const query = String(searchText || "").trim().toLowerCase();
  if (!query) {
    return null;
  }

  const compactQuery = query.replace(/\s+/g, "");
  const hexMatch = compactQuery.match(/^(?:#|0x)?([0-9a-f]{6})$/i);
  if (hexMatch) {
    return Number.parseInt(hexMatch[1], 16);
  }

  const sanitized = query.replace(/\b(?:rgb|rgba|vec3|vec4|normalized|norm|color|zone)\b/g, " ");
  const components =
    sanitized.match(/[+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?/g) || [];
  if (components.length !== 3 && components.length !== 4) {
    return null;
  }
  const remainder = sanitized
    .replace(/[+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?/g, "")
    .replace(/[\s,;:/()[\]{}]+/g, "");
  if (remainder) {
    return null;
  }

  const values = components.slice(0, 3).map((value) => Number.parseFloat(value));
  if (values.some((value) => !Number.isFinite(value) || value < 0)) {
    return null;
  }
  const normalized =
    values.every((value) => value <= 1) &&
    components
      .slice(0, 3)
      .some((value) => value.includes(".") || value.toLowerCase().includes("e"));
  const bytes = normalized
    ? values.map((value) => Math.round(value * 255))
    : values.map((value) =>
        value <= 255 && Math.abs(value - Math.round(value)) < 1e-6 ? Math.round(value) : null,
      );
  if (bytes.some((value) => !Number.isInteger(value) || value < 0 || value > 255)) {
    return null;
  }
  return rgbTripletToU32(bytes[0], bytes[1], bytes[2]);
}

function scoreZoneMatch(zone, queryTerms, parsedZoneRgb) {
  if (parsedZoneRgb != null && zone.zoneRgb === parsedZoneRgb) {
    return 500;
  }
  if (!queryTerms.length) {
    return 0;
  }
  let score = 0;
  for (const term of queryTerms) {
    const best = Math.max(
      scoreTermMatch(zone._nameSearch, term, 120),
      scoreTermMatch(zone.rgbKey, term, 220),
      scoreTermMatch(zone.rgbSpaced, term, 220),
      scoreTermMatch(zone.normalizedKey, term, 240),
      scoreTermMatch(zone.normalizedSpaced, term, 240),
      scoreTermMatch(zone.hexKey, term, 230),
      scoreTermMatch(zone.hashHexKey, term, 230),
      scoreTermMatch(zone.bareHexKey, term, 225),
    );
    if (!Number.isFinite(best)) {
      return Number.NEGATIVE_INFINITY;
    }
    score += best;
  }
  return score;
}

export function findZoneMatches(zoneCatalog, searchText) {
  const query = String(searchText || "").trim().toLowerCase();
  const terms = query ? query.split(/\s+/g).filter(Boolean) : [];
  if (!query) {
    return [];
  }
  const parsedZoneRgb = parseZoneRgbSearch(query);
  const filtered = [];
  for (const zone of zoneCatalog || []) {
    const score = scoreZoneMatch(zone, terms, parsedZoneRgb);
    if (!Number.isFinite(score)) {
      continue;
    }
    filtered.push({
      ...zone,
      _score: score,
    });
  }
  filtered.sort((left, right) => {
    if (right._score !== left._score) {
      return right._score - left._score;
    }
    if (left.confirmed !== right.confirmed) {
      return left.confirmed ? -1 : 1;
    }
    if (left.order !== right.order) {
      return left.order - right.order;
    }
    return String(left.name || "").localeCompare(String(right.name || ""));
  });
  return filtered;
}

function formatZone(zoneRgb) {
  if (zoneRgb == null) {
    return "none";
  }
  return `0x${Number(zoneRgb).toString(16).padStart(6, "0")}`;
}

function formatPatchDate(startTsUtc) {
  const tsMs = Number(startTsUtc) * 1000;
  if (!Number.isFinite(tsMs)) {
    return "";
  }
  const date = new Date(tsMs);
  const year = date.getUTCFullYear();
  const month = String(date.getUTCMonth() + 1).padStart(2, "0");
  const day = String(date.getUTCDate()).padStart(2, "0");
  return `${year}/${month}/${day}`;
}

function formatTimestampUtc(tsUtc) {
  const tsMs = Number(tsUtc) * 1000;
  if (!Number.isFinite(tsMs) || tsMs <= 0) {
    return "";
  }
  const date = new Date(tsMs);
  const year = date.getUTCFullYear();
  const month = String(date.getUTCMonth() + 1).padStart(2, "0");
  const day = String(date.getUTCDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function formatDecimal(value, digits = 3) {
  const number = Number(value);
  return Number.isFinite(number) ? number.toFixed(digits) : "n/a";
}

function formatPercent(value, digits = 1) {
  const number = Number(value);
  return Number.isFinite(number) ? `${(number * 100).toFixed(digits)}%` : "n/a";
}

function formatZoneStatus(status) {
  const raw = String(status || "").trim();
  if (!raw) {
    return "Unknown";
  }
  return raw
    .toLowerCase()
    .split(/[_\s-]+/g)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function buildZoneEvidenceSummary(zoneStats) {
  if (!zoneStats) {
    return "Click a zone on the map to load evidence.";
  }
  const parts = [];
  const confidence = zoneStats.confidence || {};
  const status = formatZoneStatus(confidence.status);
  if (status) {
    parts.push(status);
  }
  if (Number.isFinite(confidence.ess)) {
    parts.push(`ESS ${formatDecimal(confidence.ess, 1)}`);
  }
  if (Number.isFinite(confidence.totalWeight)) {
    parts.push(`weight ${formatDecimal(confidence.totalWeight, 2)}`);
  }
  const lastSeen = formatTimestampUtc(confidence.lastSeenTsUtc);
  if (lastSeen) {
    parts.push(`last seen ${lastSeen}`);
  } else if (Number.isFinite(confidence.ageDaysLast)) {
    parts.push(`last seen ${formatDecimal(confidence.ageDaysLast, 1)}d ago`);
  }
  const drift = confidence.drift;
  if (drift && Number.isFinite(drift.pDrift)) {
    parts.push(`drift ${formatPercent(drift.pDrift, 1)}`);
  }
  if (Array.isArray(confidence.notes) && confidence.notes.length) {
    parts.push(confidence.notes.join(" · "));
  }
  return parts.join(" · ") || "No confidence data.";
}

function ensureZoneEvidenceElements(elements) {
  if (elements.zoneEvidenceStatus && elements.zoneEvidenceSummary && elements.zoneEvidenceList) {
    return elements;
  }
  if (!elements.panelBody) {
    return elements;
  }

  const section = document.createElement("div");
  section.className = "space-y-2";
  section.innerHTML = `
    <div class="flex items-center justify-between gap-3">
      <span class="text-sm font-semibold">Zone Info</span>
      <span id="fishymap-zone-evidence-status" class="text-xs text-base-content/60">zone stats: idle</span>
    </div>
    <p id="fishymap-zone-evidence-summary" class="text-xs text-base-content/70">Click a zone on the map to load evidence.</p>
    <div id="fishymap-zone-evidence-list" class="max-h-72 overflow-y-auto rounded-box border border-base-300 bg-base-200 p-2"></div>
  `;

  if (elements.legend?.parentNode === elements.panelBody) {
    elements.panelBody.insertBefore(section, elements.legend);
  } else {
    elements.panelBody.appendChild(section);
  }

  elements.zoneEvidenceStatus = section.querySelector("#fishymap-zone-evidence-status");
  elements.zoneEvidenceSummary = section.querySelector("#fishymap-zone-evidence-summary");
  elements.zoneEvidenceList = section.querySelector("#fishymap-zone-evidence-list");
  return elements;
}

function orderPatchesByStart(patches) {
  return [...(patches || [])].sort(
    (left, right) => Number(left?.startTsUtc || 0) - Number(right?.startTsUtc || 0),
  );
}

function normalizePatchRangeSelection(patches, fromPatchId, toPatchId) {
  const ordered = orderPatchesByStart(patches);
  if (!ordered.length) {
    return {
      ordered,
      fromPatchId: "",
      toPatchId: "",
    };
  }

  const indexById = new Map(ordered.map((patch, index) => [patch.patchId, index]));
  let fromIndex = indexById.get(String(fromPatchId || ""));
  let toIndex = indexById.get(String(toPatchId || ""));

  if (!Number.isInteger(fromIndex)) {
    fromIndex = 0;
  }
  if (!Number.isInteger(toIndex)) {
    toIndex = ordered.length - 1;
  }
  if (toIndex < fromIndex) {
    [fromIndex, toIndex] = [toIndex, fromIndex];
  }

  return {
    ordered,
    fromPatchId: ordered[fromIndex]?.patchId || "",
    toPatchId: ordered[toIndex]?.patchId || "",
  };
}

function renderPatchOptions(select, orderedPatches, selectedPatchId, emptyLabel) {
  if (!select) {
    return;
  }
  if (!orderedPatches.length) {
    setMarkup(select, `empty:${emptyLabel}`, `<option value="">${emptyLabel}</option>`);
    select.value = "";
    return;
  }

  const options = orderedPatches.map((patch) => {
    const name = patch.patchName || patch.patchId;
    const date = formatPatchDate(patch.startTsUtc);
    const label = date ? `${name} (${date})` : name;
    return {
      patchId: patch.patchId,
      label,
      html: `<option value="${patch.patchId.replace(/"/g, "&quot;")}">${label}</option>`,
    };
  });

  setMarkup(
    select,
    JSON.stringify(options.map((option) => [option.patchId, option.label])),
    options.map((option) => option.html).join(""),
  );
  const nextValue = selectedPatchId || orderedPatches[0].patchId;
  if (select.value !== nextValue) {
    select.value = nextValue;
  }
}

function renderLayerStack(container, stateBundle) {
  const layers = resolveLayerEntries(stateBundle);
  if (!layers.length) {
    const loadingKey = "__loading__";
    if (container.dataset.renderKey !== loadingKey) {
      container.dataset.renderKey = loadingKey;
      container.innerHTML =
        '<p class="rounded-box border border-base-300/70 bg-base-200 px-3 py-3 text-xs text-base-content/60">Layer registry is loading…</p>';
    }
    return;
  }
  const renderKey = JSON.stringify(
    layers.map((layer) => [
      layer.layerId,
      layer.name,
      layer.kind,
      Boolean(layer.visible),
      Math.round(clampLayerOpacity(layer.opacity) * 1000),
      Math.round(clampLayerOpacity(layer.opacityDefault) * 1000),
      layer.clipMaskLayerId || "",
      Number.isFinite(layer.displayOrder) ? layer.displayOrder : 0,
      layer.locked ? 1 : 0,
    ]),
  );
  if (container.dataset.renderKey === renderKey) {
    return;
  }
  container.dataset.renderKey = renderKey;
  const layerNameById = new Map(layers.map((layer) => [layer.layerId, layer.name]));
  const clipMasks = {};
  for (const layer of layers) {
    const clipMaskLayerId = String(layer.clipMaskLayerId || "").trim();
    if (
      !clipMaskLayerId ||
      clipMaskLayerId === layer.layerId ||
      !layerNameById.has(clipMaskLayerId) ||
      isFixedGroundLayer(clipMaskLayerId)
    ) {
      continue;
    }
    clipMasks[layer.layerId] = clipMaskLayerId;
  }
  const flatClipMasks = flattenLayerClipMasks(clipMasks);
  const clippedLayersByMask = new Map();
  for (const layer of layers) {
    const clipMaskLayerId = String(flatClipMasks[layer.layerId] || "").trim();
    if (!clipMaskLayerId) {
      continue;
    }
    const clippedLayers = clippedLayersByMask.get(clipMaskLayerId) || [];
    clippedLayers.push({
      layer,
      indentLevel: 1,
    });
    clippedLayersByMask.set(clipMaskLayerId, clippedLayers);
  }
  const displayedLayers = [];
  const displayedLayerIds = new Set();
  for (const layer of layers) {
    if (flatClipMasks[layer.layerId]) {
      continue;
    }
    displayedLayers.push({ layer, indentLevel: 0 });
    displayedLayerIds.add(layer.layerId);
    for (const child of clippedLayersByMask.get(layer.layerId) || []) {
      displayedLayers.push(child);
      displayedLayerIds.add(child.layer.layerId);
    }
  }
  for (const layer of layers) {
    if (displayedLayerIds.has(layer.layerId)) {
      continue;
    }
    displayedLayers.push({ layer, indentLevel: 0 });
  }
  container.innerHTML = displayedLayers
    .map(({ layer, indentLevel }) => {
      const visible = Boolean(layer.visible);
      const locked = Boolean(layer.locked);
      const kind = layerKindLabel(layer.kind);
      const visibilityLabel = visible ? "Hide" : "Show";
      const clipMaskValue = String(flatClipMasks[layer.layerId] || "").trim();
      const clipMaskName = clipMaskValue ? layerNameById.get(clipMaskValue) || clipMaskValue : "";
      const clippedLayers = clippedLayersByMask.get(layer.layerId) || [];
      const clippedLayerNames = clippedLayers.map((candidate) => candidate.layer.name);
      const relationBadges = [];
      if (clipMaskName) {
        relationBadges.push(
          `<span class="badge badge-soft badge-xs">Clipped by ${escapeHtml(clipMaskName)}</span>`,
        );
      }
      if (clippedLayers.length) {
        relationBadges.push(
          `<span class="badge badge-soft badge-xs">Masks ${clippedLayers.length}</span>`,
        );
      }
      let helperText = "Drop onto a layer to attach. Drop between layers to reorder.";
      if (locked) {
        helperText = "Pinned as the base layer.";
      } else if (clipMaskName) {
        helperText = "Attached. Drop between layers to detach, or onto another layer to retarget.";
      } else if (clippedLayers.length) {
        helperText = "Drop another layer onto this card to use it as a clip mask.";
      }
      return `
        <article
          class="fishymap-layer-card card card-border bg-base-200"
          data-layer-id="${layer.layerId.replace(/"/g, "&quot;")}"
          data-indent-level="${indentLevel > 0 ? "1" : "0"}"
          data-locked="${locked ? "true" : "false"}"
          data-clip-mask-source="${locked ? "false" : "true"}"
          style="--fishymap-layer-indent:${indentLevel};"
        >
          <button
            class="fishymap-layer-drag btn btn-sm btn-circle btn-ghost"
            data-layer-drag="${layer.layerId.replace(/"/g, "&quot;")}"
            type="button"
            aria-label="${locked ? `${layer.name} is pinned to the ground layer` : `Drag ${layer.name}`}"
            draggable="${locked ? "false" : "true"}"
            ${locked ? "disabled" : ""}
            tabindex="-1"
          >
            ${dragHandleIcon()}
          </button>
          <div class="fishymap-layer-body min-w-0">
            <div class="flex items-center gap-2">
              <span class="truncate text-sm font-semibold">${escapeHtml(layer.name)}</span>
              <span class="badge badge-ghost badge-xs">${kind}</span>
              ${locked ? '<span class="badge badge-outline badge-xs">Ground</span>' : ""}
            </div>
            ${relationBadges.length ? `<div class="fishymap-layer-relations">${relationBadges.join("")}</div>` : ""}
            <p class="text-[11px] text-base-content/55">${helperText}</p>
            ${
              clippedLayerNames.length
                ? `
                  <p class="text-[11px] text-base-content/45">
                    Masking ${escapeHtml(clippedLayerNames.join(", "))}
                  </p>
                `
                : ""
            }
            ${
              locked
                ? ""
                : `
                  <fieldset class="fishymap-layer-opacity-control fieldset">
                    <div class="flex items-center justify-between gap-3">
                      <span class="fieldset-legend m-0 px-0 text-[11px] uppercase tracking-[0.18em] text-base-content/45">Opacity</span>
                      <span class="text-xs font-semibold text-base-content/60" data-layer-opacity-value>${layerOpacityLabel(layer.opacity)}</span>
                    </div>
                    <input
                      class="fishymap-layer-opacity-range range range-primary range-xs"
                      data-layer-opacity="${layer.layerId.replace(/"/g, "&quot;")}"
                      type="range"
                      min="0"
                      max="1"
                      step="0.05"
                      value="${layerOpacityValue(layer.opacity)}"
                      aria-label="Opacity for ${escapeHtml(layer.name)}"
                    >
                  </fieldset>
                `
            }
          </div>
          <button
            class="fishymap-layer-visibility btn btn-sm btn-circle ${
              visible ? "btn-soft btn-primary" : "btn-ghost"
            }"
            data-layer-visibility="${layer.layerId.replace(/"/g, "&quot;")}"
            data-layer-visible="${visible ? "true" : "false"}"
            type="button"
            aria-label="${visibilityLabel} ${escapeHtml(layer.name)}"
            title="${visibilityLabel} ${escapeHtml(layer.name)}"
          >
            ${eyeIcon(visible)}
          </button>
        </article>
      `;
    })
    .join("");
}

function addSelectedFishId(selectedFishIds, fishId) {
  return selectedFishIds.includes(fishId) ? selectedFishIds : selectedFishIds.concat(fishId);
}

function removeSelectedFishId(selectedFishIds, fishId) {
  return selectedFishIds.filter((id) => id !== fishId);
}

function addSelectedZoneRgb(selectedZoneRgbs, zoneRgb) {
  return selectedZoneRgbs.includes(zoneRgb) ? selectedZoneRgbs : selectedZoneRgbs.concat(zoneRgb);
}

function removeSelectedZoneRgb(selectedZoneRgbs, zoneRgb) {
  return selectedZoneRgbs.filter((value) => value !== zoneRgb);
}

export function buildSearchMatches(stateBundle, searchText, zoneCatalog = []) {
  const catalogFish = stateBundle.state?.catalog?.fish || [];
  const selectedFishIds = new Set(resolveSelectedFishIds(stateBundle));
  const selectedZoneRgbs = new Set(resolveSelectedZoneRgbs(stateBundle));
  const fishMatches = findFishMatches(catalogFish, searchText)
    .filter((fish) => !selectedFishIds.has(fish.fishId))
    .map((fish) => ({
      kind: "fish",
      ...fish,
    }));
  const zoneMatches = findZoneMatches(zoneCatalog, searchText).filter(
    (zone) => !selectedZoneRgbs.has(zone.zoneRgb),
  );
  return fishMatches.concat(zoneMatches).sort((left, right) => {
    if (right._score !== left._score) {
      return right._score - left._score;
    }
    if (left.kind !== right.kind) {
      return left.kind === "zone" ? -1 : 1;
    }
    return String(left.name || "").localeCompare(String(right.name || ""));
  });
}

export function renderSearchSelection(elements, stateBundle, fishLookup) {
  const selectedFishIds = resolveSelectedFishIds(stateBundle);
  const selectedZoneRgbs = resolveSelectedZoneRgbs(stateBundle);
  const hasSelection = selectedFishIds.length > 0 || selectedZoneRgbs.length > 0;
  const zoneLookup = new Map(
    (elements.zoneCatalog || []).map((zone) => [zone.zoneRgb, zone]),
  );
  const renderKey = JSON.stringify({
    selectedFishIds,
    selectedZoneRgbs,
    selectedFish: selectedFishIds.map((fishId) => {
      const fish = fishLookup.get(fishId);
      return [
        fishId,
        fish?.name || "",
        fish?.itemId || null,
        fish?.encyclopediaId || null,
        fish?.grade || "",
        fish?.isPrize === true ? 1 : 0,
      ];
    }),
    selectedZones: selectedZoneRgbs.map((zoneRgb) => {
      const zone = zoneLookup.get(zoneRgb);
      return [zoneRgb, zone?.name || "", zone?.rgbKey || ""];
    }),
  });
  if (elements.searchSelection.dataset.renderKey === renderKey) {
    elements.searchSelection.hidden = !hasSelection;
    if (elements.searchSelectionShell) {
      elements.searchSelectionShell.hidden = !hasSelection;
    }
    if (elements.searchWindow) {
      elements.searchWindow.dataset.hasSelection = hasSelection ? "true" : "false";
    }
    return;
  }
  elements.searchSelection.dataset.renderKey = renderKey;

  if (!hasSelection) {
    elements.searchSelection.innerHTML = "";
    elements.searchSelection.hidden = true;
    if (elements.searchSelectionShell) {
      elements.searchSelectionShell.hidden = true;
    }
    if (elements.searchWindow) {
      elements.searchWindow.dataset.hasSelection = "false";
    }
    return;
  }

  elements.searchSelection.hidden = false;
  if (elements.searchSelectionShell) {
    elements.searchSelectionShell.hidden = false;
  }
  if (elements.searchWindow) {
    elements.searchWindow.dataset.hasSelection = "true";
  }

  elements.searchSelection.innerHTML = selectedFishIds
    .map((fishId) => {
      const fish = fishLookup.get(fishId);
      const name = fish?.name || `Fish ${fishId}`;
      return `
        <div class="join items-center rounded-full border border-base-300 bg-base-100 p-1 text-base-content">
          <span class="inline-flex min-w-0 items-center gap-2 px-2 text-sm">
            ${renderFishAvatar(fish, "size-5", { gradeFrame: true })}
            <span class="truncate max-w-36">${escapeHtml(name)}</span>
          </span>
          <button
            class="fishymap-selection-remove btn btn-ghost btn-xs btn-circle join-item h-7 min-h-0 w-7 border-0 text-base-content/70"
            data-fish-id="${fishId}"
            type="button"
            aria-label="Remove ${escapeHtml(name)}"
          >
            ×
          </button>
        </div>
      `;
    })
    .concat(
      selectedZoneRgbs.map((zoneRgb) => {
        const zone = zoneLookup.get(zoneRgb);
        const name = zone?.name || `Zone ${formatZone(zoneRgb)}`;
        const swatch = `rgb(${zone?.r ?? 0}, ${zone?.g ?? 0}, ${zone?.b ?? 0})`;
        return `
          <div class="join items-center rounded-full border border-base-300 bg-base-100 p-1 text-base-content">
            <span class="inline-flex min-w-0 items-center gap-2 px-2 text-sm">
              <span
                class="inline-flex size-5 shrink-0 rounded-full border border-base-300 shadow-sm"
                style="background-color: ${escapeHtml(swatch)};"
                aria-hidden="true"
              ></span>
              <span class="truncate max-w-40">${escapeHtml(name)}</span>
            </span>
            <button
              class="fishymap-selection-remove btn btn-ghost btn-xs btn-circle join-item h-7 min-h-0 w-7 border-0 text-base-content/70"
              data-zone-rgb="${zoneRgb}"
              type="button"
              aria-label="Remove ${escapeHtml(name)}"
            >
              ×
            </button>
          </div>
        `;
      }),
    )
    .join("");
}

function renderSearchResults(elements, matches, stateBundle) {
  const query = String(stateBundle.inputState?.filters?.searchText || "").trim();
  const showResults = matches.length > 0;
  const activeMatches = matches.slice(0, 12);
  const renderKey = JSON.stringify({
    query,
    results: activeMatches.map((match) =>
      match.kind === "zone"
        ? ["zone", match.zoneRgb, match.name, match.rgbKey]
        : [
            "fish",
            match.fishId,
            match.itemId ?? null,
            match.encyclopediaId ?? null,
            match.grade || "",
            match.isPrize === true ? 1 : 0,
          ],
    ),
    total: matches.length,
  });
  if (elements.searchResultsShell) {
    setBooleanProperty(elements.searchResultsShell, "hidden", !showResults);
  }
  if (elements.searchCount) {
    setTextContent(elements.searchCount, `${matches.length} ${matches.length === 1 ? "match" : "matches"}`);
    setBooleanProperty(elements.searchCount, "hidden", !query);
  }
  if (elements.searchResults.dataset.renderKey === renderKey) {
    return;
  }
  elements.searchResults.dataset.renderKey = renderKey;
  if (!showResults) {
    elements.searchResults.innerHTML = "";
    return;
  }
  elements.searchResults.innerHTML = activeMatches
    .map((match) => {
      if (match.kind === "zone") {
        const swatch = `rgb(${match.r}, ${match.g}, ${match.b})`;
        return `
        <li>
          <button
            class="items-start gap-3 rounded-box px-3 py-2 text-sm"
            data-zone-rgb="${match.zoneRgb}"
            type="button"
          >
            <span
              class="mt-0.5 inline-flex size-6 shrink-0 rounded-full border border-base-300 shadow-sm"
              style="background-color: ${escapeHtml(swatch)};"
              aria-hidden="true"
            ></span>
            <span class="min-w-0 flex-1 text-left">
              <span class="flex items-center gap-2">
                <span class="truncate">${escapeHtml(match.name)}</span>
                <span class="badge badge-outline badge-xs">Zone</span>
              </span>
              <span class="block truncate text-xs text-base-content/60">
                <code>${escapeHtml(match.rgbKey)}</code>
                <span class="ml-2">${escapeHtml(formatZone(match.zoneRgb))}</span>
              </span>
            </span>
          </button>
        </li>
      `;
      }
      return `
        <li>
          <button
            class="gap-3 rounded-box px-3 py-2 text-sm"
            data-fish-id="${match.fishId}"
            type="button"
          >
            ${renderFishAvatar(match, "size-6", { gradeFrame: true })}
            <span class="truncate">${escapeHtml(match.name)}</span>
          </button>
        </li>
      `;
    })
    .join("");
}

function renderZoneEvidence(elements, stateBundle, fishLookup) {
  if (!elements.zoneEvidenceStatus || !elements.zoneEvidenceSummary || !elements.zoneEvidenceList) {
    return;
  }
  const zoneStats = stateBundle.state?.selection?.zoneStats || null;
  const zoneStatsStatus = stateBundle.state?.statuses?.zoneStatsStatus || "zone stats: idle";
  const summary = buildZoneEvidenceSummary(zoneStats);

  setTextContent(elements.zoneEvidenceStatus, zoneStatsStatus);
  setTextContent(elements.zoneEvidenceSummary, summary);

  const distribution = Array.isArray(zoneStats?.distribution) ? zoneStats.distribution : [];
  const renderKey = JSON.stringify({
    zoneRgb: zoneStats?.zoneRgb ?? null,
    status: zoneStatsStatus,
    summary,
    distribution: distribution.map((entry) => {
      const fish = fishLookup.get(entry.fishId);
      return [
        entry.fishId,
        entry.fishName || "",
        fish?.itemId ?? entry.itemId ?? null,
        fish?.encyclopediaId ?? entry.encyclopediaId ?? null,
        fish?.grade || "",
        fish?.isPrize === true ? 1 : 0,
        entry.evidenceWeight,
        entry.pMean,
        entry.ciLow,
        entry.ciHigh,
      ];
    }),
  });
  if (elements.zoneEvidenceList.dataset.renderKey === renderKey) {
    return;
  }
  elements.zoneEvidenceList.dataset.renderKey = renderKey;

  if (!zoneStats) {
    elements.zoneEvidenceList.innerHTML =
      '<div class="px-2 py-3 text-xs text-base-content/60">Click a zone on the map to load evidence.</div>';
    return;
  }
  if (!distribution.length) {
    elements.zoneEvidenceList.innerHTML =
      '<div class="px-2 py-3 text-xs text-base-content/60">No fish evidence in this window.</div>';
    return;
  }

  elements.zoneEvidenceList.innerHTML = distribution
    .map((entry) => {
      const fish = fishLookup.get(entry.fishId);
      const evidenceFish = {
        fishId: entry.fishId,
        itemId: fish?.itemId ?? entry.itemId ?? null,
        encyclopediaId: fish?.encyclopediaId ?? entry.encyclopediaId ?? null,
        name: fish?.name || entry.fishName || `Fish ${entry.fishId}`,
      };
      const ci =
        Number.isFinite(entry.ciLow) && Number.isFinite(entry.ciHigh)
          ? `${formatDecimal(entry.ciLow)}-${formatDecimal(entry.ciHigh)}`
          : "n/a";
      const detailLabel = `p ${formatDecimal(entry.pMean)} · weight ${formatDecimal(entry.evidenceWeight)} · CI ${ci}`;
      return `
        <button
          class="list-row w-full rounded-box border border-transparent bg-base-100 px-2.5 py-2 text-left hover:border-base-300"
          data-zone-evidence-fish-id="${evidenceFish.fishId}"
          type="button"
        >
          <div>${renderFishItemIcon(evidenceFish, "size-6")}</div>
          <div class="min-w-0">
            <div class="truncate font-semibold">${escapeHtml(evidenceFish.name)}</div>
          </div>
          <span
            class="badge badge-outline badge-sm cursor-help"
            title="${escapeHtml(detailLabel)}"
            aria-label="${escapeHtml(detailLabel)}"
          >${formatPercent(entry.pMean)}</span>
        </button>
      `;
    })
    .join("");
}

function renderStatusLines(container, statuses) {
  const lines = [
    statuses?.metaStatus,
    statuses?.layersStatus,
    statuses?.zonesStatus,
    statuses?.pointsStatus,
    statuses?.fishStatus,
    statuses?.zoneStatsStatus,
  ].filter(Boolean);
  setMarkup(
    container,
    JSON.stringify(lines),
    lines
      .map(
        (line) =>
          `<div class="rounded-box border border-base-300 bg-base-100 px-3 py-2">${escapeHtml(line)}</div>`,
      )
      .join(""),
  );
}

function setToolbarButtonState(button, open, label) {
  if (!button) {
    return;
  }
  const action = open ? "Hide" : "Show";
  button.dataset.open = open ? "true" : "false";
  button.setAttribute("aria-pressed", open ? "true" : "false");
  button.setAttribute("aria-label", `${action} ${label}`);
  button.title = `${action} ${label}`;
}

function setManagedWindowPosition(root, state) {
  if (!root) {
    return;
  }
  if (Number.isFinite(state?.x) && Number.isFinite(state?.y)) {
    root.style.left = `${state.x}px`;
    root.style.top = `${state.y}px`;
    root.style.right = "auto";
    root.style.bottom = "auto";
    root.style.transform = "none";
    return;
  }
  root.style.removeProperty("left");
  root.style.removeProperty("top");
  root.style.removeProperty("right");
  root.style.removeProperty("bottom");
  root.style.removeProperty("transform");
}

function isWindowTitlebarInteractiveTarget(target) {
  return Boolean(
    target instanceof Element &&
      target.closest(
        "input, textarea, select, button, a, label, summary, [data-window-drag-ignore='true']",
      ),
  );
}

function applyWindowVisibility(elements, windowUiState) {
  const managedWindows = [
    {
      state: windowUiState.search,
      root: elements.searchWindow,
      body: elements.searchBody,
      titlebar: elements.searchTitlebar,
      toggle: elements.searchWindowToggle,
      label: "Search",
    },
    {
      state: windowUiState.settings,
      root: elements.panel,
      body: elements.panelBody,
      titlebar: elements.panelTitlebar,
      toggle: elements.settingsWindowToggle,
      label: "Settings",
    },
    {
      state: windowUiState.zoneInfo,
      root: elements.zoneEvidenceWindow,
      body: elements.zoneEvidenceBody,
      titlebar: elements.zoneEvidenceTitlebar,
      toggle: elements.zoneInfoWindowToggle,
      label: "Zone Info",
    },
    {
      state: windowUiState.layers,
      root: elements.layersWindow,
      body: elements.layersBody,
      titlebar: elements.layersTitlebar,
      toggle: elements.layersWindowToggle,
      label: "Layers",
    },
  ];

  for (const windowPart of managedWindows) {
    const isOpen = windowPart.state?.open !== false;
    const isCollapsed = Boolean(windowPart.state?.collapsed);
    setBooleanProperty(windowPart.root, "hidden", !isOpen);
    if (windowPart.root) {
      windowPart.root.dataset.collapsed = isCollapsed ? "true" : "false";
    }
    if (windowPart.titlebar) {
      setAttributeValue(windowPart.titlebar, "aria-expanded", String(!isCollapsed));
    }
    if (windowPart.body) {
      setBooleanProperty(windowPart.body, "hidden", isCollapsed);
    }
    if (windowPart.root && isOpen) {
      setManagedWindowPosition(windowPart.root, windowPart.state);
    }
    setToolbarButtonState(windowPart.toggle, isOpen, windowPart.label);
  }
}

function syncLayerOpacityControl(container, layerId, opacity) {
  if (!container || !layerId) {
    return false;
  }
  const slider = Array.from(container.querySelectorAll("input[data-layer-opacity]")).find(
    (candidate) => candidate.getAttribute("data-layer-opacity") === layerId,
  );
  if (!slider) {
    return false;
  }
  const normalized = clampLayerOpacity(opacity);
  const value = layerOpacityValue(normalized);
  if (slider.value !== value) {
    slider.value = value;
  }
  const label = slider
    .closest(".fishymap-layer-opacity-control")
    ?.querySelector?.("[data-layer-opacity-value]");
  if (label) {
    setTextContent(label, layerOpacityLabel(normalized));
  }
  return true;
}

function renderPanel(elements, stateBundle, zoneCatalog = []) {
  const state = stateBundle.state || {};
  const inputState = stateBundle.inputState || {};
  const isReady = state.ready === true;
  const catalogFish = isReady ? state.catalog?.fish || [] : [];
  const patchRange = normalizePatchRangeSelection(
    isReady ? state.catalog?.patches || [] : [],
    inputState.filters?.fromPatchId ??
      state.filters?.fromPatchId ??
      inputState.filters?.patchId ??
      state.filters?.patchId ??
      null,
    inputState.filters?.toPatchId ??
      state.filters?.toPatchId ??
      inputState.filters?.patchId ??
      state.filters?.patchId ??
      null,
  );
  const searchText = inputState.filters?.searchText || "";
  const showPoints = (inputState.ui?.showPoints ?? state.ui?.showPoints) !== false;
  const showPointIcons =
    (inputState.ui?.showPointIcons ?? state.ui?.showPointIcons) !== false;
  const pointIconScale = clampPointIconScale(
    inputState.ui?.pointIconScale ?? state.ui?.pointIconScale ?? FISHYMAP_POINT_ICON_SCALE_MIN,
  );
  const fishLookup = mergeZoneEvidenceIntoFishLookup(buildFishLookup(catalogFish), isReady ? state.selection?.zoneStats || null : null);
  elements.zoneCatalog = zoneCatalog;

  applyThemeToShell(elements.shell);

  setTextContent(elements.readyPill, state.ready ? "Ready" : "Loading");
  setClassName(
    elements.readyPill,
    `badge badge-sm ${state.ready ? "badge-success" : "badge-outline"}`,
  );
  renderViewState(elements, state);
  if (elements.pointIconScale) {
    const sliderValue = pointIconScaleValue(pointIconScale);
    if (elements.pointIconScale.value !== sliderValue) {
      elements.pointIconScale.value = sliderValue;
    }
    setBooleanProperty(elements.pointIconScale, "disabled", !showPoints || !showPointIcons);
  }
  if (elements.pointIconScaleValue) {
    setTextContent(elements.pointIconScaleValue, pointIconScaleLabel(pointIconScale));
  }
  if (elements.showPoints) {
    setBooleanProperty(elements.showPoints, "checked", showPoints);
  }
  if (elements.showPointIcons) {
    setBooleanProperty(elements.showPointIcons, "checked", showPointIcons);
  }

  if (elements.search.value !== searchText) {
    elements.search.value = searchText;
  }

  renderPatchOptions(
    elements.patchFrom,
    patchRange.ordered,
    patchRange.fromPatchId,
    "Loading patches…",
  );
  renderPatchOptions(
    elements.patchTo,
    patchRange.ordered,
    patchRange.toPatchId,
    "Loading patches…",
  );
  if (
    isReady &&
    elements.layerOpacityInteraction?.activeLayerId &&
    Number.isFinite(elements.layerOpacityInteraction?.activeValue) &&
    syncLayerOpacityControl(
      elements.layers,
      elements.layerOpacityInteraction.activeLayerId,
      elements.layerOpacityInteraction.activeValue,
    )
  ) {
    // Keep the active slider mounted while the user is dragging it.
  } else {
    renderLayerStack(
      elements.layers,
      isReady ? stateBundle : { state: { catalog: { layers: [] } }, inputState: {} },
    );
  }
  if (elements.layersCount) {
    setTextContent(elements.layersCount, String(isReady ? (state.catalog?.layers || []).length : 0));
  }

  const matches = isReady ? buildSearchMatches(stateBundle, searchText, zoneCatalog) : [];
  renderSearchSelection(elements, stateBundle, fishLookup);
  renderSearchResults(elements, matches, stateBundle);
  renderZoneEvidence(elements, stateBundle, fishLookup);

  if (elements.legend) {
    setBooleanProperty(elements.legend, "open", Boolean(inputState.ui?.legendOpen));
  }
  if (elements.diagnostics) {
    setBooleanProperty(elements.diagnostics, "open", Boolean(inputState.ui?.diagnosticsOpen));
  }

  const zoneName =
    state.selection?.zoneName ||
    (state.selection?.zoneRgb != null ? `Zone ${formatZone(state.selection.zoneRgb)}` : null);
  if (elements.selectionSummary) {
    if (zoneName) {
      setTextContent(elements.selectionSummary, zoneName);
    } else {
      setTextContent(elements.selectionSummary, "No zone selected.");
    }
  }

  renderHoverTooltip(elements, state.hover || null);

  renderStatusLines(elements.statusLines, state.statuses || {});
  setTextContent(
    elements.diagnosticJson,
    JSON.stringify(state.lastDiagnostic || state.statuses || {}, null, 2),
  );
}

function applySearchMatchSelection(shell, elements, renderCurrentState, stateBundle, match) {
  if (!match) {
    return;
  }
  elements.search.value = "";
  dispatchMapState(shell, {
    version: 1,
    filters: {
      searchText: "",
      ...(match.kind === "fish"
        ? { fishIds: addSelectedFishId(resolveSelectedFishIds(stateBundle), match.fishId) }
        : { zoneRgbs: addSelectedZoneRgb(resolveSelectedZoneRgbs(stateBundle), match.zoneRgb) }),
    },
  });
  if (match.kind === "zone") {
    dispatchMapCommand(shell, {
      selectZoneRgb: match.zoneRgb,
    });
  }
  renderCurrentState(requestBridgeState(shell));
}

function bindUi(shell, elements, options = {}) {
  let isRendering = false;
  let latestStateBundle = requestBridgeState(shell);
  let zoneCatalog = normalizeZoneCatalog(options.zoneCatalog);
  let windowUiState = parseWindowUiState(elements.windowStateInput?.value);
  let nextWindowZIndex = 30;
  const managedWindows = {
    search: {
      root: elements.searchWindow,
      body: elements.searchBody,
      titlebar: elements.searchTitlebar,
      toggle: elements.searchWindowToggle,
    },
    settings: {
      root: elements.panel,
      body: elements.panelBody,
      titlebar: elements.panelTitlebar,
      toggle: elements.settingsWindowToggle,
    },
    zoneInfo: {
      root: elements.zoneEvidenceWindow,
      body: elements.zoneEvidenceBody,
      titlebar: elements.zoneEvidenceTitlebar,
      toggle: elements.zoneInfoWindowToggle,
    },
    layers: {
      root: elements.layersWindow,
      body: elements.layersBody,
      titlebar: elements.layersTitlebar,
      toggle: elements.layersWindowToggle,
    },
  };
  const toolbarTargetToWindowId = {
    search: "search",
    settings: "settings",
    "zone-info": "zoneInfo",
    layers: "layers",
  };
  const windowDragState = {
    windowId: null,
    pointerId: null,
    startClientX: 0,
    startClientY: 0,
    baseX: 0,
    baseY: 0,
    moved: false,
    titlebar: null,
  };
  const layerDragState = {
    draggingLayerId: null,
    overLayerId: null,
    mode: null,
  };
  const layerOpacityInteraction = {
    activeLayerId: null,
    activeValue: null,
  };
  elements.layerOpacityInteraction = layerOpacityInteraction;

  function stateBundleFromEvent(event) {
    return {
      state: event.detail?.state || FishyMapBridge.getCurrentState(),
      inputState:
        event.detail?.inputState ||
        (typeof FishyMapBridge.getCurrentInputState === "function"
          ? FishyMapBridge.getCurrentInputState()
          : {}),
    };
  }

  function persistWindowUiState() {
    if (!elements.windowStateInput) {
      return;
    }
    const serialized = serializeWindowUiState(windowUiState);
    if (elements.windowStateInput.value === serialized) {
      return;
    }
    elements.windowStateInput.value = serialized;
    elements.windowStateInput.dispatchEvent(new Event("input", { bubbles: true }));
  }

  function updateWindowUiEntry(windowId, patch) {
    if (!hasOwnKey(managedWindows, windowId)) {
      return false;
    }
    const currentEntry = windowUiState[windowId] || DEFAULT_WINDOW_UI_STATE[windowId];
    const nextEntry = normalizeWindowUiEntry(
      { ...currentEntry, ...patch },
      DEFAULT_WINDOW_UI_STATE[windowId],
    );
    if (
      currentEntry.open === nextEntry.open &&
      currentEntry.collapsed === nextEntry.collapsed &&
      currentEntry.x === nextEntry.x &&
      currentEntry.y === nextEntry.y
    ) {
      return false;
    }
    windowUiState = {
      ...windowUiState,
      [windowId]: nextEntry,
    };
    return true;
  }

  function bringManagedWindowToFront(windowId) {
    const root = managedWindows[windowId]?.root;
    if (!root) {
      return;
    }
    nextWindowZIndex += 1;
    root.style.zIndex = String(nextWindowZIndex);
  }

  function currentManagedWindowPosition(windowId) {
    const entry = windowUiState[windowId];
    if (Number.isFinite(entry?.x) && Number.isFinite(entry?.y)) {
      return {
        x: entry.x,
        y: entry.y,
      };
    }
    const root = managedWindows[windowId]?.root;
    if (!root) {
      return { x: 0, y: 0 };
    }
    const shellRect = elements.shell.getBoundingClientRect();
    const rootRect = root.getBoundingClientRect();
    return {
      x: Math.round(rootRect.left - shellRect.left),
      y: Math.round(rootRect.top - shellRect.top),
    };
  }

  function clampManagedWindowPosition(windowId, x, y) {
    const part = managedWindows[windowId];
    if (!part?.root) {
      return {
        x: normalizeWindowCoordinate(x) ?? 0,
        y: normalizeWindowCoordinate(y) ?? 0,
      };
    }
    setManagedWindowPosition(part.root, { x, y });
    const shellRect = elements.shell.getBoundingClientRect();
    const rootRect = part.root.getBoundingClientRect();
    const titlebarHeight = part.titlebar?.offsetHeight || WINDOW_TITLEBAR_FALLBACK_HEIGHT_PX;
    return {
      x: clamp(
        Math.round(x),
        0,
        Math.max(0, Math.round(shellRect.width - Math.min(rootRect.width, shellRect.width))),
      ),
      y: clamp(Math.round(y), 0, Math.max(0, Math.round(shellRect.height - titlebarHeight))),
    };
  }

  function clampOpenManagedWindows() {
    let changed = false;
    for (const windowId of Object.keys(managedWindows)) {
      const entry = windowUiState[windowId];
      if (
        !entry ||
        entry.open === false ||
        !Number.isFinite(entry.x) ||
        !Number.isFinite(entry.y)
      ) {
        continue;
      }
      const clamped = clampManagedWindowPosition(windowId, entry.x, entry.y);
      changed = updateWindowUiEntry(windowId, clamped) || changed;
    }
    return changed;
  }

  function applyManagedWindows({ persist = false } = {}) {
    applyWindowVisibility(elements, windowUiState);
    if (clampOpenManagedWindows()) {
      applyWindowVisibility(elements, windowUiState);
      persist = true;
    }
    if (persist) {
      persistWindowUiState();
    }
  }

  function toggleManagedWindowOpen(windowId) {
    const entry = windowUiState[windowId] || DEFAULT_WINDOW_UI_STATE[windowId];
    if (!updateWindowUiEntry(windowId, { open: entry.open === false })) {
      return;
    }
    if (windowUiState[windowId].open !== false) {
      bringManagedWindowToFront(windowId);
    } else if (windowId === "search") {
      elements.search?.blur?.();
    }
    applyManagedWindows({ persist: true });
  }

  function toggleManagedWindowCollapsed(windowId) {
    const entry = windowUiState[windowId] || DEFAULT_WINDOW_UI_STATE[windowId];
    if (!updateWindowUiEntry(windowId, { collapsed: !entry.collapsed })) {
      return;
    }
    bringManagedWindowToFront(windowId);
    applyManagedWindows({ persist: true });
  }

  function clearManagedWindowDrag() {
    if (windowDragState.windowId) {
      const root = managedWindows[windowDragState.windowId]?.root;
      if (root) {
        delete root.dataset.dragging;
      }
    }
    windowDragState.windowId = null;
    windowDragState.pointerId = null;
    windowDragState.moved = false;
    windowDragState.titlebar = null;
  }

  function finishManagedWindowDrag(toggleOnTap) {
    const windowId = windowDragState.windowId;
    const pointerId = windowDragState.pointerId;
    const titlebar = windowDragState.titlebar;
    const moved = windowDragState.moved;
    if (titlebar && pointerId != null && titlebar.hasPointerCapture?.(pointerId)) {
      titlebar.releasePointerCapture(pointerId);
    }
    clearManagedWindowDrag();
    if (!windowId) {
      return;
    }
    if (!moved && toggleOnTap && windowId !== "search") {
      toggleManagedWindowCollapsed(windowId);
      return;
    }
    applyManagedWindows({ persist: moved });
  }

  function renderCurrentState(stateBundle = requestBridgeState(shell)) {
    latestStateBundle = stateBundle;
    isRendering = true;
    try {
      renderPanel(elements, stateBundle, zoneCatalog);
      applyManagedWindows();
    } finally {
      isRendering = false;
    }
  }

  function clearLayerDropState() {
    layerDragState.overLayerId = null;
    layerDragState.mode = null;
    elements.layers
      ?.querySelectorAll?.(".fishymap-layer-card[data-drop-position]")
      ?.forEach?.((card) => {
        delete card.dataset.dropPosition;
      });
  }

  function applyLayerDropState(targetLayerId, mode) {
    clearLayerDropState();
    layerDragState.overLayerId = targetLayerId;
    layerDragState.mode = mode;
    const card = Array.from(
      elements.layers?.querySelectorAll?.(".fishymap-layer-card") || [],
    ).find((candidate) => candidate.getAttribute("data-layer-id") === targetLayerId);
    if (card) {
      card.dataset.dropPosition = mode;
    }
  }

  function setActiveLayerOpacity(slider) {
    if (!slider) {
      return;
    }
    const layerId = slider.getAttribute("data-layer-opacity");
    if (!layerId) {
      return;
    }
    layerOpacityInteraction.activeLayerId = layerId;
    layerOpacityInteraction.activeValue = clampLayerOpacity(slider.value);
    syncLayerOpacityControl(elements.layers, layerId, layerOpacityInteraction.activeValue);
  }

  function clearActiveLayerOpacity() {
    if (!layerOpacityInteraction.activeLayerId) {
      return;
    }
    latestStateBundle = requestBridgeState(shell);
    layerOpacityInteraction.activeLayerId = null;
    layerOpacityInteraction.activeValue = null;
    renderCurrentState(latestStateBundle);
  }

  elements.canvas.addEventListener("pointermove", (event) => {
    elements.hoverPointerActive = true;
    setHoverTooltipPosition(elements, event.clientX, event.clientY);
  });

  elements.canvas.addEventListener("pointerleave", () => {
    elements.hoverPointerActive = false;
    renderHoverTooltip(elements, null);
  });

  for (const windowId of Object.keys(managedWindows)) {
    managedWindows[windowId]?.root?.addEventListener(
      "pointerdown",
      () => {
        bringManagedWindowToFront(windowId);
      },
      { capture: true },
    );
    const titlebar = managedWindows[windowId]?.titlebar;
    if (!titlebar) {
      continue;
    }
    titlebar.addEventListener("pointerdown", (event) => {
      if (event.button !== 0) {
        return;
      }
      if (isWindowTitlebarInteractiveTarget(event.target)) {
        return;
      }
      const entry = windowUiState[windowId] || DEFAULT_WINDOW_UI_STATE[windowId];
      if (entry.open === false) {
        return;
      }
      const currentPosition = currentManagedWindowPosition(windowId);
      bringManagedWindowToFront(windowId);
      windowDragState.windowId = windowId;
      windowDragState.pointerId = event.pointerId;
      windowDragState.startClientX = event.clientX;
      windowDragState.startClientY = event.clientY;
      windowDragState.baseX = currentPosition.x;
      windowDragState.baseY = currentPosition.y;
      windowDragState.moved = false;
      windowDragState.titlebar = titlebar;
      if (managedWindows[windowId]?.root) {
        managedWindows[windowId].root.dataset.dragging = "true";
      }
      titlebar.setPointerCapture?.(event.pointerId);
      event.preventDefault();
    });

    titlebar.addEventListener("pointermove", (event) => {
      if (
        windowDragState.windowId !== windowId ||
        windowDragState.pointerId !== event.pointerId
      ) {
        return;
      }
      const deltaX = event.clientX - windowDragState.startClientX;
      const deltaY = event.clientY - windowDragState.startClientY;
      if (
        !windowDragState.moved &&
        Math.abs(deltaX) < WINDOW_DRAG_THRESHOLD_PX &&
        Math.abs(deltaY) < WINDOW_DRAG_THRESHOLD_PX
      ) {
        return;
      }
      windowDragState.moved = true;
      const nextPosition = clampManagedWindowPosition(
        windowId,
        windowDragState.baseX + deltaX,
        windowDragState.baseY + deltaY,
      );
      updateWindowUiEntry(windowId, nextPosition);
      setManagedWindowPosition(managedWindows[windowId]?.root, nextPosition);
    });

    titlebar.addEventListener("pointerup", (event) => {
      if (
        windowDragState.windowId !== windowId ||
        windowDragState.pointerId !== event.pointerId
      ) {
        return;
      }
      finishManagedWindowDrag(true);
    });

    titlebar.addEventListener("pointercancel", (event) => {
      if (
        windowDragState.windowId !== windowId ||
        windowDragState.pointerId !== event.pointerId
      ) {
        return;
      }
      finishManagedWindowDrag(false);
    });

    titlebar.addEventListener("keydown", (event) => {
      if (windowId === "search") {
        return;
      }
      if (isWindowTitlebarInteractiveTarget(event.target)) {
        return;
      }
      if (event.key !== "Enter" && event.key !== " ") {
        return;
      }
      event.preventDefault();
      toggleManagedWindowCollapsed(windowId);
    });
  }

  function pushSearchPatch() {
    const searchText = elements.search.value;
    dispatchMapState(shell, {
      version: 1,
      filters: {
        searchText,
      },
    });
    renderCurrentState(requestBridgeState(shell));
  }

  function pushPatchRangePatch() {
    const current = requestBridgeState(shell);
    const patchRange = normalizePatchRangeSelection(
      current.state.catalog?.patches || [],
      elements.patchFrom.value || null,
      elements.patchTo.value || null,
    );
    if (!patchRange.ordered.length) {
      return;
    }

    elements.patchFrom.value = patchRange.fromPatchId;
    elements.patchTo.value = patchRange.toPatchId;
    dispatchMapState(shell, {
      version: 1,
      filters: {
        fromPatchId: patchRange.fromPatchId,
        toPatchId: patchRange.toPatchId,
      },
    });
    renderCurrentState(requestBridgeState(shell));
  }

  elements.search.addEventListener("input", () => {
    if (isRendering) {
      return;
    }
    pushSearchPatch();
  });

  elements.search.addEventListener("keydown", (event) => {
    if (event.key !== "Enter") {
      return;
    }
    const current = requestBridgeState(shell);
    const matches = buildSearchMatches(current, elements.search.value, zoneCatalog);
    const top = matches[0];
    if (!top) {
      return;
    }
    event.preventDefault();
    applySearchMatchSelection(shell, elements, renderCurrentState, current, top);
  });

  elements.searchResults.addEventListener("click", (event) => {
    const button = event.target.closest("button[data-fish-id], button[data-zone-rgb]");
    if (!button) {
      return;
    }
    const current = requestBridgeState(shell);
    const zoneRgb = Number.parseInt(button.getAttribute("data-zone-rgb"), 10);
    if (Number.isFinite(zoneRgb)) {
      applySearchMatchSelection(shell, elements, renderCurrentState, current, {
        kind: "zone",
        zoneRgb,
      });
      return;
    }
    const fishId = Number.parseInt(button.getAttribute("data-fish-id"), 10);
    if (!Number.isFinite(fishId)) {
      return;
    }
    applySearchMatchSelection(shell, elements, renderCurrentState, current, {
      kind: "fish",
      fishId,
    });
  });

  elements.searchSelection.addEventListener("click", (event) => {
    const removeButton = event.target.closest(
      "button.fishymap-selection-remove[data-fish-id], button.fishymap-selection-remove[data-zone-rgb]",
    );
    if (!removeButton) {
      return;
    }
    const current = requestBridgeState(shell);
    const zoneRgb = Number.parseInt(removeButton.getAttribute("data-zone-rgb"), 10);
    if (Number.isFinite(zoneRgb)) {
      dispatchMapState(shell, {
        version: 1,
        filters: {
          zoneRgbs: removeSelectedZoneRgb(resolveSelectedZoneRgbs(current), zoneRgb),
        },
      });
      renderCurrentState(requestBridgeState(shell));
      return;
    }
    const fishId = Number.parseInt(removeButton.getAttribute("data-fish-id"), 10);
    dispatchMapState(shell, {
      version: 1,
      filters: {
        fishIds: removeSelectedFishId(resolveSelectedFishIds(current), fishId),
      },
    });
    renderCurrentState(requestBridgeState(shell));
  });

  if (elements.zoneEvidenceList) {
    elements.zoneEvidenceList.addEventListener("click", (event) => {
      const button = event.target.closest("button[data-zone-evidence-fish-id]");
      if (!button) {
        return;
      }
      const fishId = Number.parseInt(button.getAttribute("data-zone-evidence-fish-id"), 10);
      if (!Number.isFinite(fishId)) {
        return;
      }
      const current = requestBridgeState(shell);
      dispatchMapState(shell, {
        version: 1,
        filters: {
          fishIds: moveFishIdToCurrent(resolveSelectedFishIds(current), fishId),
        },
      });
      renderCurrentState(requestBridgeState(shell));
    });
  }

  elements.patchFrom.addEventListener("change", () => {
    if (isRendering) {
      return;
    }
    pushPatchRangePatch();
  });

  elements.patchTo.addEventListener("change", () => {
    if (isRendering) {
      return;
    }
    pushPatchRangePatch();
  });

  if (elements.viewToggle) {
    elements.viewToggle.addEventListener("click", () => {
      const current = latestStateBundle?.state || requestBridgeState(shell).state;
      const nextViewMode = current?.view?.viewMode === "3d" ? "2d" : "3d";
      dispatchMapCommand(shell, {
        setViewMode: nextViewMode,
      });
    });
  }

  if (elements.toolbar) {
    elements.toolbar.addEventListener("click", (event) => {
      const button = event.target.closest("button[data-window-toggle]");
      if (!button) {
        return;
      }
      const windowId = toolbarTargetToWindowId[button.getAttribute("data-window-toggle") || ""];
      if (!windowId) {
        return;
      }
      toggleManagedWindowOpen(windowId);
    });
  }

  if (elements.pointIconScale) {
    elements.pointIconScale.addEventListener("input", () => {
      if (isRendering) {
        return;
      }
      const pointIconScale = clampPointIconScale(elements.pointIconScale.value);
      if (elements.pointIconScaleValue) {
        elements.pointIconScaleValue.textContent = pointIconScaleLabel(pointIconScale);
      }
      dispatchMapState(shell, {
        version: 1,
        ui: {
          pointIconScale,
        },
      });
      renderCurrentState(requestBridgeState(shell));
    });
  }

  if (elements.showPoints) {
    elements.showPoints.addEventListener("change", () => {
      if (isRendering) {
        return;
      }
      dispatchMapState(shell, {
        version: 1,
        ui: {
          showPoints: elements.showPoints.checked,
        },
      });
      renderCurrentState(requestBridgeState(shell));
    });
  }

  if (elements.showPointIcons) {
    elements.showPointIcons.addEventListener("change", () => {
      if (isRendering) {
        return;
      }
      dispatchMapState(shell, {
        version: 1,
        ui: {
          showPointIcons: elements.showPointIcons.checked,
        },
      });
      renderCurrentState(requestBridgeState(shell));
    });
  }

  elements.layers.addEventListener("click", (event) => {
    const button = event.target.closest("button[data-layer-visibility]");
    if (isRendering || !button) {
      return;
    }
    const layerId = button.getAttribute("data-layer-visibility");
    if (!layerId) {
      return;
    }
    const current = requestBridgeState(shell);
    const visibleIds = new Set(resolveVisibleLayerIds(current));
    if (visibleIds.has(layerId)) {
      visibleIds.delete(layerId);
    } else {
      visibleIds.add(layerId);
    }
    dispatchMapState(shell, {
      version: 1,
      filters: {
        layerIdsVisible: resolveLayerEntries(current)
          .map((layer) => layer.layerId)
          .filter((candidateId) => visibleIds.has(candidateId)),
      },
    });
    renderCurrentState(requestBridgeState(shell));
  });

  elements.layers.addEventListener("input", (event) => {
    const slider = event.target.closest("input[data-layer-opacity]");
    if (isRendering || !slider) {
      return;
    }
    setActiveLayerOpacity(slider);
    const layerId = slider.getAttribute("data-layer-opacity");
    if (!layerId) {
      return;
    }
    const current = latestStateBundle || requestBridgeState(shell);
    dispatchMapState(shell, {
      version: 1,
      filters: {
        layerOpacities: buildLayerOpacityPatch(current, layerId, slider.value),
      },
    });
    latestStateBundle = requestBridgeState(shell);
  });

  elements.layers.addEventListener("pointerdown", (event) => {
    const slider = event.target.closest("input[data-layer-opacity]");
    if (!slider) {
      return;
    }
    setActiveLayerOpacity(slider);
  });

  elements.layers.addEventListener("focusin", (event) => {
    const slider = event.target.closest("input[data-layer-opacity]");
    if (!slider) {
      return;
    }
    setActiveLayerOpacity(slider);
  });

  elements.layers.addEventListener("change", (event) => {
    const slider = event.target.closest("input[data-layer-opacity]");
    if (slider) {
      setActiveLayerOpacity(slider);
      clearActiveLayerOpacity();
      return;
    }
  });

  elements.layers.addEventListener("focusout", (event) => {
    const slider = event.target.closest("input[data-layer-opacity]");
    if (!slider) {
      return;
    }
    queueMicrotask(() => {
      clearActiveLayerOpacity();
    });
  });

  elements.layers.addEventListener("dragstart", (event) => {
    const handle = event.target.closest("button[data-layer-drag][draggable='true']");
    const card = handle?.closest(".fishymap-layer-card");
    if (!card || !handle || isRendering) {
      return;
    }
    const layerId = card.getAttribute("data-layer-id");
    if (!layerId) {
      return;
    }
    layerDragState.draggingLayerId = layerId;
    card.dataset.dragging = "true";
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", layerId);
    }
  });

  elements.layers.addEventListener("dragover", (event) => {
    if (!layerDragState.draggingLayerId) {
      return;
    }
    const card = event.target.closest(".fishymap-layer-card");
    if (!card) {
      clearLayerDropState();
      return;
    }
    const targetLayerId = card.getAttribute("data-layer-id");
    if (!targetLayerId || targetLayerId === layerDragState.draggingLayerId) {
      clearLayerDropState();
      return;
    }
    event.preventDefault();
    const rect = card.getBoundingClientRect();
    const offsetY = event.clientY - rect.top;
    const locked = card.getAttribute("data-locked") === "true";
    const canAttach = card.getAttribute("data-clip-mask-source") === "true";
    let mode = "before";
    if (locked) {
      mode = "before";
    } else if (!canAttach) {
      mode = offsetY >= rect.height / 2 ? "after" : "before";
    } else {
      const edgeThreshold = Math.max(14, Math.min(rect.height * 0.24, 22));
      if (offsetY <= edgeThreshold) {
        mode = "before";
      } else if (offsetY >= rect.height - edgeThreshold) {
        mode = "after";
      } else {
        mode = "attach";
      }
    }
    applyLayerDropState(targetLayerId, mode);
  });

  elements.layers.addEventListener("drop", (event) => {
    if (
      !layerDragState.draggingLayerId ||
      !layerDragState.overLayerId ||
      !layerDragState.mode
    ) {
      clearLayerDropState();
      return;
    }
    event.preventDefault();
    const current = requestBridgeState(shell);
    const dropMode = layerDragState.mode;
    const nextOrder = moveLayerIdBefore(
      resolveLayerEntries(current),
      layerDragState.draggingLayerId,
      layerDragState.overLayerId,
      dropMode === "after" ? "after" : "before",
    );
    const nextClipMasks = buildLayerClipMaskPatch(
      current,
      layerDragState.draggingLayerId,
      dropMode === "attach" ? layerDragState.overLayerId : "",
    );
    clearLayerDropState();
    layerDragState.draggingLayerId = null;
    dispatchMapState(shell, {
      version: 1,
      filters: {
        layerIdsOrdered: nextOrder,
        layerClipMasks: nextClipMasks,
      },
    });
    renderCurrentState(requestBridgeState(shell));
  });

  elements.layers.addEventListener("dragend", () => {
    elements.layers
      ?.querySelectorAll?.(".fishymap-layer-card[data-dragging]")
      ?.forEach?.((card) => {
        delete card.dataset.dragging;
      });
    layerDragState.draggingLayerId = null;
    clearLayerDropState();
  });

  elements.layers.addEventListener("dragleave", (event) => {
    const related = event.relatedTarget;
    if (related && elements.layers.contains(related)) {
      return;
    }
    clearLayerDropState();
  });

  elements.resetView.addEventListener("click", () => {
    dispatchMapCommand(shell, { resetView: true });
  });

  if (elements.legend) {
    elements.legend.addEventListener("toggle", () => {
      if (isRendering) {
        return;
      }
      dispatchMapState(shell, {
        version: 1,
        ui: {
          legendOpen: elements.legend.open,
        },
      });
      renderCurrentState(requestBridgeState(shell));
    });
  }

  if (elements.diagnostics) {
    elements.diagnostics.addEventListener("toggle", () => {
      if (isRendering) {
        return;
      }
      dispatchMapState(shell, {
        version: 1,
        ui: {
          diagnosticsOpen: elements.diagnostics.open,
        },
      });
      renderCurrentState(requestBridgeState(shell));
    });
  }

  shell.addEventListener(FISHYMAP_EVENTS.ready, (event) => {
    renderCurrentState(stateBundleFromEvent(event));
  });

  shell.addEventListener(FISHYMAP_EVENTS.stateChanged, (event) => {
    renderCurrentState(stateBundleFromEvent(event));
  });

  shell.addEventListener(FISHYMAP_EVENTS.selectionChanged, (event) => {
    renderCurrentState(stateBundleFromEvent(event));
  });

  shell.addEventListener(FISHYMAP_EVENTS.diagnostic, (event) => {
    renderCurrentState(stateBundleFromEvent(event));
  });

  shell.addEventListener(FISHYMAP_EVENTS.viewChanged, (event) => {
    latestStateBundle = stateBundleFromEvent(event);
    renderViewState(elements, latestStateBundle.state);
  });

  shell.addEventListener(FISHYMAP_EVENTS.hoverChanged, (event) => {
    const hover = hoverFromEventDetail(event.detail || {});
    if (latestStateBundle?.state) {
      latestStateBundle.state = {
        ...latestStateBundle.state,
        hover,
      };
    }
    renderHoverTooltip(elements, hover);
  });

  window.addEventListener("fishystuff:themechange", () => applyThemeToShell(elements.shell));
  window.addEventListener("resize", () => applyManagedWindows({ persist: true }));

  renderCurrentState();
  return {
    setZoneCatalog(nextZoneCatalog) {
      zoneCatalog = normalizeZoneCatalog(nextZoneCatalog);
      renderCurrentState(requestBridgeState(shell));
    },
  };
}

async function main() {
  const shell = document.getElementById("map-page-shell");
  const canvas = document.getElementById("bevy");
  if (!shell || !canvas) {
    return;
  }

  const elements = {
    shell,
    toolbar: document.getElementById("fishymap-toolbar"),
    windowStateInput: document.getElementById("fishymap-window-state-input"),
    searchWindowToggle: document.querySelector("[data-window-toggle='search']"),
    settingsWindowToggle: document.querySelector("[data-window-toggle='settings']"),
    zoneInfoWindowToggle: document.querySelector("[data-window-toggle='zone-info']"),
    layersWindowToggle: document.querySelector("[data-window-toggle='layers']"),
    searchWindow: document.getElementById("fishymap-search-window"),
    searchTitlebar: document.getElementById("fishymap-search-titlebar"),
    searchBody: document.getElementById("fishymap-search-body"),
    panel: document.getElementById("fishymap-panel"),
    panelTitlebar: document.getElementById("fishymap-panel-titlebar"),
    panelBody: document.getElementById("fishymap-panel-body"),
    readyPill: document.getElementById("fishymap-ready-pill"),
    search: document.getElementById("fishymap-search"),
    searchSelectionShell: document.getElementById("fishymap-search-selection-shell"),
    searchSelection: document.getElementById("fishymap-search-selection"),
    searchResultsShell: document.getElementById("fishymap-search-results-shell"),
    searchResults: document.getElementById("fishymap-search-results"),
    searchCount: document.getElementById("fishymap-search-count"),
    patchFrom: document.getElementById("fishymap-patch-from"),
    patchTo: document.getElementById("fishymap-patch-to"),
    viewToggle: document.getElementById("fishymap-view-toggle"),
    viewToggleIcon: document.getElementById("fishymap-view-toggle-icon"),
    showPoints: document.getElementById("fishymap-show-points"),
    showPointIcons: document.getElementById("fishymap-show-point-icons"),
    pointIconScale: document.getElementById("fishymap-point-icon-scale"),
    pointIconScaleValue: document.getElementById("fishymap-point-icon-scale-value"),
    layers: document.getElementById("fishymap-layers"),
    layersWindow: document.getElementById("fishymap-layers-window"),
    layersTitlebar: document.getElementById("fishymap-layers-titlebar"),
    layersBody: document.getElementById("fishymap-layers-body"),
    layersCount: document.getElementById("fishymap-layers-count"),
    resetView: document.getElementById("fishymap-reset-view"),
    legend: document.getElementById("fishymap-legend"),
    diagnostics: document.getElementById("fishymap-diagnostics"),
    statusLines: document.getElementById("fishymap-status-lines"),
    diagnosticJson: document.getElementById("fishymap-diagnostic-json"),
    selectionSummary: document.getElementById("fishymap-selection-summary"),
    zoneEvidenceWindow: document.getElementById("fishymap-zone-evidence-window"),
    zoneEvidenceTitlebar: document.getElementById("fishymap-zone-evidence-titlebar"),
    zoneEvidenceBody: document.getElementById("fishymap-zone-evidence-body"),
    zoneEvidenceStatus: document.getElementById("fishymap-zone-evidence-status"),
    zoneEvidenceSummary: document.getElementById("fishymap-zone-evidence-summary"),
    zoneEvidenceList: document.getElementById("fishymap-zone-evidence-list"),
    hoverTooltip: document.getElementById("fishymap-hover-tooltip"),
    hoverSummary: document.getElementById("fishymap-hover-summary"),
    hoverLayers: document.getElementById("fishymap-hover-layers"),
    viewReadout: document.getElementById("fishymap-view-readout"),
    errorOverlay: document.getElementById("fishymap-error-overlay"),
    errorMessage: document.getElementById("fishymap-error-message"),
    canvas,
  };

  ensureZoneEvidenceElements(elements);

  const ui = bindUi(shell, elements);
  applyThemeToShell(shell);
  void loadZoneCatalog().then((zoneCatalog) => {
    ui.setZoneCatalog(zoneCatalog);
  });
  installRendererErrorHandlers(elements);

  if (!supportsWebgl2(document)) {
    setMapError(
      elements,
      "WebGL2 is required to render the map, but this browser/runtime did not provide a WebGL2 context.",
    );
    return;
  }

  try {
    await FishyMapBridge.mount(shell, { canvas });
  } catch (error) {
    console.error("Failed to mount FishyMap bridge", error);
    setMapError(elements, error);
  }
}

if (
  typeof window !== "undefined" &&
  typeof document !== "undefined" &&
  globalThis.__fishystuffLoaderAutoStart !== false
) {
  main();
}
