import FishyMapBridge, {
  FISHYMAP_CONTRACT_VERSION,
  FISHYMAP_EVENTS,
  FISHYMAP_POINT_ICON_SCALE_MAX,
  FISHYMAP_POINT_ICON_SCALE_MIN,
  FISHYMAP_STORAGE_KEYS,
  applyStatePatch,
  resolveApiBaseUrl,
  resolveCdnBaseUrl,
  zoneRgbFromLayerSamples,
} from "./map-host.js";
import { DATASTAR_SIGNAL_PATCH_EVENT } from "../js/datastar-signals.js";

const FIXED_GROUND_LAYER_IDS = new Set(["minimap"]);
const DEFAULT_ZONE_CATALOG_PATH = "/api/v1/zones";
const ICON_SPRITE_URL = "/img/icons.svg?v=20260326-2";
let currentZoneCatalog = [];
const WINDOW_DRAG_THRESHOLD_PX = 8;
const WINDOW_TITLEBAR_FALLBACK_HEIGHT_PX = 52;
const DRAG_AUTOSCROLL_EDGE_PX = 56;
const DRAG_AUTOSCROLL_MAX_STEP_PX = 20;
const BOOKMARK_COORDINATE_DECIMALS = 3;
const BOOKMARK_XML_POS_Y = "-8175.0";
const BOOKMARK_XML_GENERATED_BY = "FishyStuff";
const BOOKMARK_XML_PREVIEW_URL = "https://fishystuff.fish/map/";
const PRIMARY_SEMANTIC_ROW_KEYS = Object.freeze(["zone", "resources", "origin"]);
const TERRITORY_SUMMARY_FACT_KEYS = Object.freeze(["resources", "origin"]);
const DEFAULT_ZONE_INFO_TAB = "";
const DEFAULT_AUTO_ADJUST_VIEW = true;
const FISHYMAP_WINDOW_UI_STORAGE_KEY = "fishystuff.map.window_ui.v1";
const FISH_FILTER_TERM_ORDER = Object.freeze(["favourite", "missing"]);
const FISH_FILTER_TERM_METADATA = Object.freeze({
  favourite: Object.freeze({
    label: "Favourite",
    description: "Fish marked with a heart in Fishydex.",
    searchText: "favourite favourites favorite favorites heart liked",
    icon: "heart-fill",
    iconClass: "text-error",
  }),
  missing: Object.freeze({
    label: "Missing",
    description: "Fish not marked caught in Fishydex.",
    searchText: "missing uncaught not caught not yet caught",
    icon: "check-circle-dash-fill",
    iconClass: "text-warning",
  }),
});
const ZONE_INFO_TAB_BUTTON_CLASS =
  "tab shrink-0 gap-2 whitespace-nowrap text-xs font-semibold sm:text-sm";
const POINT_DETAIL_PANE_BUILDERS = Object.freeze([buildLayerSamplePointDetailPanes]);
const POINT_DETAIL_SECTION_BUILDERS = Object.freeze([buildZoneEvidencePointDetailSection]);
const MAP_WORLD_BOUNDS = Object.freeze({
  minX: -2048000,
  maxX: 1433600,
  minZ: -1126400,
  maxZ: 2048000,
});
const MAP_CAMERA_ZOOM_MIN_FACTOR_OF_FIT = 0.0025;
const FOCUS_RECT_PADDING_FACTOR = 1.35;
const FOCUS_MIN_SPAN_HEADROOM_FACTOR = 1.2;
const FOCUS_TERRAIN_DEFAULT_YAW = 0.0;
const FOCUS_TERRAIN_DEFAULT_PITCH = -0.58;
const FOCUS_TERRAIN_DISTANCE_FACTOR = 1.35;
const FOCUS_TERRAIN_MIN_DISTANCE = 2000.0;
const FOCUS_TERRAIN_MAX_DISTANCE = 900000.0;
const MAP_LEFT = -160.0;
const MAP_TOP = 160.0;
const MAP_SECTOR_PER_PIXEL = 0.0235294122248888;
const MAP_SECTOR_SCALE = 12800.0;
const DEFAULT_WINDOW_UI_STATE = Object.freeze({
  search: Object.freeze({ open: true, collapsed: false, x: null, y: null }),
  settings: Object.freeze({
    open: true,
    collapsed: false,
    x: null,
    y: null,
    autoAdjustView: DEFAULT_AUTO_ADJUST_VIEW,
  }),
  zoneInfo: Object.freeze({
    open: true,
    collapsed: false,
    x: null,
    y: null,
    tab: DEFAULT_ZONE_INFO_TAB,
  }),
  layers: Object.freeze({ open: true, collapsed: false, x: null, y: null }),
  bookmarks: Object.freeze({ open: false, collapsed: false, x: null, y: null }),
});
const DEFAULT_MAP_UI_SIGNAL_STATE = Object.freeze({
  windowUi: DEFAULT_WINDOW_UI_STATE,
  search: Object.freeze({ open: false }),
  bookmarks: Object.freeze({ placing: false, selectedIds: [] }),
});
const DEFAULT_MAP_INPUT_SIGNAL_STATE = Object.freeze({
  filters: Object.freeze({
    searchText: "",
    patchId: null,
    fromPatchId: null,
    toPatchId: null,
  }),
  ui: Object.freeze({
    diagnosticsOpen: false,
  }),
});
const DEFAULT_MAP_BOOKMARKS_SIGNAL_STATE = Object.freeze({
  entries: [],
});

function cloneJsonValue(value) {
  return JSON.parse(JSON.stringify(value));
}

function mapSignalHelper() {
  const helper = globalThis.window?.__fishystuffMap || globalThis.__fishystuffMap;
  return helper && typeof helper.signalObject === "function" ? helper : null;
}

function currentMapUiSignalState() {
  const helper = mapSignalHelper();
  const raw = helper?.readSignal?.("_map_ui");
  const normalizedBookmarks = raw?.bookmarks && typeof raw.bookmarks === "object"
    ? raw.bookmarks
    : {};
  return {
    windowUi: normalizeWindowUiState(raw?.windowUi),
    search: {
      open: raw?.search?.open === true,
    },
    bookmarks: {
      placing: normalizedBookmarks.placing === true,
      selectedIds: Array.isArray(normalizedBookmarks.selectedIds)
        ? normalizedBookmarks.selectedIds
            .map((value) => String(value ?? "").trim())
            .filter(Boolean)
        : [],
    },
  };
}

function patchMapUiSignalState(patch) {
  const helper = mapSignalHelper();
  if (!helper) {
    return;
  }
  const current = currentMapUiSignalState();
  helper.patchSignals({
    _map_ui: {
      windowUi: cloneJsonValue(
        patch?.windowUi && typeof patch.windowUi === "object"
          ? patch.windowUi
          : current.windowUi || DEFAULT_MAP_UI_SIGNAL_STATE.windowUi,
      ),
      search: cloneJsonValue(
        patch?.search && typeof patch.search === "object"
          ? patch.search
          : current.search || DEFAULT_MAP_UI_SIGNAL_STATE.search,
      ),
      bookmarks: cloneJsonValue(
        patch?.bookmarks && typeof patch.bookmarks === "object"
          ? patch.bookmarks
          : current.bookmarks || DEFAULT_MAP_UI_SIGNAL_STATE.bookmarks,
      ),
    },
  });
}

function currentMapInputSignalState() {
  const helper = mapSignalHelper();
  const raw = helper?.readSignal?.("_map_input");
  return applyStatePatch(
    DEFAULT_MAP_INPUT_SIGNAL_STATE,
    raw && typeof raw === "object" ? raw : {},
  );
}

function patchMapInputSignalState(patch) {
  const helper = mapSignalHelper();
  if (!helper) {
    return;
  }
  const current = currentMapInputSignalState();
  helper.patchSignals({
    _map_input: cloneJsonValue(applyStatePatch(current, patch)),
  });
}

function currentMapBookmarksSignalState() {
  const helper = mapSignalHelper();
  const raw = helper?.readSignal?.("_map_bookmarks.entries");
  return normalizeBookmarks(
    Array.isArray(raw) ? raw : DEFAULT_MAP_BOOKMARKS_SIGNAL_STATE.entries,
  );
}

function patchMapBookmarksSignalState(bookmarks) {
  const helper = mapSignalHelper();
  if (!helper) {
    return;
  }
  helper.patchSignals({
    _map_bookmarks: {
      entries: cloneJsonValue(normalizeBookmarks(bookmarks)),
    },
  });
}

function publishMapRuntimeSignals(stateBundle) {
  const helper = mapSignalHelper();
  if (!helper) {
    return;
  }
  helper.patchSignals({
    _map_runtime: {
      state: cloneJsonValue(stateBundle?.state || {}),
      inputState: cloneJsonValue(stateBundle?.inputState || {}),
    },
  });
}

function publishMapInputSignals(inputState) {
  const helper = mapSignalHelper();
  if (!helper) {
    return;
  }
  helper.patchSignals({
    _map_input: cloneJsonValue(
      applyStatePatch(
        DEFAULT_MAP_INPUT_SIGNAL_STATE,
        inputState && typeof inputState === "object" ? inputState : {},
      ),
    ),
  });
}

function dispatchMapEvent(target, type, detail) {
  target.dispatchEvent(new CustomEvent(type, { detail }));
}

function dispatchMapState(target, patch) {
  dispatchMapEvent(target, FISHYMAP_EVENTS.setState, patch);
}

function dispatchMapCommand(target, command) {
  dispatchMapEvent(target, FISHYMAP_EVENTS.command, command);
}

export function buildSelectWorldPointCommand(worldX, worldZ, options = {}) {
  const normalizedWorldX = normalizeBookmarkCoordinate(worldX);
  const normalizedWorldZ = normalizeBookmarkCoordinate(worldZ);
  if (normalizedWorldX == null || normalizedWorldZ == null) {
    return null;
  }
  const pointKind = normalizeSelectionPointKind(options.pointKind);
  const pointLabel =
    typeof options.pointLabel === "string" && options.pointLabel.trim()
      ? options.pointLabel.trim()
      : null;
  return {
    selectWorldPoint: {
      worldX: normalizedWorldX,
      worldZ: normalizedWorldZ,
      ...(pointKind ? { pointKind } : {}),
      ...(pointLabel ? { pointLabel } : {}),
    },
  };
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

function requestBridgeState(target, options = {}) {
  const detail = {};
  if (options.refresh === true) {
    dispatchMapEvent(target, FISHYMAP_EVENTS.requestState, detail);
  }
  return withSharedFishState({
    state: detail.state || FishyMapBridge.getCurrentState(),
    inputState:
      detail.inputState ||
      (typeof FishyMapBridge.getCurrentInputState === "function"
        ? FishyMapBridge.getCurrentInputState()
        : {}),
  });
}

export function projectStateBundleStatePatch(stateBundle, patch) {
  return withSharedFishState({
    ...(stateBundle || {}),
    state: stateBundle?.state || {},
    inputState: applyStatePatch(stateBundle?.inputState, patch),
  });
}

function normalizeSharedFishIds(values) {
  const helper = globalThis.window?.__fishystuffSharedFishState || globalThis.__fishystuffSharedFishState;
  if (helper && typeof helper.normalizeIds === "function") {
    return helper.normalizeIds(values);
  }
  if (!Array.isArray(values)) {
    return [];
  }
  const ids = [];
  const seen = new Set();
  for (const value of values) {
    const fishId = Number.parseInt(value, 10);
    if (!Number.isInteger(fishId) || fishId <= 0 || seen.has(fishId)) {
      continue;
    }
    seen.add(fishId);
    ids.push(fishId);
  }
  return ids;
}

function loadSharedFishState(storage = globalThis.localStorage) {
  const helper = globalThis.window?.__fishystuffSharedFishState || globalThis.__fishystuffSharedFishState;
  if (helper && typeof helper.loadState === "function") {
    return helper.loadState(
      {
        caught: FISHYMAP_STORAGE_KEYS.caught,
        favourites: FISHYMAP_STORAGE_KEYS.favourites,
      },
      storage,
    );
  }
  const read = (key) => {
    try {
      return normalizeSharedFishIds(JSON.parse(storage?.getItem?.(key) || "[]"));
    } catch (_) {
      return [];
    }
  };
  const caughtIds = read(FISHYMAP_STORAGE_KEYS.caught);
  const favouriteIds = read(FISHYMAP_STORAGE_KEYS.favourites);
  return {
    caughtIds,
    favouriteIds,
    caughtSet: new Set(caughtIds),
    favouriteSet: new Set(favouriteIds),
  };
}

function withSharedFishState(stateBundle) {
  const sharedFishState = stateBundle?.sharedFishState;
  if (sharedFishState?.caughtSet instanceof Set && sharedFishState?.favouriteSet instanceof Set) {
    return stateBundle || {};
  }
  const normalizedCaughtIds = normalizeSharedFishIds(sharedFishState?.caughtIds);
  const normalizedFavouriteIds = normalizeSharedFishIds(sharedFishState?.favouriteIds);
  if (normalizedCaughtIds.length || normalizedFavouriteIds.length) {
    return {
      ...(stateBundle || {}),
      sharedFishState: {
        caughtIds: normalizedCaughtIds,
        favouriteIds: normalizedFavouriteIds,
        caughtSet: new Set(normalizedCaughtIds),
        favouriteSet: new Set(normalizedFavouriteIds),
      },
    };
  }
  return {
    ...(stateBundle || {}),
    sharedFishState: loadSharedFishState(),
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

export function normalizeZoneInfoTab(value) {
  return String(value || "").trim();
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

function normalizeZoneInfoWindowUiEntry(rawEntry, fallbackEntry) {
  const baseEntry = isPlainObject(rawEntry) ? rawEntry : {};
  return {
    ...normalizeWindowUiEntry(baseEntry, fallbackEntry),
    tab: hasOwnKey(baseEntry, "tab")
      ? normalizeZoneInfoTab(baseEntry.tab)
      : normalizeZoneInfoTab(fallbackEntry?.tab),
  };
}

function normalizeSettingsWindowUiEntry(rawEntry, fallbackEntry) {
  const baseEntry = isPlainObject(rawEntry) ? rawEntry : {};
  return {
    ...normalizeWindowUiEntry(baseEntry, fallbackEntry),
    autoAdjustView: hasOwnKey(baseEntry, "autoAdjustView")
      ? baseEntry.autoAdjustView !== false
      : fallbackEntry?.autoAdjustView !== false,
  };
}

export function normalizeWindowUiState(rawState) {
  const source = isPlainObject(rawState) ? rawState : {};
  return {
    search: {
      ...normalizeWindowUiEntry(source.search, DEFAULT_WINDOW_UI_STATE.search),
      collapsed: false,
    },
    settings: normalizeSettingsWindowUiEntry(source.settings, DEFAULT_WINDOW_UI_STATE.settings),
    zoneInfo: normalizeZoneInfoWindowUiEntry(source.zoneInfo, DEFAULT_WINDOW_UI_STATE.zoneInfo),
    layers: normalizeWindowUiEntry(source.layers, DEFAULT_WINDOW_UI_STATE.layers),
    bookmarks: normalizeWindowUiEntry(source.bookmarks, DEFAULT_WINDOW_UI_STATE.bookmarks),
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

export function buildDefaultWindowUiStateSerialized() {
  return serializeWindowUiState(DEFAULT_WINDOW_UI_STATE);
}

function windowUiEntriesEqual(left, right) {
  return (
    Boolean(left?.open) === Boolean(right?.open) &&
    Boolean(left?.collapsed) === Boolean(right?.collapsed) &&
    normalizeWindowCoordinate(left?.x) === normalizeWindowCoordinate(right?.x) &&
    normalizeWindowCoordinate(left?.y) === normalizeWindowCoordinate(right?.y) &&
    String(left?.tab || "") === String(right?.tab || "") &&
    Boolean(left?.autoAdjustView !== false) === Boolean(right?.autoAdjustView !== false)
  );
}

export function buildMapUiResetMountOptions(currentState) {
  const view = isPlainObject(currentState?.view) ? currentState.view : null;
  if (!view) {
    return {};
  }
  return {
    initialState: {
      version: FISHYMAP_CONTRACT_VERSION,
      commands: {
        setViewMode: view.viewMode === "3d" ? "3d" : "2d",
        restoreView: view,
      },
    },
  };
}

export function normalizeBookmarkCoordinate(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return null;
  }
  return Number(number.toFixed(BOOKMARK_COORDINATE_DECIMALS));
}

function normalizeNullableString(value) {
  if (value == null) {
    return null;
  }
  const normalized = String(value).trim();
  return normalized || null;
}

function normalizeSelectionPointKind(value) {
  const normalized = String(value || "").trim().toLowerCase();
  return ["clicked", "waypoint", "bookmark"].includes(normalized) ? normalized : "";
}

function formatBookmarkCoordinate(value) {
  const normalized = normalizeBookmarkCoordinate(value);
  if (normalized == null) {
    return "";
  }
  return normalized
    .toFixed(BOOKMARK_COORDINATE_DECIMALS)
    .replace(/\.?0+$/, "");
}

function normalizeViewportSize(viewportInput) {
  const width = Number(viewportInput?.width);
  const height = Number(viewportInput?.height);
  return {
    width: Number.isFinite(width) && width > 1 ? width : 1280,
    height: Number.isFinite(height) && height > 1 ? height : 720,
  };
}

function measureMapViewportSize(elements) {
  const target = elements?.canvas || elements?.shell || null;
  if (!target) {
    return normalizeViewportSize(null);
  }
  const rect = target.getBoundingClientRect?.() || {};
  return normalizeViewportSize({
    width:
      rect.width ||
      target.clientWidth ||
      target.parentElement?.clientWidth ||
      globalThis.window?.innerWidth,
    height:
      rect.height ||
      target.clientHeight ||
      target.parentElement?.clientHeight ||
      globalThis.window?.innerHeight,
  });
}

function normalizeWorldPoint(pointInput) {
  const worldX = normalizeBookmarkCoordinate(pointInput?.worldX);
  const worldZ = normalizeBookmarkCoordinate(pointInput?.worldZ);
  if (worldX == null || worldZ == null) {
    return null;
  }
  return { worldX, worldZ };
}

function pixelToWorldPoint(pixelX, pixelY, pixelCenterOffset = 1) {
  const x = Number(pixelX);
  const y = Number(pixelY);
  if (!Number.isFinite(x) || !Number.isFinite(y)) {
    return null;
  }
  return normalizeWorldPoint({
    worldX: (x * MAP_SECTOR_PER_PIXEL + MAP_LEFT) * MAP_SECTOR_SCALE,
    worldZ: (-(y + pixelCenterOffset) * MAP_SECTOR_PER_PIXEL + MAP_TOP) * MAP_SECTOR_SCALE,
  });
}

function normalizeWorldRect(rectInput) {
  const minX = Number(rectInput?.minX);
  const maxX = Number(rectInput?.maxX);
  const minZ = Number(rectInput?.minZ);
  const maxZ = Number(rectInput?.maxZ);
  if (
    !Number.isFinite(minX) ||
    !Number.isFinite(maxX) ||
    !Number.isFinite(minZ) ||
    !Number.isFinite(maxZ)
  ) {
    return null;
  }
  const normalizedMinX = Math.min(minX, maxX);
  const normalizedMaxX = Math.max(minX, maxX);
  const normalizedMinZ = Math.min(minZ, maxZ);
  const normalizedMaxZ = Math.max(minZ, maxZ);
  return {
    minX: normalizedMinX,
    maxX: normalizedMaxX,
    minZ: normalizedMinZ,
    maxZ: normalizedMaxZ,
    centerX: (normalizedMinX + normalizedMaxX) * 0.5,
    centerZ: (normalizedMinZ + normalizedMaxZ) * 0.5,
    spanX: normalizedMaxX - normalizedMinX,
    spanZ: normalizedMaxZ - normalizedMinZ,
  };
}

function geometryPixelBounds(geometry) {
  let minX = Infinity;
  let maxX = -Infinity;
  let minY = Infinity;
  let maxY = -Infinity;

  function visitCoordinates(value) {
    if (!Array.isArray(value)) {
      return;
    }
    if (value.length >= 2 && Number.isFinite(value[0]) && Number.isFinite(value[1])) {
      minX = Math.min(minX, value[0]);
      maxX = Math.max(maxX, value[0]);
      minY = Math.min(minY, value[1]);
      maxY = Math.max(maxY, value[1]);
      return;
    }
    for (const child of value) {
      visitCoordinates(child);
    }
  }

  visitCoordinates(geometry?.coordinates);
  if (!Number.isFinite(minX) || !Number.isFinite(maxX) || !Number.isFinite(minY) || !Number.isFinite(maxY)) {
    return null;
  }
  return { minX, maxX, minY, maxY };
}

function worldRectFromGeometry(geometry) {
  const pixelBounds = geometryPixelBounds(geometry);
  if (!pixelBounds) {
    return null;
  }
  const topLeft = pixelToWorldPoint(pixelBounds.minX, pixelBounds.minY, 0);
  const bottomRight = pixelToWorldPoint(pixelBounds.maxX, pixelBounds.maxY, 0);
  if (!topLeft || !bottomRight) {
    return null;
  }
  return normalizeWorldRect({
    minX: topLeft.worldX,
    maxX: bottomRight.worldX,
    minZ: bottomRight.worldZ,
    maxZ: topLeft.worldZ,
  });
}

function dedupeWorldPoints(pointsInput) {
  const seen = new Set();
  const points = [];
  for (const pointInput of Array.isArray(pointsInput) ? pointsInput : []) {
    const point = normalizeWorldPoint(pointInput);
    if (!point) {
      continue;
    }
    const key = `${point.worldX}:${point.worldZ}`;
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    points.push(point);
  }
  return points;
}

function focusFitScale(viewportSize) {
  const boundsWidth = MAP_WORLD_BOUNDS.maxX - MAP_WORLD_BOUNDS.minX;
  const boundsHeight = MAP_WORLD_BOUNDS.maxZ - MAP_WORLD_BOUNDS.minZ;
  return Math.max(boundsWidth / viewportSize.width, boundsHeight / viewportSize.height);
}

export function buildFocusWorldRect(pointsInput, viewportInput, options = {}) {
  const points = dedupeWorldPoints(pointsInput);
  if (!points.length) {
    return null;
  }
  const viewportSize = normalizeViewportSize(viewportInput);
  let minX = points[0].worldX;
  let maxX = points[0].worldX;
  let minZ = points[0].worldZ;
  let maxZ = points[0].worldZ;
  for (const point of points) {
    minX = Math.min(minX, point.worldX);
    maxX = Math.max(maxX, point.worldX);
    minZ = Math.min(minZ, point.worldZ);
    maxZ = Math.max(maxZ, point.worldZ);
  }
  const centerX = (minX + maxX) * 0.5;
  const centerZ = (minZ + maxZ) * 0.5;
  const fitScale = focusFitScale(viewportSize);
  const minScale = fitScale * MAP_CAMERA_ZOOM_MIN_FACTOR_OF_FIT;
  const minSpanX = viewportSize.width * minScale * FOCUS_MIN_SPAN_HEADROOM_FACTOR;
  const minSpanZ = viewportSize.height * minScale * FOCUS_MIN_SPAN_HEADROOM_FACTOR;
  const spanX = Math.max((maxX - minX) * FOCUS_RECT_PADDING_FACTOR, minSpanX);
  const spanZ = Math.max((maxZ - minZ) * FOCUS_RECT_PADDING_FACTOR, minSpanZ);
  return {
    minX: centerX - spanX * 0.5,
    maxX: centerX + spanX * 0.5,
    minZ: centerZ - spanZ * 0.5,
    maxZ: centerZ + spanZ * 0.5,
    centerX,
    centerZ,
    spanX,
    spanZ,
  };
}

function buildFocusWorldRectFromBaseRect(rectInput, viewportInput) {
  const rect = normalizeWorldRect(rectInput);
  if (!rect) {
    return null;
  }
  const viewportSize = normalizeViewportSize(viewportInput);
  const fitScale = focusFitScale(viewportSize);
  const minScale = fitScale * MAP_CAMERA_ZOOM_MIN_FACTOR_OF_FIT;
  const minSpanX = viewportSize.width * minScale * FOCUS_MIN_SPAN_HEADROOM_FACTOR;
  const minSpanZ = viewportSize.height * minScale * FOCUS_MIN_SPAN_HEADROOM_FACTOR;
  const spanX = Math.max(rect.spanX * FOCUS_RECT_PADDING_FACTOR, minSpanX);
  const spanZ = Math.max(rect.spanZ * FOCUS_RECT_PADDING_FACTOR, minSpanZ);
  return {
    minX: rect.centerX - spanX * 0.5,
    maxX: rect.centerX + spanX * 0.5,
    minZ: rect.centerZ - spanZ * 0.5,
    maxZ: rect.centerZ + spanZ * 0.5,
    centerX: rect.centerX,
    centerZ: rect.centerZ,
    spanX,
    spanZ,
  };
}

export function buildRestoreViewForWorldRect(rectInput, viewportInput, stateBundle) {
  const rect = isPlainObject(rectInput) ? rectInput : null;
  if (!rect) {
    return null;
  }
  const centerWorldX = Number(rect.centerX);
  const centerWorldZ = Number(rect.centerZ);
  const spanX = Number(rect.spanX);
  const spanZ = Number(rect.spanZ);
  if (
    !Number.isFinite(centerWorldX) ||
    !Number.isFinite(centerWorldZ) ||
    !Number.isFinite(spanX) ||
    !Number.isFinite(spanZ)
  ) {
    return null;
  }
  const viewMode = stateBundle?.state?.view?.viewMode === "3d" ? "3d" : "2d";
  if (viewMode === "3d") {
    const distance = clamp(
      Math.max(spanX, spanZ, 1) * FOCUS_TERRAIN_DISTANCE_FACTOR,
      FOCUS_TERRAIN_MIN_DISTANCE,
      FOCUS_TERRAIN_MAX_DISTANCE,
    );
    return {
      viewMode: "3d",
      camera: {
        pivotWorldX: centerWorldX,
        pivotWorldY: 0,
        pivotWorldZ: centerWorldZ,
        yaw: FOCUS_TERRAIN_DEFAULT_YAW,
        pitch: FOCUS_TERRAIN_DEFAULT_PITCH,
        distance,
      },
    };
  }
  const viewportSize = normalizeViewportSize(viewportInput);
  const zoom = Math.max(spanX / viewportSize.width, spanZ / viewportSize.height, 1e-5);
  return {
    viewMode: "2d",
    camera: {
      centerWorldX,
      centerWorldZ,
      zoom,
    },
  };
}

function normalizeFeatureCollectionFeatures(collection) {
  return Array.isArray(collection?.features) ? collection.features : [];
}

function normalizeIntegerId(value) {
  const number = Number.parseInt(value, 10);
  return Number.isFinite(number) && number > 0 ? number : null;
}

function addWaypointIndexEntry(waypointById, waypointId, pointInput) {
  if (!Number.isFinite(waypointId) || waypointId <= 0) {
    return;
  }
  const point = normalizeWorldPoint(pointInput);
  if (!point) {
    return;
  }
  waypointById.set(waypointId, point);
}

export function buildWaypointFocusIndex(sources = {}) {
  const waypointById = new Map();
  const regionNodeByRegionId = new Map();
  const regionById = new Map();
  const regionGroupById = new Map();

  for (const feature of normalizeFeatureCollectionFeatures(sources.regionNodes)) {
    if (String(feature?.geometry?.type || "").trim() !== "Point") {
      continue;
    }
    const coordinates = Array.isArray(feature?.geometry?.coordinates)
      ? feature.geometry.coordinates
      : [];
    const point = normalizeWorldPoint({
      worldX: coordinates[0],
      worldZ: coordinates[1],
    });
    if (!point) {
      continue;
    }
    const regionId = normalizeIntegerId(feature?.properties?.r);
    const waypointId = normalizeIntegerId(feature?.properties?.wp);
    if (regionId != null) {
      regionNodeByRegionId.set(regionId, point);
    }
    if (waypointId != null) {
      addWaypointIndexEntry(waypointById, waypointId, point);
    }
  }

  for (const feature of normalizeFeatureCollectionFeatures(sources.regions)) {
    const regionId = normalizeIntegerId(feature?.properties?.r);
    if (regionId == null) {
      continue;
    }
    const originWaypointId = normalizeIntegerId(feature?.properties?.owp);
    const resourceWaypointId = normalizeIntegerId(feature?.properties?.rgwp);
    const regionEntry = {
      regionId,
      regionGroupId: normalizeIntegerId(feature?.properties?.rg),
      originRegionId: normalizeIntegerId(feature?.properties?.o),
      originWaypointId,
      resourceWaypointId,
      bounds: worldRectFromGeometry(feature?.geometry),
      originPoint: normalizeWorldPoint({
        worldX: feature?.properties?.ox,
        worldZ: feature?.properties?.oz,
      }),
      resourcePoint: normalizeWorldPoint({
        worldX: feature?.properties?.rgx,
        worldZ: feature?.properties?.rgz,
      }),
    };
    regionById.set(regionId, regionEntry);
    addWaypointIndexEntry(waypointById, originWaypointId, regionEntry.originPoint);
    addWaypointIndexEntry(waypointById, resourceWaypointId, regionEntry.resourcePoint);
  }

  for (const feature of normalizeFeatureCollectionFeatures(sources.regionGroups)) {
    const regionGroupId = normalizeIntegerId(feature?.properties?.rg);
    if (regionGroupId == null) {
      continue;
    }
    const waypointId = normalizeIntegerId(feature?.properties?.rgwp);
    const point = normalizeWorldPoint({
      worldX: feature?.properties?.rgx,
      worldZ: feature?.properties?.rgz,
    });
    regionGroupById.set(regionGroupId, {
      regionGroupId,
      waypointId,
      bounds: worldRectFromGeometry(feature?.geometry),
      point,
      memberRegionIds: (Array.isArray(feature?.properties?.rs) ? feature.properties.rs : [])
        .map((value) => normalizeIntegerId(value))
        .filter((value) => value != null),
    });
    addWaypointIndexEntry(waypointById, waypointId, point);
  }

  return {
    waypointById,
    regionNodeByRegionId,
    regionById,
    regionGroupById,
  };
}

let waypointFocusIndexPromise = null;
let waypointFocusPreloadScheduled = false;
let zoneFocusIndexPromise = null;
let fishEvidenceSnapshotPromise = null;

function zoneRgbTriplet(zoneRgb) {
  const value = Number.parseInt(zoneRgb, 10);
  if (!Number.isFinite(value)) {
    return null;
  }
  return {
    r: (value >> 16) & 0xff,
    g: (value >> 8) & 0xff,
    b: value & 0xff,
  };
}

export function buildZoneFocusIndexFromLookupBytes(bytesInput) {
  const bytes =
    bytesInput instanceof Uint8Array
      ? bytesInput
      : bytesInput instanceof ArrayBuffer
        ? new Uint8Array(bytesInput)
        : null;
  if (!bytes || bytes.byteLength < 20) {
    return { zoneRectByRgb: new Map() };
  }
  const magic = "FSZLKP01";
  for (let index = 0; index < magic.length; index += 1) {
    if (bytes[index] !== magic.charCodeAt(index)) {
      return { zoneRectByRgb: new Map() };
    }
  }
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const width = view.getUint16(8, true);
  const height = view.getUint16(10, true);
  const rowOffsetCount = view.getUint32(12, true);
  const segmentCount = view.getUint32(16, true);
  const expectedLength = 20 + rowOffsetCount * 4 + segmentCount * 2 + segmentCount * 4;
  if (bytes.byteLength !== expectedLength || rowOffsetCount !== height + 1) {
    return { zoneRectByRgb: new Map() };
  }

  let cursor = 20;
  const rowOffsets = new Uint32Array(rowOffsetCount);
  for (let index = 0; index < rowOffsetCount; index += 1) {
    rowOffsets[index] = view.getUint32(cursor, true);
    cursor += 4;
  }
  const rowEndXs = new Uint16Array(segmentCount);
  for (let index = 0; index < segmentCount; index += 1) {
    rowEndXs[index] = view.getUint16(cursor, true);
    cursor += 2;
  }
  const rowIds = new Uint32Array(segmentCount);
  for (let index = 0; index < segmentCount; index += 1) {
    rowIds[index] = view.getUint32(cursor, true);
    cursor += 4;
  }

  const pixelBoundsByRgb = new Map();
  for (let row = 0; row < height; row += 1) {
    const start = rowOffsets[row];
    const end = rowOffsets[row + 1];
    let spanStart = 0;
    for (let index = start; index < end; index += 1) {
      const spanEnd = rowEndXs[index];
      const zoneRgb = rowIds[index];
      if (zoneRgb !== 0 && spanEnd > spanStart) {
        const current = pixelBoundsByRgb.get(zoneRgb);
        if (current) {
          current.minX = Math.min(current.minX, spanStart);
          current.maxX = Math.max(current.maxX, spanEnd);
          current.minY = Math.min(current.minY, row);
          current.maxY = Math.max(current.maxY, row + 1);
        } else {
          pixelBoundsByRgb.set(zoneRgb, {
            minX: spanStart,
            maxX: spanEnd,
            minY: row,
            maxY: row + 1,
          });
        }
      }
      spanStart = spanEnd;
    }
  }

  const zoneRectByRgb = new Map();
  for (const [zoneRgb, bounds] of pixelBoundsByRgb) {
    const topLeft = pixelToWorldPoint(bounds.minX, bounds.minY, 0);
    const bottomRight = pixelToWorldPoint(bounds.maxX, bounds.maxY, 0);
    const rect =
      topLeft && bottomRight
        ? normalizeWorldRect({
            minX: topLeft.worldX,
            maxX: bottomRight.worldX,
            minZ: bottomRight.worldZ,
            maxZ: topLeft.worldZ,
          })
        : null;
    if (rect) {
      zoneRectByRgb.set(zoneRgb, rect);
    }
  }
  return { zoneRectByRgb };
}

async function loadWaypointFocusIndex(locationLike = globalThis.window?.location) {
  if (waypointFocusIndexPromise) {
    return waypointFocusIndexPromise;
  }
  if (typeof globalThis.fetch !== "function") {
    throw new Error("fetch() is unavailable for waypoint focus data.");
  }
  const cdnBaseUrl = resolveCdnBaseUrl(locationLike);
  const urls = {
    regionNodes: `${cdnBaseUrl}/waypoints/region_nodes.v1.geojson`,
    regions: `${cdnBaseUrl}/region_groups/regions.v1.geojson`,
    regionGroups: `${cdnBaseUrl}/region_groups/v1.geojson`,
  };
  waypointFocusIndexPromise = Promise.all(
    Object.values(urls).map(async (url) => {
      const response = await globalThis.fetch(url);
      if (!response.ok) {
        throw new Error(`Failed to load waypoint focus data: ${url}`);
      }
      return response.json();
    }),
  )
    .then(([regionNodes, regions, regionGroups]) =>
      buildWaypointFocusIndex({ regionNodes, regions, regionGroups }),
    )
    .catch((error) => {
      waypointFocusIndexPromise = null;
      throw error;
    });
  return waypointFocusIndexPromise;
}

async function loadZoneFocusIndex(locationLike = globalThis.window?.location) {
  if (zoneFocusIndexPromise) {
    return zoneFocusIndexPromise;
  }
  if (typeof globalThis.fetch !== "function") {
    throw new Error("fetch() is unavailable for zone focus data.");
  }
  const cdnBaseUrl = resolveCdnBaseUrl(locationLike);
  const url = `${cdnBaseUrl}/images/exact_lookup/zone_mask.v1.bin`;
  zoneFocusIndexPromise = globalThis.fetch(url)
    .then(async (response) => {
      if (!response.ok) {
        throw new Error(`Failed to load zone focus data: ${url}`);
      }
      return buildZoneFocusIndexFromLookupBytes(await response.arrayBuffer());
    })
    .catch((error) => {
      zoneFocusIndexPromise = null;
      throw error;
    });
  return zoneFocusIndexPromise;
}

async function loadFishEvidenceSnapshot(locationLike = globalThis.window?.location) {
  if (fishEvidenceSnapshotPromise) {
    return fishEvidenceSnapshotPromise;
  }
  if (typeof globalThis.fetch !== "function") {
    throw new Error("fetch() is unavailable for fish evidence focus data.");
  }
  const apiBaseUrl = resolveApiBaseUrl(locationLike);
  const metaUrl = `${apiBaseUrl}/api/v1/events_snapshot_meta`;
  fishEvidenceSnapshotPromise = globalThis.fetch(metaUrl)
    .then(async (response) => {
      if (!response.ok) {
        throw new Error(`Failed to load fish evidence focus meta: ${metaUrl}`);
      }
      return response.json();
    })
    .then(async (meta) => {
      const snapshotUrl = new URL(
        String(meta?.snapshot_url || `/api/v1/events_snapshot?revision=${encodeURIComponent(String(meta?.revision || ""))}`),
        apiBaseUrl,
      ).toString();
      const response = await globalThis.fetch(snapshotUrl);
      if (!response.ok) {
        throw new Error(`Failed to load fish evidence focus data: ${snapshotUrl}`);
      }
      return response.json();
    })
    .catch((error) => {
      fishEvidenceSnapshotPromise = null;
      throw error;
    });
  return fishEvidenceSnapshotPromise;
}

function scheduleWaypointFocusIndexPreload(locationLike = globalThis.window?.location) {
  if (waypointFocusIndexPromise || waypointFocusPreloadScheduled) {
    return;
  }
  waypointFocusPreloadScheduled = true;
  const run = () => {
    waypointFocusPreloadScheduled = false;
    void Promise.all([
      loadWaypointFocusIndex(locationLike),
      loadZoneFocusIndex(locationLike),
      loadFishEvidenceSnapshot(locationLike),
    ]).catch((error) => {
      console.warn("Failed to preload focus data", error);
    });
  };
  if (typeof globalThis.queueMicrotask === "function") {
    globalThis.queueMicrotask(run);
    return;
  }
  if (typeof globalThis.setTimeout === "function") {
    globalThis.setTimeout(run, 0);
    return;
  }
  run();
}

export function resolveSemanticIdentityFocusPoints(identityInput, focusIndex) {
  const identity =
    typeof identityInput === "string" ? parseSemanticIdentityText(identityInput) : identityInput;
  if (!identity || !focusIndex) {
    return [];
  }
  const numericId = Number.parseInt(String(identity.code || "").replace(/^(?:RG|R|N)/, ""), 10);
  if (!Number.isFinite(numericId)) {
    return [];
  }
  if (identity.kind === "N") {
    return dedupeWorldPoints([focusIndex.waypointById?.get(numericId)]);
  }
  if (identity.kind === "R") {
    const region = focusIndex.regionById?.get(numericId);
    return dedupeWorldPoints([
      focusIndex.regionNodeByRegionId?.get(numericId),
      region?.originPoint,
      region?.resourcePoint,
    ]);
  }
  if (identity.kind === "RG") {
    const regionGroup = focusIndex.regionGroupById?.get(numericId);
    const memberPoints = Array.isArray(regionGroup?.memberRegionIds)
      ? regionGroup.memberRegionIds.map((regionId) =>
          focusIndex.regionNodeByRegionId?.get(regionId),
        )
      : [];
    return dedupeWorldPoints([regionGroup?.point, ...memberPoints]);
  }
  return [];
}

function resolveSemanticIdentityFocusRect(identityInput, focusIndex, viewportInput) {
  const identity =
    typeof identityInput === "string" ? parseSemanticIdentityText(identityInput) : identityInput;
  if (!identity || !focusIndex) {
    return null;
  }
  const numericId = Number.parseInt(String(identity.code || "").replace(/^(?:RG|R|N)/, ""), 10);
  if (!Number.isFinite(numericId)) {
    return null;
  }
  if (identity.kind === "R") {
    return buildFocusWorldRectFromBaseRect(
      focusIndex.regionById?.get(numericId)?.bounds,
      viewportInput,
    );
  }
  if (identity.kind === "RG") {
    return buildFocusWorldRectFromBaseRect(
      focusIndex.regionGroupById?.get(numericId)?.bounds,
      viewportInput,
    );
  }
  return null;
}

export function buildSemanticIdentityCommand(
  identityInput,
  focusIndex,
  stateBundle,
  viewportInput,
  options = {},
) {
  const identity =
    typeof identityInput === "string" ? parseSemanticIdentityText(identityInput) : identityInput;
  if (!identity) {
    return null;
  }
  const numericId = Number.parseInt(String(identity.code || "").replace(/^(?:RG|R|N)/, ""), 10);
  if (!Number.isFinite(numericId)) {
    return null;
  }
  const autoAdjustView = options.autoAdjustView !== false;
  const focusPoints = resolveSemanticIdentityFocusPoints(identity, focusIndex);
  let command = null;
  if (identity.kind === "R") {
    command = {
      selectSemanticField: {
        layerId: "regions",
        fieldId: numericId,
      },
    };
  } else if (identity.kind === "RG") {
    command = {
      selectSemanticField: {
        layerId: "region_groups",
        fieldId: numericId,
      },
    };
  } else if (identity.kind === "N") {
    const point = focusPoints[0];
    if (!point) {
      return null;
    }
    command = buildSelectWorldPointCommand(point.worldX, point.worldZ, {
      pointKind: "waypoint",
      pointLabel: identity.name ? `${identity.name} (${identity.code})` : identity.code,
    });
  }
  if (!command) {
    return null;
  }
  if (!autoAdjustView) {
    return command;
  }
  const rect =
    resolveSemanticIdentityFocusRect(identity, focusIndex, viewportInput) ||
    buildFocusWorldRect(focusPoints, viewportInput);
  const restoreView = buildRestoreViewForWorldRect(rect, viewportInput, stateBundle);
  return restoreView
    ? {
        ...command,
        restoreView,
      }
    : command;
}

export function buildZoneIdentityCommand(
  zoneRgbInput,
  zoneFocusIndex,
  stateBundle,
  viewportInput,
  options = {},
) {
  const zoneRgb = Number.parseInt(zoneRgbInput, 10);
  if (!Number.isFinite(zoneRgb)) {
    return null;
  }
  const command = { selectZoneRgb: zoneRgb };
  if (options.autoAdjustView === false) {
    return command;
  }
  const rect = zoneFocusIndex?.zoneRectByRgb?.get(zoneRgb) || null;
  const restoreView = buildRestoreViewForWorldRect(rect, viewportInput, stateBundle);
  return restoreView ? { ...command, restoreView } : command;
}

function resolvePatchRangeEventTimeBounds(stateBundle) {
  const patchRange = normalizePatchRangeSelection(
    stateBundle?.state?.catalog?.patches || [],
    stateBundle?.inputState?.filters?.fromPatchId ??
      stateBundle?.state?.filters?.fromPatchId ??
      stateBundle?.inputState?.filters?.patchId ??
      stateBundle?.state?.filters?.patchId ??
      null,
    stateBundle?.inputState?.filters?.toPatchId ??
      stateBundle?.state?.filters?.toPatchId ??
      stateBundle?.inputState?.filters?.patchId ??
      stateBundle?.state?.filters?.patchId ??
      null,
  );
  const ordered = Array.isArray(patchRange?.ordered) ? patchRange.ordered : [];
  if (!ordered.length) {
    return { minTsUtc: -Infinity, maxTsUtc: Infinity };
  }
  const fromIndex = ordered.findIndex((patch) => patch?.patchId === patchRange.fromPatchId);
  const toIndex = ordered.findIndex((patch) => patch?.patchId === patchRange.toPatchId);
  const startIndex = fromIndex >= 0 ? fromIndex : 0;
  const endIndex = toIndex >= 0 ? toIndex : ordered.length - 1;
  const minTsUtc = Number(ordered[startIndex]?.startTsUtc);
  const nextPatch = ordered[endIndex + 1] || null;
  const nextStartTsUtc = Number(nextPatch?.startTsUtc);
  return {
    minTsUtc: Number.isFinite(minTsUtc) ? minTsUtc : -Infinity,
    maxTsUtc: Number.isFinite(nextStartTsUtc) ? nextStartTsUtc - 1 : Infinity,
  };
}

function fishEvidencePointFromEvent(event) {
  const worldPoint = normalizeWorldPoint({
    worldX: event?.world_x ?? event?.worldX,
    worldZ: event?.world_z ?? event?.worldZ,
  });
  if (worldPoint) {
    return worldPoint;
  }
  return pixelToWorldPoint(event?.map_px_x ?? event?.mapPxX, event?.map_px_y ?? event?.mapPxY);
}

function fishEvidenceBucketKey(event, bucketPx = 96) {
  const pxX = Number(event?.map_px_x ?? event?.mapPxX);
  const pxY = Number(event?.map_px_y ?? event?.mapPxY);
  if (!Number.isFinite(pxX) || !Number.isFinite(pxY)) {
    return "";
  }
  const bucketX = Math.floor(pxX / bucketPx);
  const bucketY = Math.floor(pxY / bucketPx);
  return `${bucketX}:${bucketY}`;
}

export function buildFishEvidenceFocusCommand(
  fishIdInput,
  snapshot,
  stateBundle,
  viewportInput,
  options = {},
) {
  if (options.autoAdjustView === false) {
    return null;
  }
  const fishId = Number.parseInt(fishIdInput, 10);
  if (!Number.isFinite(fishId)) {
    return null;
  }
  const events = Array.isArray(snapshot?.events) ? snapshot.events : [];
  if (!events.length) {
    return null;
  }
  const timeBounds = resolvePatchRangeEventTimeBounds(stateBundle);
  const buckets = new Map();
  for (const event of events) {
    if (Number.parseInt(event?.fish_id ?? event?.fishId, 10) !== fishId) {
      continue;
    }
    const tsUtc = Number(event?.ts_utc ?? event?.tsUtc);
    if (Number.isFinite(timeBounds.minTsUtc) && tsUtc < timeBounds.minTsUtc) {
      continue;
    }
    if (Number.isFinite(timeBounds.maxTsUtc) && tsUtc > timeBounds.maxTsUtc) {
      continue;
    }
    const point = fishEvidencePointFromEvent(event);
    if (!point) {
      continue;
    }
    const bucketKey = fishEvidenceBucketKey(event);
    if (!bucketKey) {
      continue;
    }
    let bucket = buckets.get(bucketKey);
    if (!bucket) {
      bucket = { count: 0, points: [] };
      buckets.set(bucketKey, bucket);
    }
    bucket.count += 1;
    bucket.points.push(point);
  }
  let bestBucket = null;
  for (const bucket of buckets.values()) {
    if (!bestBucket || bucket.count > bestBucket.count) {
      bestBucket = bucket;
    }
  }
  if (!bestBucket || !bestBucket.points.length) {
    return null;
  }
  const rect = buildFocusWorldRect(bestBucket.points, viewportInput);
  const restoreView = buildRestoreViewForWorldRect(rect, viewportInput, stateBundle);
  return restoreView ? { restoreView } : null;
}

function buildFocusCommandForWorldPoint(
  worldX,
  worldZ,
  stateBundle,
  viewportInput,
  options = {},
) {
  const command = buildSelectWorldPointCommand(worldX, worldZ, options);
  if (!command) {
    return null;
  }
  if (options.autoAdjustView === false) {
    return command;
  }
  const rect = buildFocusWorldRect([{ worldX, worldZ }], viewportInput);
  const restoreView = buildRestoreViewForWorldRect(rect, viewportInput, stateBundle);
  return restoreView
    ? {
        ...command,
        restoreView,
      }
    : command;
}

function formatBookmarkClipboardText(bookmarks, options = {}) {
  return serializeBookmarksForExport(bookmarks, options);
}

function pluralizeBookmarks(count) {
  return count === 1 ? "bookmark" : "bookmarks";
}

function formatBookmarkExportTimestamp(timestamp) {
  const date = new Date(timestamp);
  const year = date.getUTCFullYear();
  const month = String(date.getUTCMonth() + 1).padStart(2, "0");
  const day = String(date.getUTCDate()).padStart(2, "0");
  const hours = String(date.getUTCHours()).padStart(2, "0");
  const minutes = String(date.getUTCMinutes()).padStart(2, "0");
  const seconds = String(date.getUTCSeconds()).padStart(2, "0");
  return `${year}${month}${day}-${hours}${minutes}${seconds}`;
}

function buildBookmarkExportFilename(timestamp = Date.now()) {
  return `fishystuff-map-bookmarks-${formatBookmarkExportTimestamp(timestamp)}.xml`;
}

function createBookmarkId() {
  if (typeof globalThis.crypto?.randomUUID === "function") {
    return globalThis.crypto.randomUUID();
  }
  return `bookmark-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

function defaultBookmarkLabel(index, preferredName = "") {
  const normalizedPreferredName = String(preferredName || "").trim();
  if (normalizedPreferredName) {
    return normalizedPreferredName;
  }
  return `Bookmark ${index + 1}`;
}

function normalizeBookmarkLayerSamples(layerSamplesInput) {
  return (Array.isArray(layerSamplesInput) ? layerSamplesInput : []).filter((sample) =>
    Boolean(String(sample?.layerId || "").trim()),
  );
}

function resolveZoneCatalog(stateBundle) {
  return Array.isArray(stateBundle?.zoneCatalog) ? stateBundle.zoneCatalog : currentZoneCatalog;
}

function bookmarkPrimaryFactValue(layerSamplesInput, stateBundle = null) {
  const rows = overviewRowsForLayerSamples(layerSamplesInput, stateBundle);
  for (const key of PRIMARY_SEMANTIC_ROW_KEYS) {
    const row = rows.find((entry) => entry.key === key);
    if (row?.value) {
      return row.value;
    }
  }
  return rows[0]?.value || "";
}

export function normalizeBookmarks(rawBookmarks, stateBundle = null) {
  const entries = Array.isArray(rawBookmarks)
    ? rawBookmarks
    : Array.isArray(rawBookmarks?.bookmarks)
      ? rawBookmarks.bookmarks
      : [];
  const normalized = [];
  const seen = new Set();
  for (const entry of entries) {
    const worldX = normalizeBookmarkCoordinate(entry?.worldX);
    const worldZ = normalizeBookmarkCoordinate(entry?.worldZ);
    const id = String(entry?.id || "").trim();
    if (!id || worldX == null || worldZ == null || seen.has(id)) {
      continue;
    }
    seen.add(id);
    const layerSamples = normalizeBookmarkLayerSamples(entry?.layerSamples);
    const preferredName = bookmarkPrimaryFactValue(layerSamples, stateBundle);
    const zoneRgb = Number.parseInt(entry?.zoneRgb, 10);
    const createdAt = String(entry?.createdAt || "").trim();
    normalized.push({
      id,
      label: String(entry?.label || "").trim() || defaultBookmarkLabel(normalized.length, preferredName),
      worldX,
      worldZ,
      ...(layerSamples.length ? { layerSamples } : {}),
      zoneRgb: Number.isFinite(zoneRgb) ? zoneRgb : null,
      createdAt: createdAt || null,
    });
  }
  return normalized;
}

export function createBookmarkFromPlacement(
  placement,
  existingBookmarks = [],
  options = {},
) {
  const worldX = normalizeBookmarkCoordinate(placement?.worldX);
  const worldZ = normalizeBookmarkCoordinate(placement?.worldZ);
  if (worldX == null || worldZ == null) {
    return null;
  }
  const layerSamples = normalizeBookmarkLayerSamples(placement?.layerSamples);
  const zoneRgb = Number.parseInt(placement?.zoneRgb, 10);
  const now = Number.isFinite(options.now) ? options.now : Date.now();
  return {
    id: typeof options.idFactory === "function" ? options.idFactory() : createBookmarkId(),
    label: defaultBookmarkLabel(
      existingBookmarks.length,
      bookmarkPrimaryFactValue(layerSamples, options.stateBundle || null),
    ),
    worldX,
    worldZ,
    ...(layerSamples.length ? { layerSamples } : {}),
    zoneRgb: Number.isFinite(zoneRgb) ? zoneRgb : null,
    createdAt: new Date(now).toISOString(),
  };
}

export function renameBookmark(bookmarks, bookmarkId, nextLabel, options = {}) {
  const targetId = String(bookmarkId || "").trim();
  if (!targetId) {
    return normalizeBookmarks(bookmarks);
  }
  const normalizedBookmarks = normalizeBookmarks(bookmarks);
  const requestedLabel = String(nextLabel ?? "").trim();
  return normalizedBookmarks.map((bookmark, index) => {
    if (bookmark.id !== targetId) {
      return bookmark;
    }
    return {
      ...bookmark,
      label:
        requestedLabel ||
        defaultBookmarkLabel(
          index,
          bookmarkPrimaryFactValue(bookmark?.layerSamples, options.stateBundle || null),
        ),
    };
  });
}

function escapeXml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

function unescapeXml(value) {
  return String(value ?? "")
    .replaceAll("&quot;", '"')
    .replaceAll("&apos;", "'")
    .replaceAll("&gt;", ">")
    .replaceAll("&lt;", "<")
    .replaceAll("&amp;", "&");
}

function formatBookmarkXmlCoordinate(value) {
  const normalized = normalizeBookmarkCoordinate(value);
  if (normalized == null) {
    return "";
  }
  if (Number.isInteger(normalized)) {
    return `${normalized.toFixed(1)}`;
  }
  return String(normalized);
}

function describeBookmarksForExport(bookmarks, stateBundle = null) {
  const normalizedBookmarks = normalizeBookmarks(bookmarks);
  if (!normalizedBookmarks.length) {
    return "0 FishyStuff Bookmarks";
  }
  const semanticNames = normalizedBookmarks
    .map((bookmark) => bookmarkPrimaryFactValue(bookmark?.layerSamples, stateBundle))
    .filter(Boolean);
  if (
    semanticNames.length === normalizedBookmarks.length &&
    semanticNames.every((name) => name === semanticNames[0])
  ) {
    return semanticNames[0];
  }
  const labels = normalizedBookmarks
    .map((bookmark) => String(bookmark.label || "").trim())
    .filter(Boolean);
  if (labels.length === normalizedBookmarks.length && labels.every((name) => name === labels[0])) {
    return labels[0];
  }
  if (normalizedBookmarks.length === 1) {
    return labels[0] || semanticNames[0] || "FishyStuff Bookmark";
  }
  return `${normalizedBookmarks.length} FishyStuff Bookmarks`;
}

function bookmarkDisplayLabel(bookmark, fallbackIndex = 0, stateBundle = null) {
  return (
    String(bookmark?.label || "").trim() ||
    defaultBookmarkLabel(
      fallbackIndex,
      bookmarkPrimaryFactValue(bookmark?.layerSamples, stateBundle),
    )
  );
}

export function buildBookmarkDeletionPrompt(bookmarks, options = {}) {
  const normalizedBookmarks = normalizeBookmarks(bookmarks);
  if (!normalizedBookmarks.length) {
    return "";
  }
  if (normalizedBookmarks.length === 1) {
    return `Delete bookmark "${bookmarkDisplayLabel(normalizedBookmarks[0], 0)}"?`;
  }
  const previewLines = normalizedBookmarks
    .slice(0, 3)
    .map((bookmark, index) => `${index + 1}. ${bookmarkDisplayLabel(bookmark, index)}`);
  const remainingCount = normalizedBookmarks.length - previewLines.length;
  return [
    `Delete ${normalizedBookmarks.length} ${options.selection ? "selected " : ""}${pluralizeBookmarks(normalizedBookmarks.length)}?`,
    "",
    ...previewLines,
    remainingCount > 0 ? `...and ${remainingCount} more.` : null,
  ]
    .filter((line) => line != null)
    .join("\n");
}

function formatBookmarkXmlName(bookmark, index, stateBundle = null) {
  return `${index + 1}: ${bookmarkDisplayLabel(bookmark, index, stateBundle)}`;
}

function parseBookmarkXmlAttributes(nodeText) {
  const attributes = {};
  const attributePattern = /([A-Za-z_:][A-Za-z0-9:._-]*)\s*=\s*(?:"([^"]*)"|'([^']*)')/g;
  for (const match of String(nodeText || "").matchAll(attributePattern)) {
    attributes[match[1]] = unescapeXml(match[2] ?? match[3] ?? "");
  }
  return attributes;
}

function normalizeBookmarkLabelFromXml(label, index) {
  const trimmedLabel = String(label || "").trim().replace(/^\d+\s*:\s*/, "").trim();
  return trimmedLabel || defaultBookmarkLabel(index);
}

function bookmarkMergeKey(bookmark) {
  const normalizedBookmark = normalizeBookmarks([bookmark])[0];
  if (!normalizedBookmark) {
    return "";
  }
  return [
    String(normalizedBookmark.label || "").trim().toLowerCase(),
    formatBookmarkCoordinate(normalizedBookmark.worldX),
    formatBookmarkCoordinate(normalizedBookmark.worldZ),
  ].join("\u0000");
}

function parseXmlBookmarks(serializedBookmarks, options = {}) {
  const serialized = String(serializedBookmarks || "");
  const nodes = Array.from(serialized.matchAll(/<BookMark\b[^>]*\/?>/gi));
  if (!nodes.length) {
    return [];
  }
  const idFactory = typeof options.idFactory === "function" ? options.idFactory : createBookmarkId;
  return normalizeBookmarks(
    nodes.map((match, index) => {
      const attributes = parseBookmarkXmlAttributes(match[0]);
      const label = normalizeBookmarkLabelFromXml(attributes.BookMarkName, index);
      return {
        id: idFactory(),
        label,
        worldX: attributes.PosX,
        worldZ: attributes.PosZ,
      };
    }),
  );
}

export function serializeBookmarksForExport(bookmarks, options = {}) {
  const normalizedBookmarks = normalizeBookmarks(bookmarks);
  const title =
    String(options.title || "").trim() ||
    describeBookmarksForExport(normalizedBookmarks, options.stateBundle || null);
  const generatedBy = String(options.generatedBy || "").trim() || BOOKMARK_XML_GENERATED_BY;
  const previewUrl = String(options.previewUrl || "").trim() || BOOKMARK_XML_PREVIEW_URL;
  const posY = String(options.posY || "").trim() || BOOKMARK_XML_POS_Y;
  const lines = [
    "<!--",
    `\tWaypoints for: ${title}`,
    `\tAuto-Generated by: ${generatedBy}`,
    `\tPreview at: ${previewUrl}`,
    "-->",
    "<WorldmapBookMark>",
    ...normalizedBookmarks.map(
      (bookmark, index) =>
        `\t<BookMark BookMarkName="${escapeXml(formatBookmarkXmlName(bookmark, index, options.stateBundle || null))}" PosX="${escapeXml(formatBookmarkXmlCoordinate(bookmark.worldX))}" PosY="${escapeXml(posY)}" PosZ="${escapeXml(formatBookmarkXmlCoordinate(bookmark.worldZ))}" />`,
    ),
    "</WorldmapBookMark>",
  ];
  return lines.join("\n");
}

export function parseImportedBookmarks(serializedBookmarks, options = {}) {
  if (typeof serializedBookmarks !== "string" || !serializedBookmarks.trim()) {
    return [];
  }
  const xmlBookmarks = parseXmlBookmarks(serializedBookmarks, options);
  if (xmlBookmarks.length) {
    return xmlBookmarks;
  }
  return normalizeBookmarks(JSON.parse(serializedBookmarks));
}

export function mergeImportedBookmarks(existingBookmarks, importedBookmarks) {
  const merged = normalizeBookmarks(existingBookmarks);
  const seenKeys = new Set(merged.map((bookmark) => bookmarkMergeKey(bookmark)).filter(Boolean));
  for (const bookmark of normalizeBookmarks(importedBookmarks)) {
    const mergeKey = bookmarkMergeKey(bookmark);
    if (!mergeKey || seenKeys.has(mergeKey)) {
      continue;
    }
    seenKeys.add(mergeKey);
    merged.push(bookmark);
  }
  return merged;
}

export function moveBookmarkBefore(bookmarks, movingBookmarkId, targetBookmarkId, position = "before") {
  const normalizedBookmarks = normalizeBookmarks(bookmarks);
  const sourceId = String(movingBookmarkId || "").trim();
  const targetId = String(targetBookmarkId || "").trim();
  if (!sourceId || !targetId || sourceId === targetId) {
    return normalizedBookmarks;
  }
  const currentIndex = normalizedBookmarks.findIndex((bookmark) => bookmark.id === sourceId);
  const targetIndex = normalizedBookmarks.findIndex((bookmark) => bookmark.id === targetId);
  if (currentIndex < 0 || targetIndex < 0) {
    return normalizedBookmarks;
  }
  const reordered = normalizedBookmarks.slice();
  const [bookmark] = reordered.splice(currentIndex, 1);
  const baseIndex = reordered.findIndex((candidate) => candidate.id === targetId);
  const nextIndex = position === "after" ? baseIndex + 1 : baseIndex;
  reordered.splice(nextIndex, 0, bookmark);
  return reordered;
}

export function computeDragAutoScrollDelta(containerRect, pointerClientY, options = {}) {
  const top = Number(containerRect?.top);
  const bottom = Number(containerRect?.bottom);
  const clientY = Number(pointerClientY);
  if (!Number.isFinite(top) || !Number.isFinite(bottom) || !Number.isFinite(clientY) || bottom <= top) {
    return 0;
  }
  const edgeThreshold = Math.max(
    16,
    Number.isFinite(options.edgeThreshold) ? Number(options.edgeThreshold) : DRAG_AUTOSCROLL_EDGE_PX,
  );
  const maxStep = Math.max(
    4,
    Number.isFinite(options.maxStep) ? Number(options.maxStep) : DRAG_AUTOSCROLL_MAX_STEP_PX,
  );
  const topDistance = clientY - top;
  if (topDistance >= -edgeThreshold && topDistance <= edgeThreshold) {
    const intensity = 1 - Math.abs(topDistance) / edgeThreshold;
    return -Math.max(1, Math.round(maxStep * intensity));
  }
  const bottomDistance = bottom - clientY;
  if (bottomDistance >= -edgeThreshold && bottomDistance <= edgeThreshold) {
    const intensity = 1 - Math.abs(bottomDistance) / edgeThreshold;
    return Math.max(1, Math.round(maxStep * intensity));
  }
  return 0;
}

function downloadBookmarkExport(bookmarks, options = {}) {
  const doc = options.document ?? globalThis.document;
  const urlApi = options.url ?? globalThis.URL;
  const blobCtor = options.Blob ?? globalThis.Blob;
  if (
    !doc?.createElement ||
    !doc?.body?.appendChild ||
    typeof blobCtor !== "function" ||
    typeof urlApi?.createObjectURL !== "function"
  ) {
    throw new Error("Bookmark export is unavailable");
  }
  const timestamp = Number.isFinite(options.now) ? options.now : Date.now();
  const anchor = doc.createElement("a");
  const href = urlApi.createObjectURL(
    new blobCtor([serializeBookmarksForExport(bookmarks, { now: timestamp, stateBundle: options.stateBundle || null })], {
      type: "application/xml",
    }),
  );
  anchor.href = href;
  anchor.download = buildBookmarkExportFilename(timestamp);
  anchor.rel = "noopener";
  anchor.hidden = true;
  doc.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  globalThis.setTimeout?.(() => {
    urlApi.revokeObjectURL?.(href);
  }, 0);
}

async function readBookmarkImportFile(file) {
  if (typeof file?.text === "function") {
    return file.text();
  }
  const readerCtor = globalThis.FileReader;
  if (typeof readerCtor !== "function") {
    throw new Error("Bookmark import is unavailable");
  }
  return new Promise((resolve, reject) => {
    const reader = new readerCtor();
    reader.onerror = () => reject(reader.error || new Error("Failed to read bookmark import"));
    reader.onload = () => resolve(String(reader.result ?? ""));
    reader.readAsText(file);
  });
}

async function copyTextToClipboard(text) {
  if (globalThis.navigator?.clipboard?.writeText) {
    await globalThis.navigator.clipboard.writeText(text);
    return;
  }
  const doc = globalThis.document;
  if (!doc?.createElement || !doc?.body?.appendChild) {
    throw new Error("Clipboard API unavailable");
  }
  const probe = doc.createElement("textarea");
  probe.value = String(text ?? "");
  probe.setAttribute("readonly", "");
  probe.style.position = "fixed";
  probe.style.opacity = "0";
  probe.style.pointerEvents = "none";
  doc.body.appendChild(probe);
  probe.select();
  probe.setSelectionRange(0, probe.value.length);
  const copied = doc.execCommand?.("copy");
  probe.remove();
  if (!copied) {
    throw new Error("Clipboard API unavailable");
  }
}

function showSiteToast(tone, message, options = {}) {
  const text = String(message || "").trim();
  if (!text) {
    return;
  }
  const toast = globalThis.__fishystuffToast;
  if (!toast) {
    return;
  }
  const handler =
    typeof toast[tone] === "function"
      ? toast[tone]
      : typeof toast.show === "function"
        ? (value, extra) => toast.show({ tone, message: value, ...(extra || {}) })
        : null;
  handler?.(text, options);
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

function parseCatalogInteger(value) {
  const number = Number.parseInt(value, 10);
  return Number.isFinite(number) ? number : null;
}

function parseCatalogBoolean(value) {
  if (value === true || value === false) {
    return value;
  }
  if (value === 1 || value === 0) {
    return value === 1;
  }
  const normalized = String(value ?? "").trim().toLowerCase();
  if (normalized === "true" || normalized === "1") {
    return true;
  }
  if (normalized === "false" || normalized === "0") {
    return false;
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
    const confirmed = parseCatalogBoolean(entry?.confirmed);
    const active = parseCatalogBoolean(entry?.active);
    const order = parseCatalogInteger(entry?.order ?? entry?.index);
    const biteTimeMin = parseCatalogInteger(entry?.biteTimeMin ?? entry?.bite_time_min);
    const biteTimeMax = parseCatalogInteger(entry?.biteTimeMax ?? entry?.bite_time_max);
    normalized.push({
      kind: "zone",
      zoneRgb,
      r,
      g,
      b,
      name,
      confirmed: confirmed === true,
      active,
      order: Number.isFinite(order) ? order : Number.MAX_SAFE_INTEGER,
      biteTimeMin,
      biteTimeMax,
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
  currentZoneCatalog = normalized;
  return normalized;
}

async function loadZoneCatalog(
  fetchImpl = globalThis.fetch,
  locationLike = globalThis.window?.location,
) {
  if (typeof fetchImpl !== "function") {
    return [];
  }
  const apiBaseUrl = resolveApiBaseUrl(locationLike);
  const url = `${apiBaseUrl}${DEFAULT_ZONE_CATALOG_PATH}`;
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

function renderSearchableDropdownTextContent(label) {
  return `<span class="truncate font-medium">${escapeHtml(label)}</span>`;
}

function renderPatchDropdownContentHtml(name, dateLabel) {
  if (!dateLabel) {
    return renderSearchableDropdownTextContent(name);
  }
  return `
    <span class="grid min-w-0 gap-0.5">
      <span class="truncate font-medium">${escapeHtml(name)}</span>
      <span class="truncate text-xs text-base-content/55">${escapeHtml(dateLabel)}</span>
    </span>
  `;
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
  return `/images/items/${digits}.webp`;
}

function fishEncyclopediaIconPath(encyclopediaId) {
  const numeric = Number.parseInt(encyclopediaId, 10);
  if (!Number.isFinite(numeric) || numeric <= 0) {
    return "";
  }
  return `/images/items/IC_0${numeric}.webp`;
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

function buildLayerPointIconsPatch(stateBundle, targetLayerId, visible) {
  const next = {};
  for (const layer of resolveLayerEntries(stateBundle)) {
    if (!layer.supportsPointIcons) {
      continue;
    }
    const effectiveVisible =
      layer.layerId === targetLayerId ? visible !== false : layer.pointIconsVisible !== false;
    if (effectiveVisible === (layer.pointIconsDefault !== false)) {
      continue;
    }
    next[layer.layerId] = effectiveVisible;
  }
  return next;
}

function buildLayerPointIconScalePatch(stateBundle, targetLayerId, scale) {
  const next = {};
  for (const layer of resolveLayerEntries(stateBundle)) {
    if (!layer.supportsPointIcons) {
      continue;
    }
    const effectiveScale =
      layer.layerId === targetLayerId
        ? clampPointIconScale(scale)
        : clampPointIconScale(layer.pointIconScale);
    if (Math.abs(effectiveScale - clampPointIconScale(layer.pointIconScaleDefault)) <= 0.0001) {
      continue;
    }
    next[layer.layerId] = effectiveScale;
  }
  return next;
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
  if (kind === "fish-evidence") {
    return "Evidence";
  }
  if (kind === "vector-geojson") {
    return "Vector";
  }
  if (kind === "waypoints") {
    return "Waypoints";
  }
  if (kind === "tiled-raster") {
    return "Raster";
  }
  return "Layer";
}

function spriteIcon(name, sizeClass = "size-5") {
  return `<svg class="fishy-icon ${sizeClass}" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="${ICON_SPRITE_URL}#fishy-${name}"></use></svg>`;
}

function fishFilterTermIconMarkup(term, sizeClass = "size-4") {
  const metadata = FISH_FILTER_TERM_METADATA[normalizeFishFilterTerm(term)] || null;
  return spriteIcon(
    metadata?.icon || "question-mark",
    `${sizeClass} shrink-0 ${metadata?.iconClass || "text-base-content/60"}`.trim(),
  );
}

function dragHandleIcon() {
  return spriteIcon("drag-handle");
}

function layerSettingsIcon() {
  return spriteIcon("settings-1");
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
  const inputWaypointConnectionsOverride = isPlainObject(
    stateBundle.inputState?.filters?.layerWaypointConnectionsVisible,
  )
    ? stateBundle.inputState.filters.layerWaypointConnectionsVisible
    : null;
  const stateWaypointConnectionsOverride = isPlainObject(
    stateBundle.state?.filters?.layerWaypointConnectionsVisible,
  )
    ? stateBundle.state.filters.layerWaypointConnectionsVisible
    : null;
  const inputWaypointLabelsOverride = isPlainObject(
    stateBundle.inputState?.filters?.layerWaypointLabelsVisible,
  )
    ? stateBundle.inputState.filters.layerWaypointLabelsVisible
    : null;
  const stateWaypointLabelsOverride = isPlainObject(
    stateBundle.state?.filters?.layerWaypointLabelsVisible,
  )
    ? stateBundle.state.filters.layerWaypointLabelsVisible
    : null;
  const inputPointIconsOverride = isPlainObject(
    stateBundle.inputState?.filters?.layerPointIconsVisible,
  )
    ? stateBundle.inputState.filters.layerPointIconsVisible
    : null;
  const statePointIconsOverride = isPlainObject(
    stateBundle.state?.filters?.layerPointIconsVisible,
  )
    ? stateBundle.state.filters.layerPointIconsVisible
    : null;
  const inputPointIconScaleOverride = isPlainObject(
    stateBundle.inputState?.filters?.layerPointIconScales,
  )
    ? stateBundle.inputState.filters.layerPointIconScales
    : null;
  const statePointIconScaleOverride = isPlainObject(
    stateBundle.state?.filters?.layerPointIconScales,
  )
    ? stateBundle.state.filters.layerPointIconScales
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
    const supportsWaypointConnections = layer.supportsWaypointConnections === true;
    const waypointConnectionsDefault = supportsWaypointConnections
      ? layer.waypointConnectionsDefault !== false
      : false;
    let waypointConnectionsVisible = supportsWaypointConnections
      ? layer.waypointConnectionsVisible !== false
      : false;
    if (supportsWaypointConnections && inputWaypointConnectionsOverride) {
      waypointConnectionsVisible = hasOwnKey(inputWaypointConnectionsOverride, layer.layerId)
        ? inputWaypointConnectionsOverride[layer.layerId] !== false
        : waypointConnectionsDefault;
    } else if (
      supportsWaypointConnections &&
      stateWaypointConnectionsOverride &&
      hasOwnKey(stateWaypointConnectionsOverride, layer.layerId)
    ) {
      waypointConnectionsVisible = stateWaypointConnectionsOverride[layer.layerId] !== false;
    }
    const supportsWaypointLabels = layer.supportsWaypointLabels === true;
    const waypointLabelsDefault = supportsWaypointLabels
      ? layer.waypointLabelsDefault !== false
      : false;
    let waypointLabelsVisible = supportsWaypointLabels
      ? layer.waypointLabelsVisible !== false
      : false;
    if (supportsWaypointLabels && inputWaypointLabelsOverride) {
      waypointLabelsVisible = hasOwnKey(inputWaypointLabelsOverride, layer.layerId)
        ? inputWaypointLabelsOverride[layer.layerId] !== false
        : waypointLabelsDefault;
    } else if (
      supportsWaypointLabels &&
      stateWaypointLabelsOverride &&
      hasOwnKey(stateWaypointLabelsOverride, layer.layerId)
    ) {
      waypointLabelsVisible = stateWaypointLabelsOverride[layer.layerId] !== false;
    }
    const supportsPointIcons = layer.supportsPointIcons === true;
    const pointIconsDefault = supportsPointIcons ? layer.pointIconsDefault !== false : false;
    let pointIconsVisible = supportsPointIcons ? layer.pointIconsVisible !== false : false;
    if (supportsPointIcons && inputPointIconsOverride) {
      pointIconsVisible = hasOwnKey(inputPointIconsOverride, layer.layerId)
        ? inputPointIconsOverride[layer.layerId] !== false
        : pointIconsDefault;
    } else if (
      supportsPointIcons &&
      statePointIconsOverride &&
      hasOwnKey(statePointIconsOverride, layer.layerId)
    ) {
      pointIconsVisible = statePointIconsOverride[layer.layerId] !== false;
    }
    const pointIconScaleDefault = supportsPointIcons
      ? clampPointIconScale(layer.pointIconScaleDefault ?? FISHYMAP_POINT_ICON_SCALE_MIN)
      : FISHYMAP_POINT_ICON_SCALE_MIN;
    let pointIconScale = supportsPointIcons
      ? clampPointIconScale(layer.pointIconScale ?? pointIconScaleDefault)
      : FISHYMAP_POINT_ICON_SCALE_MIN;
    if (supportsPointIcons && inputPointIconScaleOverride) {
      pointIconScale = hasOwnKey(inputPointIconScaleOverride, layer.layerId)
        ? clampPointIconScale(inputPointIconScaleOverride[layer.layerId])
        : pointIconScaleDefault;
    } else if (
      supportsPointIcons &&
      statePointIconScaleOverride &&
      hasOwnKey(statePointIconScaleOverride, layer.layerId)
    ) {
      pointIconScale = clampPointIconScale(statePointIconScaleOverride[layer.layerId]);
    }
    const entry = {
      ...layer,
      visible,
      opacity,
      opacityDefault,
      clipMaskLayerId,
      supportsWaypointConnections,
      waypointConnectionsVisible,
      waypointConnectionsDefault,
      supportsWaypointLabels,
      waypointLabelsVisible,
      waypointLabelsDefault,
      supportsPointIcons,
      pointIconsVisible,
      pointIconsDefault,
      pointIconScale,
      pointIconScaleDefault,
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

function buildLayerWaypointConnectionsPatch(stateBundle, targetLayerId, visible) {
  const next = {};
  for (const layer of resolveLayerEntries(stateBundle)) {
    if (!layer.supportsWaypointConnections) {
      continue;
    }
    const effectiveVisible =
      layer.layerId === targetLayerId ? visible !== false : layer.waypointConnectionsVisible !== false;
    if (effectiveVisible === (layer.waypointConnectionsDefault !== false)) {
      continue;
    }
    next[layer.layerId] = effectiveVisible;
  }
  return next;
}

function buildLayerWaypointLabelsPatch(stateBundle, targetLayerId, visible) {
  const next = {};
  for (const layer of resolveLayerEntries(stateBundle)) {
    if (!layer.supportsWaypointLabels) {
      continue;
    }
    const effectiveVisible =
      layer.layerId === targetLayerId ? visible !== false : layer.waypointLabelsVisible !== false;
    if (effectiveVisible === (layer.waypointLabelsDefault !== false)) {
      continue;
    }
    next[layer.layerId] = effectiveVisible;
  }
  return next;
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

function overviewRowMarkup(row, iconSizeClass = "size-4") {
  const label = String(row?.label || "").trim();
  const value = String(row?.value || "").trim();
  const icon = String(row?.icon || "").trim();
  const statusIcon = String(row?.statusIcon || "").trim();
  const statusIconTone = String(row?.statusIconTone || "").trim();
  const hideLabel = row?.hideLabel === true;
  if ((!label && !hideLabel) || !value || !icon) {
    return "";
  }
  const valueMarkup =
    semanticIdentityMarkup(value, { interactive: true }) ||
    `<span class="fishymap-overview-value-text">${escapeHtml(value)}</span>`;
  return `
    <div class="fishymap-overview-row${hideLabel ? " fishymap-overview-row--label-less" : ""}">
      <span class="fishymap-overview-icon" aria-hidden="true">${spriteIcon(icon, iconSizeClass)}</span>
      ${hideLabel ? "" : `<span class="fishymap-overview-label">${escapeHtml(label)}</span>`}
      <span class="fishymap-overview-value">
        ${valueMarkup}
        ${
          statusIcon
            ? `<span class="fishymap-overview-status${
                statusIconTone === "subtle" ? " fishymap-overview-status--subtle" : ""
              }" aria-hidden="true">${spriteIcon(statusIcon, "size-4")}</span>`
            : ""
        }
      </span>
    </div>
  `;
}

function parseSemanticIdentityText(text) {
  const trimmed = String(text || "").trim();
  if (!trimmed) {
    return null;
  }
  const namedMatch = trimmed.match(/^(?:(.+?):\s+)?(.+?)\s+\((N|RG|R)(\d+)\)$/);
  if (namedMatch) {
    const [, rawPrefix = "", rawName = "", kind = "", id = ""] = namedMatch;
    const prefix = String(rawPrefix).trim();
    const name = String(rawName).trim();
    const code = `${kind}${id}`;
    if (!code || !name) {
      return null;
    }
    return { prefix, code, name, kind };
  }
  const idOnlyMatch = trimmed.match(/^(?:(.+?):\s+)?((?:N|RG|R)\d+)$/);
  if (!idOnlyMatch) {
    return null;
  }
  const [, rawPrefix = "", code = ""] = idOnlyMatch;
  const prefix = String(rawPrefix).trim();
  const normalizedCode = String(code).trim();
  const kind = normalizedCode.startsWith("RG")
    ? "RG"
    : normalizedCode.startsWith("R")
      ? "R"
      : normalizedCode.startsWith("N")
        ? "N"
        : "";
  if (!kind) {
    return null;
  }
  return { prefix, code: normalizedCode, name: "", kind };
}

function zoneIdentityMarkup(zoneInput, options = {}) {
  const zoneRgb = Number.parseInt(zoneInput?.zoneRgb ?? zoneInput, 10);
  if (!Number.isFinite(zoneRgb)) {
    return "";
  }
  const name = String(zoneInput?.name || "").trim() || `Zone ${formatZone(zoneRgb)}`;
  const rgb = zoneRgbTriplet(zoneRgb);
  if (!rgb) {
    return "";
  }
  const swatch = `rgb(${rgb.r}, ${rgb.g}, ${rgb.b})`;
  const body = `
    <span
      class="inline-flex size-5 shrink-0 rounded-full border border-base-300 shadow-sm"
      style="background-color: ${escapeHtml(swatch)};"
      aria-hidden="true"
    ></span>
    <span class="truncate">${escapeHtml(name)}</span>
  `;
  if (options?.interactive !== true) {
    return `<span class="inline-flex min-w-0 items-center gap-2">${body}</span>`;
  }
  return `
    <button
      class="inline-flex min-w-0 items-center gap-2 rounded-full border border-base-300/80 bg-base-100 px-2 py-1 text-sm text-left text-base-content shadow-sm transition-colors hover:bg-base-200/70"
      type="button"
      data-zone-focus-rgb="${zoneRgb}"
      aria-label="Focus ${escapeHtml(name)}"
      title="Focus ${escapeHtml(name)}"
    >
      ${body}
    </button>
  `;
}

function zoneCatalogEntryForRgb(zoneCatalog, zoneRgbInput) {
  const zoneRgb = Number.parseInt(zoneRgbInput, 10);
  if (!Number.isFinite(zoneRgb)) {
    return null;
  }
  return (Array.isArray(zoneCatalog) ? zoneCatalog : []).find((zone) => zone.zoneRgb === zoneRgb) || null;
}

function zoneDisplayNameFromCatalog(zoneCatalog, zoneRgbInput) {
  const zoneRgb = Number.parseInt(zoneRgbInput, 10);
  if (!Number.isFinite(zoneRgb)) {
    return "";
  }
  const zone = zoneCatalogEntryForRgb(zoneCatalog, zoneRgb);
  return String(zone?.name || "").trim() || `Zone ${formatZone(zoneRgb)}`;
}

function formatZoneBiteTimeRange(zone) {
  const min = parseCatalogInteger(zone?.biteTimeMin);
  const max = parseCatalogInteger(zone?.biteTimeMax);
  if (min == null && max == null) {
    return "";
  }
  if (min != null && max != null) {
    return `${min}-${max} s`;
  }
  return `${min ?? max} s`;
}

function zoneRgbFromSample(sample) {
  if (Number.isFinite(sample?.rgbU32)) {
    return sample.rgbU32;
  }
  const rgb = sample?.rgb;
  if (rgb && Number.isFinite(rgb?.r) && Number.isFinite(rgb?.g) && Number.isFinite(rgb?.b)) {
    return rgbTripletToU32(rgb.r, rgb.g, rgb.b);
  }
  return null;
}

function buildZoneCatalogDetailSection(sample, zoneCatalog) {
  const zoneRgb = zoneRgbFromSample(sample);
  if (!Number.isFinite(zoneRgb)) {
    return null;
  }
  const zone = zoneCatalogEntryForRgb(zoneCatalog, zoneRgb);
  const rgbDisplay = String(zone?.rgbKey || "").trim() || formatZone(zoneRgb);
  const facts = [
    {
      key: "zone",
      label: "Zone",
      value: zoneDisplayNameFromCatalog(zoneCatalog, zoneRgb),
      icon: "hover-zone",
    },
    {
      key: "rgb",
      label: "RGB",
      value: rgbDisplay,
      icon: "theme-palette",
    },
  ];
  const biteTimeRange = formatZoneBiteTimeRange(zone);
  if (biteTimeRange) {
    facts.push({
      key: "bite_time",
      label: "Bite Time",
      value: biteTimeRange,
      icon: "stopwatch",
    });
  }
  return {
    id: "zone",
    kind: "facts",
    title: "Zone",
    facts,
    targets: [],
  };
}

function buildZoneCatalogOverviewRows(sample, stateBundle = null) {
  const zoneSection = buildZoneCatalogDetailSection(sample, resolveZoneCatalog(stateBundle));
  if (!zoneSection) {
    return [];
  }
  return normalizePointDetailFacts(zoneSection.facts)
    .filter((fact) => String(fact?.key || "").trim() === "zone")
    .map((fact) => ({
      key: String(fact?.key || "").trim() || "zone",
      icon: displayIconForDetailFact(fact),
      label: displayLabelForDetailFact(zoneSection, fact),
      value: String(fact?.value || "").trim(),
      ...(fact?.statusIcon ? { statusIcon: String(fact.statusIcon).trim() } : {}),
      ...(fact?.statusIconTone ? { statusIconTone: String(fact.statusIconTone).trim() } : {}),
    }))
    .filter((row) => Boolean(row.icon && row.label && row.value));
}

function fishIdentityMarkup(fishInput, options = {}) {
  const fishId = Number.parseInt(fishInput?.fishId ?? fishInput, 10);
  if (!Number.isFinite(fishId)) {
    return "";
  }
  const fish = isPlainObject(fishInput) ? fishInput : { fishId };
  const name = String(fish?.name || "").trim() || `Fish ${fishId}`;
  const body = `
    ${renderFishAvatar(fish, "size-5", { gradeFrame: true })}
    <span class="truncate max-w-40">${escapeHtml(name)}</span>
  `;
  if (options?.interactive !== true) {
    return `<span class="inline-flex min-w-0 items-center gap-2">${body}</span>`;
  }
  return `
    <button
      class="inline-flex min-w-0 items-center gap-2 rounded-full border border-base-300/80 bg-base-100 px-2 py-1 text-sm text-left text-base-content shadow-sm transition-colors hover:bg-base-200/70"
      type="button"
      data-fish-focus-id="${fishId}"
      aria-label="Focus ${escapeHtml(name)}"
      title="Focus ${escapeHtml(name)}"
    >
      ${body}
    </button>
  `;
}

function semanticKindFromIdentityKind(kind) {
  if (kind === "RG") {
    return "region-group";
  }
  if (kind === "R") {
    return "region";
  }
  if (kind === "N") {
    return "node";
  }
  return "";
}

function semanticIdentityChipMarkup(parsed, semanticKind) {
  const chipLabel = parsed.name ? `${parsed.code} ${parsed.name}` : parsed.code;
  return `
    <span class="fishymap-semantic-chip" data-semantic-kind="${semanticKind}" aria-label="${escapeHtml(chipLabel)}">
      <span class="fishymap-semantic-chip-code">${escapeHtml(parsed.code)}</span>
      ${
        parsed.name
          ? `<span class="fishymap-semantic-chip-name">${escapeHtml(parsed.name)}</span>`
          : ""
      }
    </span>
  `;
}

function semanticIdentityMarkup(text, options = {}) {
  const parsed = parseSemanticIdentityText(text);
  if (!parsed) {
    return "";
  }
  const semanticKind = semanticKindFromIdentityKind(parsed.kind);
  if (!semanticKind) {
    return "";
  }
  const prefixMarkup = parsed.prefix
    ? `<span class="fishymap-semantic-prefix">${escapeHtml(parsed.prefix)}</span>`
    : "";
  const chipMarkup = semanticIdentityChipMarkup(parsed, semanticKind);
  if (options?.interactive !== true) {
    return `
      <span class="fishymap-semantic-inline">
        ${prefixMarkup}
        ${chipMarkup}
      </span>
    `;
  }
  const buttonLabel = parsed.prefix
    ? `${parsed.prefix}: ${parsed.name ? `${parsed.code} ${parsed.name}` : parsed.code}`
    : parsed.name
      ? `${parsed.code} ${parsed.name}`
      : parsed.code;
  return `
    <span class="fishymap-semantic-inline">
      ${prefixMarkup}
      <button
        class="fishymap-semantic-button"
        type="button"
        data-semantic-focus-code="${escapeHtml(parsed.code)}"
        aria-label="${escapeHtml(buttonLabel)}"
        title="${escapeHtml(buttonLabel)}"
      >
        ${chipMarkup}
      </button>
    </span>
  `;
}

function overviewRowsForSample(sample, stateBundle = null) {
  if (String(sample?.layerId || "").trim() === "zone_mask") {
    return buildZoneCatalogOverviewRows(sample, stateBundle);
  }
  const territoryRows = territoryOverviewRowsFromSections(
    normalizePointDetailSections(sample?.detailSections),
  );
  if (territoryRows.length > 0) {
    return territoryRows;
  }
  const sections = normalizePointDetailSections(sample?.detailSections);
  const rows = [];
  for (const section of sections) {
    if (String(section?.kind || "").trim() !== "facts") {
      continue;
    }
    for (const fact of normalizePointDetailFacts(section?.facts)) {
      const factKey = String(fact?.key || "").trim();
      if (!PRIMARY_SEMANTIC_ROW_KEYS.includes(factKey)) {
        continue;
      }
      const icon = displayIconForDetailFact(fact);
      if (!icon) {
        continue;
      }
      const displayLabel = displayLabelForDetailFact(section, fact);
      rows.push({
        key:
          factKey ||
          `${String(section?.id || "section").trim()}:${String(fact?.label || "").trim().toLowerCase()}`,
        icon,
        label: displayLabel,
        value: String(fact?.value || "").trim(),
        ...(fact?.statusIcon ? { statusIcon: String(fact.statusIcon).trim() } : {}),
        ...(fact?.statusIconTone ? { statusIconTone: String(fact.statusIconTone).trim() } : {}),
      });
    }
  }
  return rows.filter((row) => Boolean(row.icon && row.label && row.value));
}

function displayLabelForDetailFact(section, fact) {
  switch (String(fact?.key || "").trim()) {
    case "zone":
      return "Zone";
    case "resources":
    case "resource_group":
    case "resource_region":
      return "Resources";
    case "origin":
    case "origin_region":
      return "Origin";
    default:
      return (
        String(fact?.label || "").trim() ||
        String(section?.title || "").trim() ||
        "Details"
      );
  }
}

function displayIconForDetailFact(fact) {
  switch (String(fact?.key || "").trim()) {
    case "resources":
    case "resource_group":
    case "resource_region":
      return "hover-resources";
    case "origin":
    case "origin_region":
      return "trade-origin";
    default:
      return String(fact?.icon || "").trim();
  }
}

function overviewRowsForLayerSamples(layerSamplesInput, stateBundle) {
  const layerSamples = Array.isArray(layerSamplesInput) ? layerSamplesInput : [];
  const sampleByLayerId = new Map(
    layerSamples
      .map((sample) => [String(sample?.layerId || "").trim(), sample])
      .filter(([layerId]) => Boolean(layerId)),
  );
  const layerIds = orderedLayerIdsForLayerSamples(layerSamples, sampleByLayerId, stateBundle);
  return layerIds.flatMap((layerId) => overviewRowsForSample(sampleByLayerId.get(layerId), stateBundle));
}

function hoverLayerOverviewRows(layerId, sampleByLayerId, stateBundle) {
  const sample = sampleByLayerId.get(layerId);
  if (!sample) {
    return [];
  }
  return overviewRowsForSample(sample, stateBundle).map((row) => ({
    layerId,
    icon: row.icon,
    label: row.label,
    value: row.value,
    ...(row.hideLabel === true ? { hideLabel: true } : {}),
    ...(row.statusIcon ? { statusIcon: row.statusIcon } : {}),
    ...(row.statusIconTone ? { statusIconTone: row.statusIconTone } : {}),
  }));
}

const BOOKMARK_HIT_RADIUS_SCREEN_PX = 14;

function bookmarkHitRadiusWorld(stateBundle) {
  const zoom = Number(stateBundle?.state?.view?.camera?.zoom);
  const normalizedZoom = Number.isFinite(zoom) && zoom > 0 ? zoom : 1;
  return BOOKMARK_HIT_RADIUS_SCREEN_PX * normalizedZoom;
}

export function resolveHoveredBookmark(hover, stateBundle, bookmarks) {
  const worldX = normalizeBookmarkCoordinate(hover?.worldX);
  const worldZ = normalizeBookmarkCoordinate(hover?.worldZ);
  const normalizedBookmarks = normalizeBookmarks(bookmarks);
  if (worldX == null || worldZ == null || normalizedBookmarks.length === 0) {
    return null;
  }
  const maxDistanceSq = bookmarkHitRadiusWorld(stateBundle) ** 2;
  let closestBookmark = null;
  for (let index = 0; index < normalizedBookmarks.length; index += 1) {
    const bookmark = normalizedBookmarks[index];
    const dx = bookmark.worldX - worldX;
    const dz = bookmark.worldZ - worldZ;
    const distanceSq = dx * dx + dz * dz;
    if (distanceSq > maxDistanceSq) {
      continue;
    }
    if (!closestBookmark || distanceSq < closestBookmark.distanceSq) {
      closestBookmark = {
        bookmark,
        index,
        distanceSq,
      };
    }
  }
  return closestBookmark
    ? {
        bookmark: closestBookmark.bookmark,
        index: closestBookmark.index,
      }
    : null;
}

export function buildHoverOverviewRows(hover, stateBundle) {
  return buildOverviewRowsForLayerSamples(hover?.layerSamples, stateBundle);
}

export function buildSelectionOverviewRows(selection, stateBundle) {
  const headingRow = preferredLayerSampleOverviewRow(selection?.layerSamples, stateBundle);
  const overviewRows = buildOverviewRowsForLayerSamples(selection?.layerSamples, stateBundle);
  if (!headingRow) {
    return overviewRows;
  }
  const layerSamples = Array.isArray(selection?.layerSamples) ? selection.layerSamples : [];
  const sampleByLayerId = new Map(
    layerSamples
      .map((sample) => [String(sample?.layerId || "").trim(), sample])
      .filter(([layerId]) => Boolean(layerId)),
  );
  const layerIds = orderedLayerIdsForLayerSamples(layerSamples, sampleByLayerId, stateBundle);
  let skippedHeading = false;
  const filteredRows = layerIds.flatMap((layerId) => {
    const sample = sampleByLayerId.get(layerId);
    if (!sample) {
      return [];
    }
    return overviewRowsForSample(sample, stateBundle).flatMap((row) => {
      const sameHeadingKey = String(headingRow?.key || "").trim() === String(row?.key || "").trim();
      const sameHeadingValue =
        String(headingRow?.value || "").trim() === String(row?.value || "").trim();
      if (!skippedHeading && sameHeadingKey && sameHeadingValue) {
        skippedHeading = true;
        return [];
      }
      return [
        {
          layerId,
          icon: row.icon,
          label: row.label,
          value: row.value,
          ...(row.hideLabel === true ? { hideLabel: true } : {}),
          ...(row.statusIcon ? { statusIcon: row.statusIcon } : {}),
          ...(row.statusIconTone ? { statusIconTone: row.statusIconTone } : {}),
        },
      ];
    });
  });
  return filteredRows.length > 0 ? filteredRows : overviewRows;
}

function selectionMatchesBookmark(selection, bookmark) {
  const worldX = normalizeBookmarkCoordinate(selection?.worldX);
  const worldZ = normalizeBookmarkCoordinate(selection?.worldZ);
  if (worldX == null || worldZ == null) {
    return false;
  }
  return (
    Math.abs(bookmark.worldX - worldX) <= 0.001 &&
    Math.abs(bookmark.worldZ - worldZ) <= 0.001
  );
}

function selectedBookmarkForSelection(selection, stateBundle) {
  if (normalizeSelectionPointKind(selection?.pointKind) !== "bookmark") {
    return null;
  }
  const bookmarks = normalizeBookmarks(
    stateBundle?.state?.ui?.bookmarks || stateBundle?.inputState?.ui?.bookmarks || [],
  );
  const selectedIds = normalizeSelectedBookmarkIds(
    bookmarks,
    stateBundle?.inputState?.ui?.bookmarkSelectedIds || stateBundle?.state?.ui?.bookmarkSelectedIds,
  );
  if (selectedIds.length !== 1) {
    return null;
  }
  const bookmark = bookmarks.find((entry) => entry.id === selectedIds[0]);
  return bookmark && selectionMatchesBookmark(selection, bookmark) ? bookmark : null;
}

export function buildSelectionSummaryText(selection, stateBundle) {
  const selectedBookmark = selectedBookmarkForSelection(selection, stateBundle);
  if (selectedBookmark) {
    return bookmarkDisplayLabel(selectedBookmark, 0, stateBundle);
  }
  const preferredValue = String(
    preferredLayerSampleOverviewRow(selection?.layerSamples, stateBundle)?.value || "",
  ).trim();
  if (preferredValue) {
    return preferredValue;
  }
  const pointLabel = String(selection?.pointLabel || "").trim();
  if (pointLabel) {
    return pointLabel;
  }
  const zoneRgb = selection?.zoneRgb ?? zoneRgbFromLayerSamples(selection?.layerSamples);
  if (zoneRgb != null) {
    return zoneDisplayNameFromCatalog(resolveZoneCatalog(stateBundle), zoneRgb);
  }
  return "No selection.";
}

function buildOverviewRowsForLayerSamples(layerSamplesInput, stateBundle) {
  const layerSamples = Array.isArray(layerSamplesInput) ? layerSamplesInput : [];
  const sampleByLayerId = new Map(
    layerSamples
      .map((sample) => [String(sample?.layerId || "").trim(), sample])
      .filter(([layerId]) => Boolean(layerId)),
  );
  const layerIds = orderedLayerIdsForLayerSamples(layerSamples, sampleByLayerId, stateBundle);
  return layerIds.flatMap((layerId) => hoverLayerOverviewRows(layerId, sampleByLayerId, stateBundle));
}

function orderedLayerIdsForLayerSamples(layerSamples, sampleByLayerId, stateBundle) {
  const orderedLayerIds = resolveLayerEntries(stateBundle || {})
    .map((layer) => String(layer?.layerId || "").trim())
    .filter((layerId) => sampleByLayerId.has(layerId))
    .reverse();
  return orderedLayerIds.length
    ? orderedLayerIds
    : layerSamples
        .map((sample) => String(sample?.layerId || "").trim())
        .filter(Boolean);
}

function preferredLayerSampleOverviewRow(layerSamplesInput, stateBundle) {
  const rows = overviewRowsForLayerSamples(layerSamplesInput, stateBundle);
  for (const key of PRIMARY_SEMANTIC_ROW_KEYS) {
    const row = rows.find((entry) => String(entry?.key || "").trim() === key);
    if (row) {
      return row;
    }
  }
  return rows[0] || null;
}

function renderHoverTooltip(elements, hover, stateBundle) {
  if (!elements.hoverTooltip || !elements.hoverLayers) {
    return;
  }
  const overviewRows = buildHoverOverviewRows(hover, stateBundle);
  if (overviewRows.length === 0 || !elements.hoverPointerActive) {
    setBooleanProperty(elements.hoverTooltip, "hidden", true);
    return;
  }
  setMarkup(
    elements.hoverLayers,
    JSON.stringify(overviewRows),
    overviewRows.map((row) => overviewRowMarkup(row)).join(""),
  );
  setBooleanProperty(elements.hoverLayers, "hidden", overviewRows.length === 0);
  setBooleanProperty(elements.hoverTooltip, "hidden", false);
}

function renderSelectionOverview(elements, selection, stateBundle) {
  if (!elements.selectionOverview) {
    return;
  }
  const overviewRows = buildSelectionOverviewRows(selection, stateBundle);
  if (overviewRows.length === 0) {
    setMarkup(elements.selectionOverview, "[]", "");
    setBooleanProperty(elements.selectionOverview, "hidden", true);
    return;
  }
  setMarkup(
    elements.selectionOverview,
    JSON.stringify(overviewRows),
    overviewRows.map((row) => overviewRowMarkup(row)).join(""),
  );
  setBooleanProperty(elements.selectionOverview, "hidden", false);
}

function hoverFromEventDetail(detail) {
  if (detail?.hover && typeof detail.hover === "object") {
    const layerSamples = Array.isArray(detail.hover.layerSamples) ? detail.hover.layerSamples : [];
    return {
      ...detail.hover,
      layerSamples,
    };
  }
  const layerSamples = Array.isArray(detail?.layerSamples) ? detail.layerSamples : [];
  return {
    worldX: detail?.worldX ?? null,
    worldZ: detail?.worldZ ?? null,
    layerSamples,
  };
}

function normalizeSelectedBookmarkIds(bookmarks, selectedIds) {
  const availableIds = new Set(normalizeBookmarks(bookmarks).map((bookmark) => bookmark.id));
  return Array.from(
    new Set(
      (Array.isArray(selectedIds) ? selectedIds : [])
        .map((bookmarkId) => String(bookmarkId || "").trim())
        .filter((bookmarkId) => availableIds.has(bookmarkId)),
    ),
  );
}

function selectedBookmarksInOrder(bookmarks, selectedIds) {
  const selectedIdSet = new Set(normalizeSelectedBookmarkIds(bookmarks, selectedIds));
  return normalizeBookmarks(bookmarks).filter((bookmark) => selectedIdSet.has(bookmark.id));
}

function bookmarkClearSelectionLabel(selectedCount) {
  return selectedCount > 0 ? `Clear (${selectedCount})` : "Clear";
}

export function buildBookmarkOverviewRows(bookmark, fallbackIndex = 0, stateBundle = null) {
  const label = bookmarkDisplayLabel(bookmark, fallbackIndex, stateBundle);
  const semanticRows = overviewRowsForLayerSamples(bookmark?.layerSamples, stateBundle).filter(
    (row) =>
      !(
        String(row?.key || "").trim() === "zone" &&
        String(row?.value || "").trim() === label
      ),
  ).map((row) => ({
    icon: row.icon,
    label: row.label,
    value: row.value,
    ...(row.hideLabel === true ? { hideLabel: true } : {}),
    ...(row.statusIcon ? { statusIcon: row.statusIcon } : {}),
    ...(row.statusIconTone ? { statusIconTone: row.statusIconTone } : {}),
  }));
  return [
    {
      icon: "bookmark",
      label: "Bookmark",
      value: label,
      hideLabel: true,
    },
    ...semanticRows,
  ];
}

function bookmarkListSignature(bookmarks) {
  return JSON.stringify(normalizeBookmarks(bookmarks));
}

export function resolveDisplayBookmarks(stateBundle, bookmarks) {
  const localBookmarks = normalizeBookmarks(bookmarks);
  const snapshotBookmarks = normalizeBookmarks(stateBundle?.state?.ui?.bookmarks || []);
  if (!snapshotBookmarks.length) {
    return localBookmarks;
  }
  const snapshotById = new Map(snapshotBookmarks.map((bookmark) => [bookmark.id, bookmark]));
  return localBookmarks.map((bookmark) => {
    const snapshotBookmark = snapshotById.get(bookmark.id);
    if (!snapshotBookmark) {
      return bookmark;
    }
    return {
      ...bookmark,
      layerSamples:
        bookmark.layerSamples?.length ? bookmark.layerSamples : snapshotBookmark.layerSamples || [],
    };
  });
}

function persistResolvedBookmarksFromStateBundle(stateBundle, bookmarks, bookmarkUi) {
  const resolvedBookmarks = resolveDisplayBookmarks(stateBundle, bookmarks);
  if (bookmarkListSignature(resolvedBookmarks) === bookmarkListSignature(bookmarks)) {
    return bookmarks;
  }
  bookmarkUi.selectedIds = normalizeSelectedBookmarkIds(
    resolvedBookmarks,
    bookmarkUi?.selectedIds,
  );
  return resolvedBookmarks;
}

function bookmarksNeedDerivedMetadata(bookmarks) {
  return normalizeBookmarks(bookmarks).some(
    (bookmark) => normalizeBookmarkLayerSamples(bookmark?.layerSamples).length === 0,
  );
}

function renderBookmarkManager(elements, stateBundle, bookmarks, bookmarkUi) {
  if (
    !elements.bookmarksList ||
    !elements.bookmarkPlace ||
    !elements.bookmarkPlaceLabel
  ) {
    return;
  }
  const state = stateBundle?.state || {};
  const canPlace = state.ready === true && state.view?.viewMode !== "3d";

  if (elements.shell) {
    if (bookmarkUi?.placing) {
      elements.shell.dataset.bookmarkPlacing = "true";
    } else {
      delete elements.shell.dataset.bookmarkPlacing;
    }
  }
  const normalizedBookmarks = resolveDisplayBookmarks(stateBundle, bookmarks);
  const selectedIds = normalizeSelectedBookmarkIds(normalizedBookmarks, bookmarkUi?.selectedIds);
  if (bookmarkUi) {
    bookmarkUi.selectedIds = selectedIds;
  }
  const selectedIdSet = new Set(selectedIds);
  setBooleanProperty(elements.bookmarkPlace, "disabled", !canPlace && !bookmarkUi?.placing);
  setTextContent(elements.bookmarkPlaceLabel, bookmarkUi?.placing ? "Click map to place" : "New bookmark");
  setBooleanProperty(elements.bookmarkCopySelected, "disabled", selectedIds.length === 0);
  setBooleanProperty(elements.bookmarkExport, "disabled", normalizedBookmarks.length === 0);
  setBooleanProperty(elements.bookmarkSelectAll, "disabled", normalizedBookmarks.length === 0 || selectedIds.length === normalizedBookmarks.length);
  setBooleanProperty(elements.bookmarkDeleteSelected, "disabled", selectedIds.length === 0);
  setBooleanProperty(elements.bookmarkClearSelection, "disabled", selectedIds.length === 0);
  setTextContent(elements.bookmarkClearSelectionLabel, bookmarkClearSelectionLabel(selectedIds.length));
  setBooleanProperty(elements.bookmarkCancel, "hidden", !bookmarkUi?.placing);

  setMarkup(
    elements.bookmarksList,
    JSON.stringify({
      bookmarks: normalizedBookmarks,
      selectedIds,
    }),
    normalizedBookmarks.length
      ? normalizedBookmarks
          .map((bookmark, index) => {
            const overviewRows = buildBookmarkOverviewRows(bookmark, index, stateBundle);
            const [titleRow, ...detailRows] = overviewRows;
            const displayLabel = bookmarkDisplayLabel(bookmark, index, stateBundle);
            return `
              <div class="fishymap-bookmark-card rounded-box border border-base-300/70 bg-base-100" data-bookmark-id="${escapeHtml(bookmark.id)}">
                <div class="fishymap-bookmark-rail">
                  <button
                    class="fishymap-bookmark-drag btn btn-xs btn-circle btn-ghost"
                    data-bookmark-drag="${escapeHtml(bookmark.id)}"
                    type="button"
                    aria-label="Drag ${escapeHtml(displayLabel)}"
                    draggable="true"
                    tabindex="-1"
                  >
                    ${dragHandleIcon()}
                  </button>
                  <span class="fishymap-bookmark-order badge badge-soft badge-sm">${index + 1}</span>
                  <label class="fishymap-bookmark-toggle" aria-label="Select ${escapeHtml(displayLabel)}">
                    <input
                      class="checkbox checkbox-sm"
                      type="checkbox"
                      data-bookmark-select="${escapeHtml(bookmark.id)}"
                      ${selectedIdSet.has(bookmark.id) ? "checked" : ""}
                    >
                  </label>
                </div>
                <div class="fishymap-bookmark-main">
                  <div class="fishymap-bookmark-titlebar">
                    <div class="fishymap-bookmark-title">
                      ${titleRow ? overviewRowMarkup(titleRow) : ""}
                    </div>
                    <button
                      class="fishymap-bookmark-rename btn btn-soft btn-sm btn-square"
                      type="button"
                      data-bookmark-rename="${escapeHtml(bookmark.id)}"
                      aria-label="Rename bookmark"
                      title="Rename bookmark"
                    >
                      ${spriteIcon("bookmark-edit", "size-5")}
                    </button>
                  </div>
                  ${
                    detailRows.length
                      ? `
                    <div class="fishymap-overview-list fishymap-overview-list--bookmark">
                      ${detailRows.map((row) => overviewRowMarkup(row)).join("")}
                    </div>
                  `
                      : ""
                  }
                </div>
                <div class="fishymap-bookmark-actions-rail">
                  <button
                    class="fishymap-bookmark-activate btn btn-soft btn-sm btn-square"
                    type="button"
                    data-bookmark-activate="${escapeHtml(bookmark.id)}"
                    aria-label="Inspect bookmark"
                    title="Inspect bookmark"
                  >
                    ${spriteIcon("map-view", "size-5")}
                  </button>
                  <button
                    class="fishymap-bookmark-copy btn btn-soft btn-primary btn-sm btn-square"
                    type="button"
                    data-bookmark-copy="${escapeHtml(bookmark.id)}"
                    aria-label="Copy bookmark XML"
                    title="Copy bookmark XML"
                  >
                    ${spriteIcon("copy", "size-5")}
                  </button>
                  <button
                    class="fishymap-bookmark-delete btn btn-ghost btn-error btn-xs btn-square"
                    type="button"
                    data-bookmark-delete="${escapeHtml(bookmark.id)}"
                    aria-label="Delete bookmark"
                    title="Delete bookmark"
                  >
                    ${spriteIcon("trash", "size-4")}
                  </button>
                </div>
              </div>
            `;
          })
          .join("")
      : `
        <div class="fishymap-bookmark-empty text-sm text-base-content/65">
          No bookmarks yet.
        </div>
      `,
  );

  renderHoverTooltip(elements, state.hover || null, stateBundle);
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

function normalizeSemanticFieldIdsByLayer(values) {
  if (!values || typeof values !== "object" || Array.isArray(values)) {
    return {};
  }
  const out = {};
  for (const [layerIdRaw, fieldIdsRaw] of Object.entries(values)) {
    const layerId = String(layerIdRaw || "").trim();
    if (!layerId || !Array.isArray(fieldIdsRaw)) {
      continue;
    }
    const fieldIds = [];
    const seen = new Set();
    for (const value of fieldIdsRaw) {
      const fieldId = Number.parseInt(value, 10);
      if (!Number.isFinite(fieldId) || seen.has(fieldId)) {
        continue;
      }
      seen.add(fieldId);
      fieldIds.push(fieldId);
    }
    if (fieldIds.length) {
      out[layerId] = fieldIds;
    }
  }
  return out;
}

function resolveSelectedSemanticFieldIdsByLayer(stateBundle) {
  const inputSelected = normalizeSemanticFieldIdsByLayer(
    stateBundle.inputState?.filters?.semanticFieldIdsByLayer,
  );
  if (Object.keys(inputSelected).length) {
    return inputSelected;
  }
  const stateSelected = normalizeSemanticFieldIdsByLayer(
    stateBundle.state?.filters?.semanticFieldIdsByLayer,
  );
  if (Object.keys(stateSelected).length) {
    return stateSelected;
  }
  const inputZoneRgbs = stateBundle.inputState?.filters?.zoneRgbs;
  if (Array.isArray(inputZoneRgbs) && inputZoneRgbs.length) {
    return { zone_mask: inputZoneRgbs };
  }
  const stateZoneRgbs = stateBundle.state?.filters?.zoneRgbs;
  if (Array.isArray(stateZoneRgbs) && stateZoneRgbs.length) {
    return { zone_mask: stateZoneRgbs };
  }
  return {};
}

function resolveSelectedZoneRgbs(stateBundle) {
  const selectedByLayer = resolveSelectedSemanticFieldIdsByLayer(stateBundle);
  if (Array.isArray(selectedByLayer.zone_mask)) {
    return selectedByLayer.zone_mask;
  }
  return [];
}

function normalizeFishFilterTerm(value) {
  const normalized = String(value ?? "").trim().toLowerCase();
  if (
    normalized === "favourite" ||
    normalized === "favourites" ||
    normalized === "favorite" ||
    normalized === "favorites"
  ) {
    return "favourite";
  }
  if (
    normalized === "missing" ||
    normalized === "uncaught" ||
    normalized === "not caught" ||
    normalized === "not yet caught"
  ) {
    return "missing";
  }
  return "";
}

function normalizeFishFilterTerms(values) {
  const selected = new Set();
  for (const value of Array.isArray(values) ? values : []) {
    const normalized = normalizeFishFilterTerm(value);
    if (normalized) {
      selected.add(normalized);
    }
  }
  return FISH_FILTER_TERM_ORDER.filter((term) => selected.has(term));
}

function resolveSelectedFishFilterTerms(stateBundle) {
  const inputTerms = normalizeFishFilterTerms(stateBundle?.inputState?.filters?.fishFilterTerms);
  if (inputTerms.length) {
    return inputTerms;
  }
  return normalizeFishFilterTerms(stateBundle?.state?.filters?.fishFilterTerms);
}

function addSelectedFishFilterTerm(selectedTerms, term) {
  return normalizeFishFilterTerms((selectedTerms || []).concat(term));
}

function removeSelectedFishFilterTerm(selectedTerms, term) {
  const normalizedTerm = normalizeFishFilterTerm(term);
  return normalizeFishFilterTerms((selectedTerms || []).filter((entry) => entry !== normalizedTerm));
}

function parseFishFilterDirectives(searchText) {
  const rawQuery = String(searchText || "").trim().toLowerCase().replace(/\s+/g, " ");
  if (!rawQuery) {
    return {
      rawQuery: "",
      remainingQuery: "",
      directTerms: [],
    };
  }

  let remainingQuery = rawQuery;
  const directTerms = new Set();
  const replacements = [
    {
      term: "missing",
      patterns: [/\bnot\s+yet\s+caught\b/g, /\bnot\s+caught\b/g, /\buncaught\b/g, /\bmissing\b/g],
    },
    {
      term: "favourite",
      patterns: [/\bfavou?rites?\b/g],
    },
  ];
  for (const replacement of replacements) {
    for (const pattern of replacement.patterns) {
      remainingQuery = remainingQuery.replace(pattern, () => {
        directTerms.add(replacement.term);
        return " ";
      });
    }
  }
  remainingQuery = remainingQuery.replace(/\s+/g, " ").trim();
  return {
    rawQuery,
    remainingQuery,
    directTerms: FISH_FILTER_TERM_ORDER.filter((term) => directTerms.has(term)),
  };
}

function resolveSharedFishState(stateBundle) {
  return withSharedFishState(stateBundle).sharedFishState;
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

function fishMatchesFilterTerms(fish, filterTerms, sharedFishState) {
  if (!filterTerms.length) {
    return true;
  }
  const fishId = Number.parseInt(fish?.fishId ?? fish?.itemId, 10);
  if (!Number.isInteger(fishId) || fishId <= 0) {
    return false;
  }
  for (const term of filterTerms) {
    if (term === "favourite" && !sharedFishState?.favouriteSet?.has(fishId)) {
      return false;
    }
    if (term === "missing" && sharedFishState?.caughtSet?.has(fishId)) {
      return false;
    }
  }
  return true;
}

function findFishMatches(catalogFish, searchText, options = {}) {
  const query = String(searchText || "").trim().toLowerCase();
  const terms = query ? query.split(/\s+/g).filter(Boolean) : [];
  const includeAllWhenEmpty = options.includeAllWhenEmpty === true;
  const filterTerms = normalizeFishFilterTerms(options.filterTerms);
  const sharedFishState = options.sharedFishState;
  if (!terms.length && !includeAllWhenEmpty) {
    return [];
  }
  const filtered = [];
  for (const fish of catalogFish || []) {
    if (!fishMatchesFilterTerms(fish, filterTerms, sharedFishState)) {
      continue;
    }
    const score = terms.length ? scoreFishMatch(fish, terms) : 0;
    if (terms.length && !Number.isFinite(score)) {
      continue;
    }
    filtered.push({
      ...fish,
      _score: Number.isFinite(score) ? score : 0,
    });
  }
  filtered.sort((left, right) => {
    if (terms.length && right._score !== left._score) {
      return right._score - left._score;
    }
    return String(left.name || "").localeCompare(String(right.name || ""));
  });
  return filtered;
}

function findFishFilterMatches(searchText, selectedTerms) {
  const query = String(searchText || "").trim().toLowerCase();
  const terms = query ? query.split(/\s+/g).filter(Boolean) : [];
  if (!terms.length) {
    return [];
  }
  const selected = new Set(normalizeFishFilterTerms(selectedTerms));
  const matches = [];
  for (const term of FISH_FILTER_TERM_ORDER) {
    if (selected.has(term)) {
      continue;
    }
    const metadata = FISH_FILTER_TERM_METADATA[term];
    let score = 0;
    for (const queryTerm of terms) {
      const best = Math.max(
        term === queryTerm ? 240 : Number.NEGATIVE_INFINITY,
        scoreTermMatch(String(metadata?.label || "").toLowerCase(), queryTerm, 200),
        scoreTermMatch(String(metadata?.description || "").toLowerCase(), queryTerm, 140),
        scoreTermMatch(String(metadata?.searchText || "").toLowerCase(), queryTerm, 160),
      );
      if (!Number.isFinite(best)) {
        score = Number.NEGATIVE_INFINITY;
        break;
      }
      score += best;
    }
    if (!Number.isFinite(score)) {
      continue;
    }
    matches.push({
      kind: "fish-filter",
      term,
      label: metadata?.label || term,
      description: metadata?.description || "",
      _score: score,
    });
  }
  matches.sort((left, right) => {
    if (right._score !== left._score) {
      return right._score - left._score;
    }
    return String(left.label || "").localeCompare(String(right.label || ""));
  });
  return matches;
}

function buildDefaultFishFilterMatches(stateBundle) {
  const selected = new Set(resolveSelectedFishFilterTerms(stateBundle));
  return FISH_FILTER_TERM_ORDER.filter((term) => !selected.has(term)).map((term) => ({
    kind: "fish-filter",
    term,
    label: FISH_FILTER_TERM_METADATA[term]?.label || term,
    description: FISH_FILTER_TERM_METADATA[term]?.description || "",
    _score: 0,
  }));
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

function scoreSemanticMatch(term, queryTerms) {
  if (!queryTerms.length) {
    return 0;
  }
  const fieldId = String(term.fieldId || "");
  const label = String(term.label || "").toLowerCase();
  const description = String(term.description || "").toLowerCase();
  const layerName = String(term.layerName || "").toLowerCase();
  const searchText = String(term.searchText || "").toLowerCase();
  let score = 0;
  for (const queryTerm of queryTerms) {
    const best = Math.max(
      fieldId === queryTerm ? 220 : Number.NEGATIVE_INFINITY,
      scoreTermMatch(label, queryTerm, 170),
      scoreTermMatch(description, queryTerm, 130),
      scoreTermMatch(layerName, queryTerm, 90),
      scoreTermMatch(searchText, queryTerm, 80),
    );
    if (!Number.isFinite(best)) {
      return Number.NEGATIVE_INFINITY;
    }
    score += best;
  }
  return score;
}

export function findSemanticMatches(semanticTerms, searchText) {
  const query = String(searchText || "").trim().toLowerCase();
  const terms = query ? query.split(/\s+/g).filter(Boolean) : [];
  if (!query) {
    return [];
  }
  const filteredByKey = new Map();
  for (const term of semanticTerms || []) {
    if (!term || String(term.layerId || "").trim() === "zone_mask") {
      continue;
    }
    const score = scoreSemanticMatch(term, terms);
    if (!Number.isFinite(score)) {
      continue;
    }
    const candidate = {
      kind: "semantic",
      ...term,
      _score: score,
    };
    const key = semanticTermLookupKey(term.layerId, term.fieldId);
    const existing = filteredByKey.get(key);
    if (
      !existing ||
      candidate._score > existing._score ||
      (candidate._score === existing._score &&
        String(candidate.label || "").length < String(existing.label || "").length)
    ) {
      filteredByKey.set(key, candidate);
    }
  }
  const filtered = Array.from(filteredByKey.values());
  filtered.sort((left, right) => {
    if (right._score !== left._score) {
      return right._score - left._score;
    }
    if (left.layerName !== right.layerName) {
      return String(left.layerName || "").localeCompare(String(right.layerName || ""));
    }
    return String(left.label || "").localeCompare(String(right.label || ""));
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

export function buildZoneEvidenceSummary(selection, zoneStats) {
  const zoneRgb = selection?.zoneRgb ?? zoneRgbFromLayerSamples(selection?.layerSamples);
  if (!zoneStats) {
    return zoneRgb != null
      ? "No zone evidence loaded."
      : "Zone evidence is only available for zone selections.";
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

export function selectionHasZoneEvidence(selection, zoneStats = null) {
  const zoneRgb =
    zoneStats?.zoneRgb ?? selection?.zoneRgb ?? zoneRgbFromLayerSamples(selection?.layerSamples);
  return zoneRgb != null;
}

export function buildZoneEvidenceListMarkup(distribution, fishLookup = new Map()) {
  const entries = Array.isArray(distribution) ? distribution : [];
  return entries
    .map((entry) => {
      const fish = fishLookup.get(entry.fishId);
      const evidenceFish = {
        fishId: entry.fishId,
        itemId: fish?.itemId ?? entry.itemId ?? null,
        encyclopediaId: fish?.encyclopediaId ?? entry.encyclopediaId ?? null,
        name: fish?.name || entry.fishName || `Fish ${entry.fishId}`,
        grade: fish?.grade ?? entry?.grade ?? null,
        isPrize: fish?.isPrize === true || entry?.isPrize === true,
      };
      const ci =
        Number.isFinite(entry.ciLow) && Number.isFinite(entry.ciHigh)
          ? `${formatDecimal(entry.ciLow)}-${formatDecimal(entry.ciHigh)}`
          : "n/a";
      const detailLabel = `p ${formatDecimal(entry.pMean)} · weight ${formatDecimal(entry.evidenceWeight)} · CI ${ci}`;
      return `
        <div
          class="list-row w-full rounded-box border border-transparent bg-base-100 px-2.5 py-2 text-left hover:border-base-300"
          data-zone-evidence-fish-id="${evidenceFish.fishId}"
          title="${escapeHtml(detailLabel)}"
          aria-description="${escapeHtml(detailLabel)}"
          role="button"
          tabindex="0"
        >
          <div class="min-w-0">
            ${fishIdentityMarkup(evidenceFish, { interactive: true })}
          </div>
        </div>
      `;
    })
    .join("");
}

function zoneInfoTabButtonClass(active) {
  return `${ZONE_INFO_TAB_BUTTON_CLASS}${active ? " tab-active" : ""}`;
}

function ensureZoneInfoElements(elements) {
  if (
    elements.zoneInfoStatus &&
    elements.zoneInfoTabs &&
    elements.zoneInfoPanel &&
    elements.zoneInfoTitle &&
    elements.zoneInfoTitleIcon &&
    elements.zoneInfoStatusIcon &&
    elements.zoneInfoStatusText
  ) {
    return elements;
  }
  if (!elements.panelBody) {
    return elements;
  }

  const section = document.createElement("div");
  section.className = "space-y-3";
  section.innerHTML = `
    <div class="flex items-center justify-between gap-3">
      <span class="inline-flex min-w-0 items-center gap-2 text-sm font-semibold">
        <span id="fishymap-zone-info-title-icon" class="inline-flex text-base-content/70" aria-hidden="true">${spriteIcon("information-circle", "size-4")}</span>
        <span id="fishymap-zone-info-title" class="truncate">Zone Info</span>
      </span>
      <span id="fishymap-zone-info-status" class="inline-flex min-w-0 items-center gap-2 text-xs text-base-content/60">
        <span id="fishymap-zone-info-status-icon" class="inline-flex text-base-content/55" aria-hidden="true">${spriteIcon("information-circle", "size-4")}</span>
        <span id="fishymap-zone-info-status-text" class="truncate">no selection</span>
      </span>
    </div>
    <div id="fishymap-zone-info-tabs" role="tablist" class="tabs tabs-box bg-base-200/80 p-1" aria-label="Point layer tabs" hidden></div>
    <div id="fishymap-zone-info-panel" class="space-y-3"></div>
  `;

  if (elements.legend?.parentNode === elements.panelBody) {
    elements.panelBody.insertBefore(section, elements.legend);
  } else {
    elements.panelBody.appendChild(section);
  }

  elements.zoneInfoTabs = section.querySelector("#fishymap-zone-info-tabs");
  elements.zoneInfoPanel = section.querySelector("#fishymap-zone-info-panel");
  elements.zoneInfoStatus = section.querySelector("#fishymap-zone-info-status");
  elements.zoneInfoStatusIcon = section.querySelector("#fishymap-zone-info-status-icon");
  elements.zoneInfoStatusText = section.querySelector("#fishymap-zone-info-status-text");
  elements.zoneInfoTitle = section.querySelector("#fishymap-zone-info-title");
  elements.zoneInfoTitleIcon = section.querySelector("#fishymap-zone-info-title-icon");
  return elements;
}

export function buildPointDetailPanes(selection, stateBundle) {
  return POINT_DETAIL_PANE_BUILDERS.flatMap((builder) => builder(selection, stateBundle)).filter(
    Boolean,
  );
}

function buildLayerSamplePointDetailPanes(selection, stateBundle) {
  const orderedSamples = orderedLayerSamplesForPointDetails(selection, stateBundle);
  const panes = new Map();

  orderedSamples.forEach((sample, index) => {
    const descriptor = resolvePointDetailPaneDescriptor(sample, stateBundle, index);
    const paneId = String(descriptor?.id || "").trim();
    if (!paneId) {
      return;
    }
    const existing = panes.get(paneId);
    const preferredRow = preferredLayerSampleOverviewRow([sample], stateBundle);
    const staticSections = buildLayerSampleStaticPointDetailSections(sample, stateBundle);
    if (existing) {
      existing.samples.push(sample);
      existing.sections.push(...staticSections);
      if (!existing.summary) {
        existing.summary = String(preferredRow?.value || "").trim();
      }
      return;
    }
    panes.set(paneId, {
      id: paneId,
      label: String(descriptor?.label || paneId).trim() || paneId,
      icon: String(descriptor?.icon || "").trim() || "information-circle",
      order: Number.isFinite(Number(descriptor?.order)) ? Number(descriptor.order) : index,
      summary: String(preferredRow?.value || "").trim(),
      samples: [sample],
      sections: staticSections,
    });
  });

  return [...panes.values()]
    .sort((left, right) => {
      const orderDelta = Number(left?.order || 0) - Number(right?.order || 0);
      if (orderDelta !== 0) {
        return orderDelta;
      }
      return String(left?.label || "").localeCompare(String(right?.label || ""));
    })
    .map((pane) => ({
      ...pane,
      sections: buildPointDetailPaneSections(pane, selection, stateBundle),
    }));
}

function buildPointDetailPaneSections(pane, selection, stateBundle) {
  const context = { pane, selection, stateBundle };
  const baseSections = Array.isArray(pane?.sections) ? pane.sections : [];
  const dynamicSections = POINT_DETAIL_SECTION_BUILDERS.flatMap((builder) => builder(context)).filter(
    Boolean,
  );
  return [...baseSections, ...dynamicSections];
}

function orderedLayerSamplesForPointDetails(selection, stateBundle) {
  const layerSamples = Array.isArray(selection?.layerSamples) ? selection.layerSamples : [];
  const sampleByLayerId = new Map(
    layerSamples
      .map((sample) => [String(sample?.layerId || "").trim(), sample])
      .filter(([layerId]) => Boolean(layerId)),
  );
  const layerIds = orderedLayerIdsForLayerSamples(layerSamples, sampleByLayerId, stateBundle);
  return layerIds.map((layerId) => sampleByLayerId.get(layerId)).filter(Boolean);
}

function resolvePointDetailPaneDescriptor(sample, stateBundle, fallbackOrder = 0) {
  const detailPane = sample?.detailPane;
  const detailPaneId = String(detailPane?.id || "").trim();
  if (detailPaneId) {
    return {
      id: detailPaneId,
      label: String(detailPane?.label || detailPaneId).trim() || detailPaneId,
      icon: String(detailPane?.icon || "").trim() || "information-circle",
      order: Number.isFinite(Number(detailPane?.order)) ? Number(detailPane.order) : fallbackOrder,
    };
  }
  const layerId = String(sample?.layerId || "").trim();
  if (!layerId) {
    return null;
  }
  const preferredRow = preferredLayerSampleOverviewRow([sample], stateBundle);
  return {
    id: layerId,
    label: String(sample?.layerName || layerId).trim() || layerId,
    icon: String(preferredRow?.icon || "").trim() || "information-circle",
    order: fallbackOrder + 1000,
  };
}

function buildLayerSampleStaticPointDetailSections(sample, stateBundle) {
  if (String(sample?.layerId || "").trim() === "zone_mask") {
    const zoneSection = buildZoneCatalogDetailSection(sample, resolveZoneCatalog(stateBundle));
    return zoneSection ? [zoneSection] : [];
  }
  const detailSections = normalizePointDetailSections(sample?.detailSections);
  if (detailSections.length > 0) {
    return detailSections;
  }

  const targets = normalizePointDetailTargets(sample?.targets);
  if (targets.length > 0) {
    return [
      {
      id: `${String(sample?.layerId || "layer").trim() || "layer"}-targets`,
      kind: "targets",
      title: "Targets",
      targets,
      },
    ];
  }
  return [];
}

function normalizePointDetailSections(sectionsInput) {
  const sections = Array.isArray(sectionsInput) ? sectionsInput : [];
  const normalized = [];
  for (const section of sections) {
    const id = String(section?.id || "").trim();
    const kind = String(section?.kind || "").trim();
    if (!id || !kind) {
      continue;
    }
    const title = String(section?.title || "").trim();
    const facts = normalizePointDetailFacts(section?.facts);
    const targets = normalizePointDetailTargets(section?.targets);
    if (kind === "facts" && facts.length === 0 && targets.length === 0) {
      continue;
    }
    normalized.push({
      id,
      kind,
      ...(title ? { title } : {}),
      ...(facts.length ? { facts } : {}),
      ...(targets.length ? { targets } : {}),
    });
  }
  return normalized;
}

function normalizePointDetailFacts(factsInput) {
  const facts = Array.isArray(factsInput) ? factsInput : [];
  const normalized = [];
  for (const fact of facts) {
    const label = String(fact?.label || "").trim();
    const value = String(fact?.value || "").trim();
    if (!label || !value) {
      continue;
    }
    const key = String(fact?.key || "").trim();
    const icon = String(fact?.icon || "").trim();
    const statusIcon = String(fact?.statusIcon || "").trim();
    const statusIconTone = String(fact?.statusIconTone || "").trim();
    normalized.push({
      ...(key ? { key } : {}),
      ...(icon ? { icon } : {}),
      label,
      value,
      ...(statusIcon ? { statusIcon } : {}),
      ...(statusIconTone ? { statusIconTone } : {}),
    });
  }
  return normalized;
}

function normalizePointDetailTargets(targetsInput) {
  return (Array.isArray(targetsInput) ? targetsInput : []).filter((target) => {
    const label = String(target?.label || "").trim();
    return (
      label &&
      normalizeBookmarkCoordinate(target?.worldX) != null &&
      normalizeBookmarkCoordinate(target?.worldZ) != null
    );
  });
}

function pointDetailFactsRenderKeyData(section) {
  return {
    id: String(section?.id || "").trim(),
    kind: "facts",
    title: String(section?.title || "").trim(),
    facts: normalizePointDetailFacts(section?.facts).map((fact) => [
      String(fact?.key || "").trim(),
      String(fact?.label || "").trim(),
      String(fact?.value || "").trim(),
      String(fact?.icon || "").trim(),
      String(fact?.statusIcon || "").trim(),
      String(fact?.statusIconTone || "").trim(),
    ]),
    targets: normalizePointDetailTargets(section?.targets).map((target) => [
      String(target?.key || "").trim(),
      String(target?.label || "").trim(),
      normalizeBookmarkCoordinate(target?.worldX),
      normalizeBookmarkCoordinate(target?.worldZ),
    ]),
  };
}

function pointDetailTargetsRenderKeyData(section) {
  return {
    id: String(section?.id || "").trim(),
    kind: "targets",
    title: String(section?.title || "").trim(),
    targets: normalizePointDetailTargets(section?.targets).map((target) => [
      String(target?.key || "").trim(),
      String(target?.label || "").trim(),
      normalizeBookmarkCoordinate(target?.worldX),
      normalizeBookmarkCoordinate(target?.worldZ),
    ]),
  };
}

function pointDetailZoneEvidenceRenderKeyData(section) {
  const zoneStats = section?.zoneStats || null;
  const confidence = zoneStats?.confidence || {};
  const distribution = Array.isArray(zoneStats?.distribution) ? zoneStats.distribution : [];
  return {
    id: String(section?.id || "").trim(),
    kind: "zoneEvidence",
    zoneStatsStatus: String(section?.zoneStatsStatus || "").trim(),
    zoneRgb:
      zoneStats?.zoneRgb ??
      section?.selection?.zoneRgb ??
      zoneRgbFromLayerSamples(section?.selection?.layerSamples) ??
      null,
    confidence: [
      String(confidence?.status || "").trim(),
      Number.isFinite(confidence?.ess) ? confidence.ess : null,
      Number.isFinite(confidence?.totalWeight) ? confidence.totalWeight : null,
      Number.isFinite(confidence?.lastSeenTsUtc) ? confidence.lastSeenTsUtc : null,
      Number.isFinite(confidence?.ageDaysLast) ? confidence.ageDaysLast : null,
      Array.isArray(confidence?.notes)
        ? confidence.notes.map((note) => String(note || "").trim())
        : [],
    ],
    distribution: distribution.map((entry) => [
      Number.isFinite(entry?.fishId) ? entry.fishId : null,
      Number.isFinite(entry?.pMean) ? entry.pMean : null,
      Number.isFinite(entry?.evidenceWeight) ? entry.evidenceWeight : null,
      Number.isFinite(entry?.ciLow) ? entry.ciLow : null,
      Number.isFinite(entry?.ciHigh) ? entry.ciHigh : null,
    ]),
  };
}

function pointDetailSectionRenderKeyData(section) {
  switch (String(section?.kind || "").trim()) {
    case "facts":
      return pointDetailFactsRenderKeyData(section);
    case "targets":
      return pointDetailTargetsRenderKeyData(section);
    case "zoneEvidence":
      return pointDetailZoneEvidenceRenderKeyData(section);
    default:
      return {
        id: String(section?.id || "").trim(),
        kind: String(section?.kind || "").trim(),
      };
  }
}

export function pointDetailPaneMarkupKey(pane) {
  return JSON.stringify({
    id: String(pane?.id || "").trim(),
    summary: String(pane?.summary || "").trim(),
    sections: (Array.isArray(pane?.sections) ? pane.sections : []).map((section) =>
      pointDetailSectionRenderKeyData(section),
    ),
  });
}

function buildZoneEvidencePointDetailSection({ pane, selection, stateBundle }) {
  if (pane?.id !== "zone_mask") {
    return [];
  }
  const zoneStats = stateBundle?.state?.selection?.zoneStats || null;
  if (!selectionHasZoneEvidence(selection, zoneStats)) {
    return [];
  }
  return [
    {
      id: "zone-evidence",
      kind: "zoneEvidence",
      selection,
      zoneStats,
      zoneStatsStatus: stateBundle?.state?.statuses?.zoneStatsStatus || "zone evidence: idle",
    },
  ];
}

export function resolveZoneInfoActiveTab(windowUiState, selection, stateBundle) {
  return buildPointDetailViewModel(selection, stateBundle, windowUiState).activePaneId;
}

function pointKindIcon(pointKind) {
  switch (normalizeSelectionPointKind(pointKind)) {
    case "bookmark":
      return "bookmark";
    case "waypoint":
      return "map-pin";
    case "clicked":
      return "hover-zone";
    default:
      return "information-circle";
  }
}

function pointKindStatusText(pointKind, pointLabel) {
  const normalizedLabel = String(pointLabel || "").trim();
  switch (normalizeSelectionPointKind(pointKind)) {
    case "bookmark":
      return normalizedLabel || "Bookmark";
    case "waypoint":
      return normalizedLabel || "Waypoint";
    case "clicked":
      return "Clicked point";
    default:
      return "no selection";
  }
}

function zoneInfoTitleDescriptor(selection, stateBundle) {
  const selectedBookmark = selectedBookmarkForSelection(selection, stateBundle);
  if (selectedBookmark) {
    return {
      title: bookmarkDisplayLabel(selectedBookmark),
      titleIcon: "bookmark",
      statusIcon: "bookmark",
      statusText: pointKindStatusText("bookmark", selectedBookmark.label),
      pointKind: "bookmark",
    };
  }
  if (!selection) {
    return {
      title: "Zone Info",
      titleIcon: "information-circle",
      statusIcon: "information-circle",
      statusText: "no selection",
      pointKind: "",
    };
  }
  const pointKind = normalizeSelectionPointKind(selection?.pointKind);
  const pointLabel = String(selection?.pointLabel || "").trim();
  const title = buildSelectionSummaryText(selection, stateBundle) || "Zone Info";
  return {
    title,
    titleIcon: pointKindIcon(pointKind),
    statusIcon: pointKindIcon(pointKind),
    statusText: pointKindStatusText(pointKind, pointLabel),
    pointKind,
  };
}

export function buildPointDetailViewModel(selection, stateBundle, windowUiState) {
  const panes = buildPointDetailPanes(selection, stateBundle);
  const requestedPaneId = normalizeZoneInfoTab(windowUiState?.zoneInfo?.tab);
  const activePaneId =
    requestedPaneId && panes.some((pane) => pane.id === requestedPaneId)
      ? requestedPaneId
      : panes[0]?.id || DEFAULT_ZONE_INFO_TAB;
  const activePane = panes.find((pane) => pane.id === activePaneId) || null;
  return {
    descriptor: zoneInfoTitleDescriptor(selection, stateBundle),
    panes,
    activePaneId,
    activePane,
  };
}

function zoneInfoTargetMarkup(target) {
  const label = String(target?.label || "").trim();
  const worldX = normalizeBookmarkCoordinate(target?.worldX);
  const worldZ = normalizeBookmarkCoordinate(target?.worldZ);
  if (!label || worldX == null || worldZ == null) {
    return "";
  }
  const labelMarkup =
    semanticIdentityMarkup(label, { interactive: false }) ||
    `<span class="truncate">${escapeHtml(label)}</span>`;
  return `
    <button
      class="btn btn-soft btn-sm justify-start"
      type="button"
      aria-label="${escapeHtml(label)}"
      title="${escapeHtml(label)}"
      data-zone-info-target-world-x="${worldX}"
      data-zone-info-target-world-z="${worldZ}"
      data-zone-info-target-label="${escapeHtml(label)}"
    >
      ${spriteIcon("map-pin", "size-4")}
      <span class="fishymap-target-label">${labelMarkup}</span>
    </button>
  `;
}

function zoneInfoZoneEvidenceMarkup(selection, zoneStats, zoneStatsStatus, fishLookup) {
  const summary = buildZoneEvidenceSummary(selection, zoneStats);
  const distribution = Array.isArray(zoneStats?.distribution) ? zoneStats.distribution : [];
  const listMarkup = !zoneStats
    ? '<div class="px-2 py-3 text-xs text-base-content/60">No zone evidence loaded.</div>'
    : !distribution.length
      ? '<div class="px-2 py-3 text-xs text-base-content/60">No fish evidence in this window.</div>'
      : buildZoneEvidenceListMarkup(distribution, fishLookup);
  return `
    <section class="space-y-2">
      <div class="flex items-center justify-between gap-3">
        <p class="text-[11px] font-semibold uppercase tracking-[0.18em] text-base-content/45">Zone Evidence</p>
        <span class="text-[11px] text-base-content/55">${escapeHtml(String(zoneStatsStatus || "zone evidence: idle"))}</span>
      </div>
      <p class="text-xs text-base-content/70">${escapeHtml(summary)}</p>
      <div class="rounded-box border border-warning/35 bg-warning/10 p-3 text-sm text-base-content/85 shadow-sm">
        <p class="mb-2 flex items-center gap-2 font-semibold uppercase tracking-widest text-warning">
          <svg class="fishy-icon size-4" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="${ICON_SPRITE_URL}#fishy-information-circle"></use></svg>
          Disclaimer
        </p>
        <div class="space-y-2 leading-5">
          <p>The fish displayed here are all available evidence samples that might belong to this zone.</p>
          <p>Some fish might have been close to the zone border and may actually belong to a neighbouring zone instead.</p>
          <p>You can see the exact sample locations in Layers by enabling the Fish Evidence layer and its icon toggle.</p>
          <p>Keep this in mind and verify with other sources such as BDOlytics for now.</p>
        </div>
      </div>
      <div class="list max-h-72 overflow-y-auto rounded-box border border-base-300 bg-base-200 p-1">${listMarkup}</div>
    </section>
  `;
}

function pointDetailFactsSectionMarkup(section) {
  const title = String(section?.title || "").trim();
  const facts = Array.isArray(section?.facts) ? section.facts : [];
  const targets = Array.isArray(section?.targets) ? section.targets : [];
  if (facts.length === 0 && targets.length === 0) {
    return "";
  }
  return `
    <section class="space-y-2">
      ${title ? `<p class="text-[11px] font-semibold uppercase tracking-[0.18em] text-base-content/45">${escapeHtml(title)}</p>` : ""}
      ${
        facts.length
          ? `<div class="fishymap-overview-list">${facts
              .map((fact) =>
                overviewRowMarkup({
                  icon: String(fact?.icon || "").trim() || "information-circle",
                  label: fact.label,
                  value: fact.value,
                }),
              )
              .join("")}</div>`
          : ""
      }
      ${
        targets.length
          ? `<div class="flex flex-wrap gap-2">${targets.map((target) => zoneInfoTargetMarkup(target)).join("")}</div>`
          : ""
      }
    </section>
  `;
}

function pointDetailTargetsSectionMarkup(section) {
  const targets = Array.isArray(section?.targets) ? section.targets : [];
  const title = String(section?.title || "").trim() || "Targets";
  if (!targets.length) {
    return "";
  }
  return `
    <section class="space-y-2">
      <p class="text-[11px] font-semibold uppercase tracking-[0.18em] text-base-content/45">${escapeHtml(title)}</p>
      <div class="flex flex-wrap gap-2">
        ${targets.map((target) => zoneInfoTargetMarkup(target)).join("")}
      </div>
    </section>
  `;
}

function pointDetailSectionMarkup(section, pane, fishLookup) {
  switch (String(section?.kind || "").trim()) {
    case "facts":
      return pointDetailFactsSectionMarkup(section);
    case "targets":
      return pointDetailTargetsSectionMarkup(section);
    case "zoneEvidence":
      return zoneInfoZoneEvidenceMarkup(
        section.selection,
        section.zoneStats,
        section.zoneStatsStatus,
        fishLookup,
      );
    default:
      return "";
  }
}

function pointDetailPaneMarkup(pane, fishLookup) {
  if (pane?.id === "territory") {
    return territoryPointDetailPaneMarkup(pane);
  }
  const sections = Array.isArray(pane?.sections) ? pane.sections : [];
  return `
    <section class="space-y-3" data-zone-info-layer-panel="${escapeHtml(pane.id)}">
      ${sections.map((section) => pointDetailSectionMarkup(section, pane, fishLookup)).join("")}
    </section>
  `;
}

export function territoryPointDetailPaneMarkup(pane) {
  const sections = Array.isArray(pane?.sections) ? pane.sections : [];
  return `
    <section class="space-y-3" data-zone-info-layer-panel="${escapeHtml(pane.id)}">
      ${sections
        .map((section) => {
          switch (String(section?.kind || "").trim()) {
            case "facts":
              return pointDetailFactsSectionMarkup(section);
            case "targets":
              return pointDetailTargetsSectionMarkup(section);
            default:
              return "";
          }
        })
        .join("")}
    </section>
  `;
}

function territoryOverviewRowsFromSections(sectionsInput) {
  const sections = Array.isArray(sectionsInput) ? sectionsInput : [];
  const selectedFacts = new Map();
  for (const section of sections) {
    for (const fact of normalizePointDetailFacts(section?.facts)) {
      const factKey = String(fact?.key || "").trim();
      if (
        factKey === "resource_group" &&
        !selectedFacts.has("resources")
      ) {
        selectedFacts.set("resources", {
          ...fact,
          key: "resources",
        });
      } else if (
        factKey === "resource_region" &&
        !selectedFacts.has("resources")
      ) {
        selectedFacts.set("resources", {
          ...fact,
          key: "resources",
        });
      } else if (
        factKey === "origin_region" &&
        !selectedFacts.has("origin")
      ) {
        selectedFacts.set("origin", {
          ...fact,
          key: "origin",
        });
      }
    }
  }
  return TERRITORY_SUMMARY_FACT_KEYS.map((key) => selectedFacts.get(key))
    .filter(Boolean)
    .map((fact) => ({
      key: String(fact?.key || "").trim(),
      icon: displayIconForDetailFact(fact),
      label: displayLabelForDetailFact(null, fact),
      value: String(fact?.value || "").trim(),
      ...(fact?.statusIcon ? { statusIcon: String(fact.statusIcon).trim() } : {}),
      ...(fact?.statusIconTone ? { statusIconTone: String(fact.statusIconTone).trim() } : {}),
    }))
    .filter((row) => Boolean(row.icon && row.label && row.value));
}

function pointDetailTabTitle(tab) {
  return tab.summary ? `${tab.label}: ${tab.summary}` : `Show ${tab.label} details.`;
}

function pointDetailTabButtonMarkup(tab, activeTabId) {
  const isActive = tab.id === activeTabId;
  return `
    <button
      id="fishymap-zone-info-tab-${escapeHtml(tab.id)}"
      class="${zoneInfoTabButtonClass(isActive)}"
      type="button"
      role="tab"
      aria-selected="${isActive ? "true" : "false"}"
      data-zone-info-tab="${escapeHtml(tab.id)}"
      title="${escapeHtml(pointDetailTabTitle(tab))}"
      tabindex="${isActive ? "0" : "-1"}"
    >
      ${spriteIcon(tab.icon || "information-circle", "size-4")}
      <span class="truncate">${escapeHtml(tab.label)}</span>
    </button>
  `;
}

function pointDetailTabsMarkup(pointDetail) {
  const tabs = Array.isArray(pointDetail?.panes) ? pointDetail.panes : [];
  return tabs.map((tab) => pointDetailTabButtonMarkup(tab, pointDetail?.activePaneId)).join("");
}

function emptyPointDetailPanelMarkup() {
  return '<div class="rounded-box border border-base-300/70 bg-base-200 px-3 py-3 text-sm text-base-content/60">Click the map, use a waypoint target, or select a bookmark to inspect layers at a world point.</div>';
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

function renderPatchDropdown(dropdown, input, orderedPatches, selectedPatchId, emptyLabel) {
  if (!(dropdown instanceof HTMLElement) || !(input instanceof HTMLInputElement)) {
    return;
  }
  const results = dropdown.querySelector("[data-role='results']");
  const catalog = dropdown.querySelector("[data-role='selected-content-catalog']");
  const selectedContent = dropdown.querySelector("[data-role='selected-content']");
  if (!orderedPatches.length) {
    dropdown.label = emptyLabel;
    dropdown.value = "";
    input.value = "";
    setMarkup(
      selectedContent,
      `empty:${emptyLabel}`,
      renderSearchableDropdownTextContent(emptyLabel),
    );
    setMarkup(results, `empty:${emptyLabel}`, `<li class="menu-disabled"><span>${escapeHtml(emptyLabel)}</span></li>`);
    setMarkup(catalog, `empty:${emptyLabel}`, "");
    return;
  }

  const options = orderedPatches.map((patch) => {
    const name = patch.patchName || patch.patchId;
    const date = formatPatchDate(patch.startTsUtc);
    const label = date ? `${name} (${date})` : name;
    return {
      patchId: patch.patchId,
      contentHtml: renderPatchDropdownContentHtml(name, date),
      label,
      templateHtml: `<template data-role="selected-content" data-value="${escapeHtml(patch.patchId)}" data-label="${escapeHtml(label)}" data-search-text="${escapeHtml([name, date, patch.patchId].filter(Boolean).join(" "))}">${renderPatchDropdownContentHtml(name, date)}</template>`,
    };
  });

  const nextValue = selectedPatchId || orderedPatches[0].patchId;
  const selectedOption =
    options.find((option) => option.patchId === nextValue) || options[0];
  const optionRenderKey = JSON.stringify(options.map((option) => [option.patchId, option.label]));
  const resultsRenderKey = `${selectedOption.patchId}:${optionRenderKey}`;
  setMarkup(
    results,
    resultsRenderKey,
    options
      .map((option) => {
        const isSelected = option.patchId === selectedOption.patchId;
        return `<li><button type="button" class="justify-between gap-3 text-left${
          isSelected ? " menu-active" : ""
        }" data-searchable-dropdown-option data-value="${escapeHtml(option.patchId)}" data-label="${escapeHtml(option.label)}"><span data-role="option-content" class="flex min-w-0 flex-1 items-center gap-3">${option.contentHtml}</span>${
          isSelected
            ? '<span class="badge badge-soft badge-primary badge-xs">Selected</span>'
            : ""
        }</button></li>`;
      })
      .join(""),
  );
  setMarkup(catalog, optionRenderKey, options.map((option) => option.templateHtml).join(""));
  dropdown.label = selectedOption.label;
  dropdown.value = selectedOption.patchId;
  setMarkup(
    selectedContent,
    `selected:${selectedOption.patchId}:${selectedOption.label}`,
    selectedOption.contentHtml,
  );
  if (input.value !== selectedOption.patchId) {
    input.value = selectedOption.patchId;
  }
  if (typeof dropdown.isOpen === "function" && dropdown.isOpen()) {
    dropdown.refreshResults?.();
  }
}

function renderLayerStack(container, stateBundle, options = {}) {
  const layers = resolveLayerEntries(stateBundle);
  const expandedLayerIds =
    options.expandedLayerIds instanceof Set ? options.expandedLayerIds : new Set();
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
      layer.supportsWaypointConnections ? 1 : 0,
      layer.waypointConnectionsVisible ? 1 : 0,
      layer.supportsWaypointLabels ? 1 : 0,
      layer.waypointLabelsVisible ? 1 : 0,
      layer.supportsPointIcons ? 1 : 0,
      layer.pointIconsVisible ? 1 : 0,
      Math.round(clampPointIconScale(layer.pointIconScale) * 1000),
      Math.round(clampPointIconScale(layer.pointIconScaleDefault) * 1000),
      Number.isFinite(layer.displayOrder) ? layer.displayOrder : 0,
      layer.locked ? 1 : 0,
      expandedLayerIds.has(layer.layerId) ? 1 : 0,
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
      const settingsExpanded = expandedLayerIds.has(layer.layerId);
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
      const waypointControls = [];
      if (layer.supportsWaypointConnections) {
        waypointControls.push(`
          <label class="label cursor-pointer justify-start gap-3 py-0">
            <input
              class="toggle toggle-xs toggle-primary"
              data-layer-waypoint-connections="${layer.layerId.replace(/"/g, "&quot;")}"
              type="checkbox"
              ${layer.waypointConnectionsVisible ? "checked" : ""}
            >
            <span class="label-text text-xs text-base-content/70">Connections</span>
          </label>
        `);
      }
      if (layer.supportsWaypointLabels) {
        waypointControls.push(`
          <label class="label cursor-pointer justify-start gap-3 py-0">
            <input
              class="toggle toggle-xs toggle-primary"
              data-layer-waypoint-labels="${layer.layerId.replace(/"/g, "&quot;")}"
              type="checkbox"
              ${layer.waypointLabelsVisible ? "checked" : ""}
            >
            <span class="label-text text-xs text-base-content/70">Names</span>
          </label>
        `);
      }
      const pointControls = [];
      if (layer.supportsPointIcons) {
        pointControls.push(`
          <label class="label cursor-pointer justify-start gap-3 py-0">
            <input
              class="toggle toggle-xs toggle-primary"
              data-layer-point-icons="${layer.layerId.replace(/"/g, "&quot;")}"
              type="checkbox"
              ${layer.pointIconsVisible ? "checked" : ""}
            >
            <span class="label-text text-xs text-base-content/70">Icons</span>
          </label>
        `);
      }
      return `
        <article
          class="fishymap-layer-card card card-border bg-base-200"
          data-layer-id="${layer.layerId.replace(/"/g, "&quot;")}"
          data-indent-level="${indentLevel > 0 ? "1" : "0"}"
          data-locked="${locked ? "true" : "false"}"
          data-settings-expanded="${settingsExpanded ? "true" : "false"}"
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
            <div class="fishymap-layer-header">
              <span class="truncate text-sm font-semibold">${escapeHtml(layer.name)}</span>
            </div>
            ${
              settingsExpanded
                ? `
                  <div class="fishymap-layer-controls">
                    <div class="fishymap-layer-relations">
                      <span class="badge badge-ghost badge-xs">${kind}</span>
                      ${locked ? '<span class="badge badge-outline badge-xs">Ground</span>' : ""}
                      ${relationBadges.join("")}
                    </div>
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
                    ${
                      waypointControls.length
                        ? `
                          <fieldset class="fieldset">
                            <span class="fieldset-legend m-0 px-0 text-[11px] uppercase tracking-[0.18em] text-base-content/45">Waypoints</span>
                            <div class="flex flex-wrap items-center gap-x-4 gap-y-1">
                              ${waypointControls.join("")}
                            </div>
                          </fieldset>
                        `
                        : ""
                    }
                    ${
                      pointControls.length
                        ? `
                          <fieldset class="fieldset">
                            <span class="fieldset-legend m-0 px-0 text-[11px] uppercase tracking-[0.18em] text-base-content/45">Fish Evidence</span>
                            <div class="space-y-2">
                              <div class="flex flex-wrap items-center gap-x-4 gap-y-1">
                                ${pointControls.join("")}
                              </div>
                              <div class="space-y-2">
                                <div class="flex items-center justify-between gap-3">
                                  <span class="text-xs font-semibold text-base-content/70">Fish icon size</span>
                                  <span class="text-xs font-semibold text-base-content/60" data-layer-point-icon-scale-value>${pointIconScaleLabel(layer.pointIconScale)}</span>
                                </div>
                                <input
                                  class="range range-primary range-xs"
                                  data-layer-point-icon-scale="${layer.layerId.replace(/"/g, "&quot;")}"
                                  type="range"
                                  min="${FISHYMAP_POINT_ICON_SCALE_MIN}"
                                  max="${FISHYMAP_POINT_ICON_SCALE_MAX}"
                                  step="0.05"
                                  value="${pointIconScaleValue(layer.pointIconScale)}"
                                  aria-label="Fish icon size for ${escapeHtml(layer.name)}"
                                >
                              </div>
                            </div>
                          </fieldset>
                        `
                        : ""
                    }
                  </div>
                `
                : ""
            }
          </div>
          <button
            class="fishymap-layer-settings btn btn-sm btn-circle ${
              settingsExpanded ? "btn-soft btn-primary" : "btn-ghost"
            }"
            data-layer-settings-toggle="${layer.layerId.replace(/"/g, "&quot;")}"
            type="button"
            aria-label="${settingsExpanded ? "Hide" : "Show"} settings for ${escapeHtml(layer.name)}"
            aria-expanded="${settingsExpanded ? "true" : "false"}"
            title="${settingsExpanded ? "Hide" : "Show"} settings for ${escapeHtml(layer.name)}"
          >
            ${layerSettingsIcon()}
          </button>
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

function addSelectedSemanticFieldId(selectedFieldIds, fieldId) {
  return selectedFieldIds.includes(fieldId) ? selectedFieldIds : selectedFieldIds.concat(fieldId);
}

function removeSelectedSemanticFieldId(selectedFieldIds, fieldId) {
  return selectedFieldIds.filter((value) => value !== fieldId);
}

function updateSelectedSemanticFieldIdsByLayer(selectedByLayer, layerId, nextFieldIds) {
  const next = {
    ...normalizeSemanticFieldIdsByLayer(selectedByLayer),
  };
  const normalizedLayerId = String(layerId || "").trim();
  if (!normalizedLayerId) {
    return next;
  }
  if (!Array.isArray(nextFieldIds) || !nextFieldIds.length) {
    delete next[normalizedLayerId];
    return next;
  }
  next[normalizedLayerId] = nextFieldIds;
  return normalizeSemanticFieldIdsByLayer(next);
}

export function buildSearchMatches(stateBundle, searchText, zoneCatalog = []) {
  const catalogFish = stateBundle.state?.catalog?.fish || [];
  const semanticTerms = stateBundle.state?.catalog?.semanticTerms || [];
  const selectedFishIds = new Set(resolveSelectedFishIds(stateBundle));
  const selectedFishFilterTerms = resolveSelectedFishFilterTerms(stateBundle);
  const selectedSemanticFieldIdsByLayer = resolveSelectedSemanticFieldIdsByLayer(stateBundle);
  const selectedZoneRgbs = new Set(resolveSelectedZoneRgbs(stateBundle));
  const sharedFishState = resolveSharedFishState(stateBundle);
  const filterDirectives = parseFishFilterDirectives(searchText);
  const effectiveFishFilterTerms = normalizeFishFilterTerms(
    selectedFishFilterTerms.concat(filterDirectives.directTerms),
  );
  const fishFilterMatches = findFishFilterMatches(searchText, selectedFishFilterTerms);
  const fishMatches = findFishMatches(catalogFish, filterDirectives.remainingQuery, {
    includeAllWhenEmpty: effectiveFishFilterTerms.length > 0,
    filterTerms: effectiveFishFilterTerms,
    sharedFishState,
  })
    .filter((fish) => !selectedFishIds.has(fish.fishId))
    .map((fish) => ({
      kind: "fish",
      ...fish,
    }));
  const zoneMatches = findZoneMatches(zoneCatalog, filterDirectives.remainingQuery).filter(
    (zone) => !selectedZoneRgbs.has(zone.zoneRgb),
  );
  const semanticMatches = findSemanticMatches(semanticTerms, filterDirectives.remainingQuery).filter(
    (term) => !(selectedSemanticFieldIdsByLayer[term.layerId] || []).includes(term.fieldId),
  );
  return fishFilterMatches.concat(fishMatches, zoneMatches, semanticMatches).sort((left, right) => {
    const leftPriority = searchMatchPriority(left);
    const rightPriority = searchMatchPriority(right);
    if (leftPriority !== rightPriority) {
      return leftPriority - rightPriority;
    }
    if (right._score !== left._score) {
      return right._score - left._score;
    }
    return String(left.name || left.label || "").localeCompare(
      String(right.name || right.label || ""),
    );
  });
}

function searchMatchPriority(match) {
  if (match?.kind === "fish-filter") {
    return -1;
  }
  if (match?.kind === "fish") {
    return 0;
  }
  if (match?.kind === "zone") {
    return 1;
  }
  if (match?.kind === "semantic") {
    const parsed = parseSemanticIdentityText(match.label || "");
    if (parsed?.kind === "RG") {
      return 2;
    }
    if (parsed?.kind === "N") {
      return 3;
    }
    if (parsed?.kind === "R") {
      return 3;
    }
    if (String(match.layerId || "").trim() === "region_groups") {
      return 2;
    }
    if (String(match.layerId || "").trim() === "regions") {
      return 3;
    }
    return 4;
  }
  return 9;
}

function semanticTermLookupKey(layerId, fieldId) {
  return `${String(layerId || "").trim()}:${Number.parseInt(fieldId, 10)}`;
}

function buildSemanticTermLookup(stateBundle) {
  return new Map(
    (stateBundle.state?.catalog?.semanticTerms || []).map((term) => [
      semanticTermLookupKey(term.layerId, term.fieldId),
      term,
    ]),
  );
}

export function renderSearchSelection(elements, stateBundle, fishLookup) {
  const selectedFishIds = resolveSelectedFishIds(stateBundle);
  const selectedFishFilterTerms = resolveSelectedFishFilterTerms(stateBundle);
  const selectedSemanticFieldIdsByLayer = resolveSelectedSemanticFieldIdsByLayer(stateBundle);
  const selectedZoneRgbs = resolveSelectedZoneRgbs(stateBundle);
  const hasSelection =
    selectedFishIds.length > 0 ||
    selectedFishFilterTerms.length > 0 ||
    selectedZoneRgbs.length > 0;
  const zoneLookup = new Map(
    (elements.zoneCatalog || []).map((zone) => [zone.zoneRgb, zone]),
  );
  const semanticLookup = buildSemanticTermLookup(stateBundle);
  const selectedSemanticEntries = Object.entries(selectedSemanticFieldIdsByLayer)
    .filter(([layerId]) => layerId !== "zone_mask")
    .flatMap(([layerId, fieldIds]) =>
      fieldIds.map((fieldId) => ({
        layerId,
        fieldId,
        term: semanticLookup.get(semanticTermLookupKey(layerId, fieldId)) || null,
      })),
    );
  const hasSemanticSelection = selectedSemanticEntries.length > 0;
  const hasAnySelection = hasSelection || hasSemanticSelection;
  const renderKey = JSON.stringify({
    selectedFishFilterTerms,
    selectedFishIds,
    selectedZoneRgbs,
    selectedSemantic: selectedSemanticEntries.map(({ layerId, fieldId, term }) => [
      layerId,
      fieldId,
      term?.label || "",
      term?.description || "",
      term?.layerName || "",
    ]),
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
    elements.searchSelection.hidden = !hasAnySelection;
    if (elements.searchSelectionShell) {
      elements.searchSelectionShell.hidden = !hasAnySelection;
    }
    if (elements.searchWindow) {
      elements.searchWindow.dataset.hasSelection = hasAnySelection ? "true" : "false";
    }
    return;
  }
  elements.searchSelection.dataset.renderKey = renderKey;

  if (!hasAnySelection) {
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

  elements.searchSelection.innerHTML = selectedFishFilterTerms
    .map((fishFilterTerm) => {
      const metadata = FISH_FILTER_TERM_METADATA[fishFilterTerm];
      const label = metadata?.label || fishFilterTerm;
      return `
        <div class="join items-center rounded-full border border-base-300 bg-base-100 p-1 text-base-content">
          <span class="inline-flex min-w-0 items-center gap-2 px-2 text-sm">
            ${fishFilterTermIconMarkup(fishFilterTerm)}
            <span class="font-medium">${escapeHtml(label)}</span>
          </span>
          <button
            class="fishymap-selection-remove btn btn-ghost btn-xs btn-circle join-item h-7 min-h-0 w-7 border-0 text-base-content/70"
            data-fish-filter-term="${escapeHtml(fishFilterTerm)}"
            type="button"
            aria-label="Remove ${escapeHtml(label)}"
          >
            ×
          </button>
        </div>
      `;
    })
    .concat(selectedFishIds
    .map((fishId) => {
      const fish = fishLookup.get(fishId);
      const name = fish?.name || `Fish ${fishId}`;
      const fishMarkup =
        fishIdentityMarkup({ ...fish, fishId, name }, { interactive: true }) ||
        `<span class="truncate max-w-36">${escapeHtml(name)}</span>`;
      return `
        <div class="join items-center rounded-full border border-base-300 bg-base-100 p-1 text-base-content">
          <span class="inline-flex min-w-0 items-center gap-2 px-2 text-sm">${fishMarkup}</span>
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
    }))
    .concat(
      selectedZoneRgbs.map((zoneRgb) => {
        const zone = zoneLookup.get(zoneRgb);
        const name = zone?.name || `Zone ${formatZone(zoneRgb)}`;
        const zoneMarkup =
          zoneIdentityMarkup(
            {
              zoneRgb,
              name,
              r: zone?.r,
              g: zone?.g,
              b: zone?.b,
            },
            { interactive: true },
          ) || `<span class="truncate max-w-40">${escapeHtml(name)}</span>`;
        return `
          <div class="join items-center rounded-full border border-base-300 bg-base-100 p-1 text-base-content">
            <span class="inline-flex min-w-0 items-center gap-2 px-2 text-sm">${zoneMarkup}</span>
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
    .concat(
      selectedSemanticEntries.map(({ layerId, fieldId, term }) => {
        const name = term?.label || `Field ${fieldId}`;
        const description = term?.description || "";
        const semanticMarkup =
          semanticIdentityMarkup(name, { interactive: true }) ||
          `<span class="truncate max-w-40">${escapeHtml(name)}</span>`;
        return `
          <div class="join items-center rounded-full border border-base-300 bg-base-100 p-1 text-base-content">
            <span class="inline-flex min-w-0 items-center gap-2 px-2 text-sm">
              <span class="min-w-0">${semanticMarkup}</span>
              ${
                description
                  ? `<span class="truncate max-w-40 text-xs text-base-content/55">${escapeHtml(description)}</span>`
                  : ""
              }
            </span>
            <button
              class="fishymap-selection-remove btn btn-ghost btn-xs btn-circle join-item h-7 min-h-0 w-7 border-0 text-base-content/70"
              data-semantic-layer-id="${escapeHtml(layerId)}"
              data-semantic-field-id="${fieldId}"
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

export function renderSearchResults(elements, matches, stateBundle) {
  const query = String(stateBundle.inputState?.filters?.searchText || "").trim();
  const showResults = matches.length > 0;
  const activeMatches = matches.slice(0, 12);
  const renderKey = JSON.stringify({
    query,
    results: activeMatches.map((match) =>
      match.kind === "fish-filter"
        ? ["fish-filter", match.term, match.label || "", match.description || ""]
        : match.kind === "zone"
        ? ["zone", match.zoneRgb, match.name, match.rgbKey]
        : match.kind === "semantic"
          ? [
              "semantic",
              match.layerId,
              match.fieldId,
              match.label || "",
              match.description || "",
              match.layerName || "",
            ]
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
      if (match.kind === "fish-filter") {
        return `
          <li>
            <div
              class="flex cursor-pointer items-start gap-3 rounded-box px-3 py-2 text-sm"
              data-fish-filter-term="${escapeHtml(match.term)}"
              role="button"
              tabindex="0"
              aria-label="Add ${escapeHtml(match.label || match.term)}"
              title="Add ${escapeHtml(match.label || match.term)}"
            >
              <span class="min-w-0 flex-1 text-left">
                <span class="flex items-center gap-2">
                  ${fishFilterTermIconMarkup(match.term)}
                  <span class="font-semibold">${escapeHtml(match.label || match.term)}</span>
                </span>
                <span class="mt-1 block truncate text-xs text-base-content/60">
                  ${escapeHtml(match.description || "")}
                </span>
              </span>
            </div>
          </li>
        `;
      }
      if (match.kind === "zone") {
        const zoneMarkup =
          zoneIdentityMarkup(match, { interactive: true }) ||
          `<span class="truncate">${escapeHtml(match.name)}</span>`;
        return `
        <li>
          <div
            class="flex cursor-pointer items-start gap-3 rounded-box px-3 py-2 text-sm"
            data-zone-rgb="${match.zoneRgb}"
            role="button"
            tabindex="0"
            aria-label="Add ${escapeHtml(match.name)}"
            title="Add ${escapeHtml(match.name)}"
          >
            <span class="min-w-0 flex-1 text-left">
              <span class="flex items-center gap-2">
                ${zoneMarkup}
                <span class="badge badge-outline badge-xs">Zone</span>
              </span>
              <span class="block truncate text-xs text-base-content/60">
                <code>${escapeHtml(match.rgbKey)}</code>
                <span class="ml-2">${escapeHtml(formatZone(match.zoneRgb))}</span>
              </span>
            </span>
          </div>
        </li>
      `;
      }
      if (match.kind === "semantic") {
        const semanticLabel = match.label || `Field ${match.fieldId}`;
        const semanticMarkup =
          semanticIdentityMarkup(semanticLabel, { interactive: true }) ||
          `<span class="truncate">${escapeHtml(semanticLabel)}</span>`;
        return `
          <li>
            <div
              class="flex cursor-pointer items-start gap-3 rounded-box px-3 py-2 text-sm"
              data-semantic-layer-id="${escapeHtml(match.layerId)}"
              data-semantic-field-id="${match.fieldId}"
              data-semantic-label="${escapeHtml(semanticLabel)}"
              role="button"
              tabindex="0"
              aria-label="Add ${escapeHtml(semanticLabel)}"
              title="Add ${escapeHtml(semanticLabel)}"
            >
              <span class="min-w-0 flex-1 text-left">
                <span class="block">${semanticMarkup}</span>
                <span class="mt-1 block truncate text-xs text-base-content/60">
                  ${escapeHtml(match.description || `Field ${match.fieldId}`)}
                </span>
              </span>
            </div>
          </li>
        `;
      }
      return `
        <li>
          <div
            class="flex cursor-pointer items-start gap-3 rounded-box px-3 py-2 text-sm"
            data-fish-id="${match.fishId}"
            role="button"
            tabindex="0"
            aria-label="Add ${escapeHtml(match.name)}"
            title="Add ${escapeHtml(match.name)}"
          >
            <span class="min-w-0 flex-1 text-left">
              ${fishIdentityMarkup(match, { interactive: true })}
            </span>
          </div>
        </li>
      `;
    })
    .join("");
}

function renderZoneInfoWindow(elements, stateBundle, windowUiState, fishLookup) {
  if (
    !elements.zoneInfoTabs ||
    !elements.zoneInfoPanel ||
    !elements.zoneInfoTitle ||
    !elements.zoneInfoTitleIcon ||
    !elements.zoneInfoStatusIcon ||
    !elements.zoneInfoStatusText
  ) {
    return;
  }
  const selection = stateBundle.state?.selection || null;
  const pointDetail = buildPointDetailViewModel(selection, stateBundle, windowUiState);
  const descriptor = pointDetail.descriptor;
  const tabs = pointDetail.panes;
  const activeTab = pointDetail.activePaneId;
  const activeLayerTab = pointDetail.activePane;

  if (elements.zoneInfoWindow) {
    elements.zoneInfoWindow.dataset.activeTab = activeTab || "";
    elements.zoneInfoWindow.dataset.pointKind = descriptor.pointKind || "";
  }
  if (elements.zoneInfoBody) {
    elements.zoneInfoBody.dataset.activeTab = activeTab || "";
    elements.zoneInfoBody.dataset.pointKind = descriptor.pointKind || "";
  }

  setTextContent(elements.zoneInfoTitle, descriptor.title);
  setMarkup(
    elements.zoneInfoTitleIcon,
    descriptor.titleIcon,
    spriteIcon(descriptor.titleIcon, "size-4"),
  );
  setMarkup(
    elements.zoneInfoStatusIcon,
    descriptor.statusIcon,
    spriteIcon(descriptor.statusIcon, "size-4"),
  );
  setTextContent(elements.zoneInfoStatusText, descriptor.statusText);

  setBooleanProperty(elements.zoneInfoTabs, "hidden", tabs.length === 0);
  setMarkup(
    elements.zoneInfoTabs,
    JSON.stringify({
      activeTab,
      tabs: tabs.map((tab) => [tab.id, tab.label, tab.summary]),
    }),
    pointDetailTabsMarkup(pointDetail),
  );

  if (!selection || !activeLayerTab) {
    setMarkup(
      elements.zoneInfoPanel,
      JSON.stringify({ empty: true, title: descriptor.title }),
      emptyPointDetailPanelMarkup(),
    );
    return;
  }

  setMarkup(
    elements.zoneInfoPanel,
    pointDetailPaneMarkupKey(activeLayerTab),
    pointDetailPaneMarkup(activeLayerTab, fishLookup),
  );
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
      state: windowUiState.bookmarks,
      root: elements.bookmarksWindow,
      body: elements.bookmarksBody,
      titlebar: elements.bookmarksTitlebar,
      toggle: elements.bookmarksWindowToggle,
      label: "Bookmarks",
    },
    {
      state: windowUiState.zoneInfo,
      root: elements.zoneInfoWindow,
      body: elements.zoneInfoBody,
      titlebar: elements.zoneInfoTitlebar,
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

function syncLayerPointIconScaleControl(container, layerId, scale) {
  if (!container || !layerId) {
    return false;
  }
  const slider = Array.from(container.querySelectorAll("input[data-layer-point-icon-scale]")).find(
    (candidate) => candidate.getAttribute("data-layer-point-icon-scale") === layerId,
  );
  if (!slider) {
    return false;
  }
  const normalized = clampPointIconScale(scale);
  const value = pointIconScaleValue(normalized);
  if (slider.value !== value) {
    slider.value = value;
  }
  const label = slider
    .closest(".fieldset")
    ?.querySelector?.("[data-layer-point-icon-scale-value]");
  if (label) {
    setTextContent(label, pointIconScaleLabel(normalized));
  }
  return true;
}

function renderPanel(
  elements,
  stateBundle,
  zoneCatalog = [],
  windowUiState = DEFAULT_WINDOW_UI_STATE,
  searchUiState = { open: false },
) {
  const state = stateBundle.state || {};
  const inputState = stateBundle.inputState || {};
  const renderStateBundle = {
    ...stateBundle,
    zoneCatalog,
  };
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
  const autoAdjustView = windowUiState?.settings?.autoAdjustView !== false;
  const patchEmptyLabel = isReady ? "No patches available" : "Loading patches…";
  const fishLookup = mergeZoneEvidenceIntoFishLookup(buildFishLookup(catalogFish), isReady ? state.selection?.zoneStats || null : null);
  elements.zoneCatalog = zoneCatalog;

  applyThemeToShell(elements.shell);

  setTextContent(elements.readyPill, state.ready ? "Ready" : "Loading");
  setClassName(
    elements.readyPill,
    `badge badge-sm ${state.ready ? "badge-success" : "badge-outline"}`,
  );
  renderViewState(elements, state);
  if (elements.autoAdjustView) {
    setBooleanProperty(elements.autoAdjustView, "checked", autoAdjustView);
  }

  if (elements.search.value !== searchText) {
    elements.search.value = searchText;
  }

  renderPatchDropdown(
    elements.patchFromPicker,
    elements.patchFrom,
    patchRange.ordered,
    patchRange.fromPatchId,
    patchEmptyLabel,
  );
  renderPatchDropdown(
    elements.patchToPicker,
    elements.patchTo,
    patchRange.ordered,
    patchRange.toPatchId,
    patchEmptyLabel,
  );
  const activeOpacityInteraction =
    isReady &&
    elements.layerOpacityInteraction?.activeLayerId &&
    Number.isFinite(elements.layerOpacityInteraction?.activeValue) &&
    syncLayerOpacityControl(
      elements.layers,
      elements.layerOpacityInteraction.activeLayerId,
      elements.layerOpacityInteraction.activeValue,
    );
  const activePointIconScaleInteraction =
    !activeOpacityInteraction &&
    isReady &&
    elements.layerPointIconScaleInteraction?.activeLayerId &&
    Number.isFinite(elements.layerPointIconScaleInteraction?.activeValue) &&
    syncLayerPointIconScaleControl(
      elements.layers,
      elements.layerPointIconScaleInteraction.activeLayerId,
      elements.layerPointIconScaleInteraction.activeValue,
    );
  if (!activeOpacityInteraction && !activePointIconScaleInteraction) {
    renderLayerStack(
      elements.layers,
      isReady ? stateBundle : { state: { catalog: { layers: [] } }, inputState: {} },
      { expandedLayerIds: elements.layerSettingsExpanded || new Set() },
    );
  }
  if (elements.layersCount) {
    setTextContent(elements.layersCount, String(isReady ? (state.catalog?.layers || []).length : 0));
  }

  const matches =
    isReady && searchUiState.open
      ? String(searchText || "").trim()
        ? buildSearchMatches(renderStateBundle, searchText, zoneCatalog)
        : buildDefaultFishFilterMatches(renderStateBundle)
      : [];
  renderSearchSelection(elements, renderStateBundle, fishLookup);
  renderSearchResults(elements, matches, renderStateBundle);

  if (elements.legend) {
    setBooleanProperty(elements.legend, "open", Boolean(inputState.ui?.legendOpen));
  }
  if (elements.diagnostics) {
    setBooleanProperty(elements.diagnostics, "open", Boolean(inputState.ui?.diagnosticsOpen));
  }

  renderZoneInfoWindow(elements, renderStateBundle, windowUiState, fishLookup);

  renderHoverTooltip(elements, state.hover || null, renderStateBundle);

  renderStatusLines(elements.statusLines, state.statuses || {});
  setTextContent(
    elements.diagnosticJson,
    JSON.stringify(state.lastDiagnostic || state.statuses || {}, null, 2),
  );
}

function applySearchMatchSelection(
  shell,
  elements,
  renderCurrentState,
  patchInputState,
  stateBundle,
  match,
) {
  if (!match) {
    return;
  }
  elements.search.value = "";
  const selectedSemanticFieldIdsByLayer = resolveSelectedSemanticFieldIdsByLayer(stateBundle);
  const selectedFishFilterTerms = resolveSelectedFishFilterTerms(stateBundle);
  const patch = {
    version: 1,
    filters: {
      searchText: "",
      ...(match.kind === "fish-filter"
        ? {
            fishFilterTerms: addSelectedFishFilterTerm(selectedFishFilterTerms, match.term),
          }
        : match.kind === "fish"
        ? { fishIds: addSelectedFishId(resolveSelectedFishIds(stateBundle), match.fishId) }
        : match.kind === "zone"
          ? {
              semanticFieldIdsByLayer: updateSelectedSemanticFieldIdsByLayer(
                selectedSemanticFieldIdsByLayer,
                "zone_mask",
                addSelectedZoneRgb(resolveSelectedZoneRgbs(stateBundle), match.zoneRgb),
              ),
            }
          : {
              semanticFieldIdsByLayer: updateSelectedSemanticFieldIdsByLayer(
                selectedSemanticFieldIdsByLayer,
                match.layerId,
                addSelectedSemanticFieldId(
                  selectedSemanticFieldIdsByLayer[match.layerId] || [],
                  match.fieldId,
                ),
              ),
            }),
    },
  };
  patchInputState(patch);
  if (match.kind === "semantic") {
    dispatchMapCommand(shell, {
      selectSemanticField: {
        layerId: match.layerId,
        fieldId: match.fieldId,
      },
    });
  }
  renderCurrentState(projectStateBundleStatePatch(stateBundle, patch));
}

function bindUi(shell, elements, options = {}) {
  let isRendering = false;
  let latestStateBundle = requestBridgeState(shell);
  const initialMapUiState = currentMapUiSignalState();
  const searchUiState = {
    open: initialMapUiState.search.open,
  };
  let zoneCatalog = normalizeZoneCatalog(options.zoneCatalog);
  let windowUiState = initialMapUiState.windowUi;
  let bookmarks = currentMapBookmarksSignalState();
  let bookmarkMetadataRefreshTimer = 0;
  let bookmarkMetadataRefreshAttempts = 0;
  const bookmarkUi = {
    placing: initialMapUiState.bookmarks.placing,
    selectedIds: normalizeSelectedBookmarkIds(bookmarks, initialMapUiState.bookmarks.selectedIds),
  };
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
    bookmarks: {
      root: elements.bookmarksWindow,
      body: elements.bookmarksBody,
      titlebar: elements.bookmarksTitlebar,
      toggle: elements.bookmarksWindowToggle,
    },
    zoneInfo: {
      root: elements.zoneInfoWindow,
      body: elements.zoneInfoBody,
      titlebar: elements.zoneInfoTitlebar,
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
    bookmarks: "bookmarks",
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
  const bookmarkDragState = {
    draggingBookmarkId: null,
    overBookmarkId: null,
    mode: null,
  };
  const dragAutoScrollState = {
    container: null,
    clientY: null,
    frameId: 0,
  };
  const layerOpacityInteraction = {
    activeLayerId: null,
    activeValue: null,
  };
  const layerPointIconScaleInteraction = {
    activeLayerId: null,
    activeValue: null,
  };
  const layerSettingsExpanded = new Set();
  elements.layerOpacityInteraction = layerOpacityInteraction;
  elements.layerPointIconScaleInteraction = layerPointIconScaleInteraction;
  elements.layerSettingsExpanded = layerSettingsExpanded;

  function autoAdjustViewEnabled() {
    return windowUiState?.settings?.autoAdjustView !== false;
  }

  function currentViewportSize() {
    return measureMapViewportSize(elements);
  }

  async function buildSemanticFocusCommandFromCode(code) {
    const identity = parseSemanticIdentityText(code);
    if (!identity) {
      return null;
    }
    const latest = getLatestStateBundle();
    let focusIndex = null;
    if (autoAdjustViewEnabled() || identity.kind === "N") {
      focusIndex = await loadWaypointFocusIndex(globalThis.window?.location);
    }
    return buildSemanticIdentityCommand(
      identity,
      focusIndex,
      latest,
      currentViewportSize(),
      { autoAdjustView: autoAdjustViewEnabled() },
    );
  }

  async function buildZoneFocusCommandFromRgb(zoneRgb) {
    const numericZoneRgb = Number.parseInt(zoneRgb, 10);
    if (!Number.isFinite(numericZoneRgb)) {
      return null;
    }
    const latest = getLatestStateBundle();
    let zoneFocusIndex = null;
    if (autoAdjustViewEnabled()) {
      zoneFocusIndex = await loadZoneFocusIndex(globalThis.window?.location);
    }
    return buildZoneIdentityCommand(
      numericZoneRgb,
      zoneFocusIndex,
      latest,
      currentViewportSize(),
      { autoAdjustView: autoAdjustViewEnabled() },
    );
  }

  async function buildFishFocusCommandFromId(fishId) {
    const numericFishId = Number.parseInt(fishId, 10);
    if (!Number.isFinite(numericFishId)) {
      return null;
    }
    const latest = getLatestStateBundle();
    const snapshot = await loadFishEvidenceSnapshot(globalThis.window?.location);
    return buildFishEvidenceFocusCommand(
      numericFishId,
      snapshot,
      latest,
      currentViewportSize(),
      { autoAdjustView: autoAdjustViewEnabled() },
    );
  }

  async function dispatchSemanticFocusFromCode(code) {
    if (!code) {
      return;
    }
    try {
      const command = await buildSemanticFocusCommandFromCode(code);
      if (command) {
        dispatchMapCommand(shell, command);
      }
    } catch (error) {
      console.error("Failed to resolve semantic focus command", error);
      showSiteToast("error", "Unable to load waypoint focus data.");
    }
  }

  async function dispatchZoneFocusFromRgb(zoneRgb) {
    if (!Number.isFinite(Number.parseInt(zoneRgb, 10))) {
      return;
    }
    try {
      const command = await buildZoneFocusCommandFromRgb(zoneRgb);
      if (command) {
        dispatchMapCommand(shell, command);
      }
    } catch (error) {
      console.error("Failed to resolve zone focus command", error);
      showSiteToast("error", "Unable to load zone focus data.");
    }
  }

  async function dispatchFishFocusFromId(fishId) {
    if (!Number.isFinite(Number.parseInt(fishId, 10))) {
      return;
    }
    try {
      const command = await buildFishFocusCommandFromId(fishId);
      if (command) {
        dispatchMapCommand(shell, command);
      }
    } catch (error) {
      console.error("Failed to resolve fish focus command", error);
      showSiteToast("error", "Unable to load fish evidence focus data.");
    }
  }

  function stopDragAutoScroll() {
    if (dragAutoScrollState.frameId && typeof window.cancelAnimationFrame === "function") {
      window.cancelAnimationFrame(dragAutoScrollState.frameId);
    }
    dragAutoScrollState.container = null;
    dragAutoScrollState.clientY = null;
    dragAutoScrollState.frameId = 0;
  }

  function tickDragAutoScroll() {
    dragAutoScrollState.frameId = 0;
    const container = dragAutoScrollState.container;
    if (!container || !Number.isFinite(dragAutoScrollState.clientY)) {
      return;
    }
    const delta = computeDragAutoScrollDelta(
      container.getBoundingClientRect(),
      dragAutoScrollState.clientY,
    );
    if (!delta) {
      return;
    }
    const maxScrollTop = Math.max(0, container.scrollHeight - container.clientHeight);
    const nextScrollTop = Math.max(0, Math.min(maxScrollTop, container.scrollTop + delta));
    if (nextScrollTop === container.scrollTop) {
      return;
    }
    container.scrollTop = nextScrollTop;
    dragAutoScrollState.frameId = window.requestAnimationFrame(tickDragAutoScroll);
  }

  function updateDragAutoScroll(container, clientY) {
    dragAutoScrollState.container = container || null;
    dragAutoScrollState.clientY = Number.isFinite(clientY) ? clientY : null;
    if (!container || !Number.isFinite(dragAutoScrollState.clientY)) {
      stopDragAutoScroll();
      return;
    }
    const delta = computeDragAutoScrollDelta(container.getBoundingClientRect(), dragAutoScrollState.clientY);
    if (!delta) {
      if (dragAutoScrollState.frameId && typeof window.cancelAnimationFrame === "function") {
        window.cancelAnimationFrame(dragAutoScrollState.frameId);
      }
      dragAutoScrollState.frameId = 0;
      return;
    }
    if (!dragAutoScrollState.frameId) {
      dragAutoScrollState.frameId = window.requestAnimationFrame(tickDragAutoScroll);
    }
  }

  function stateBundleFromEvent(event) {
    return withSharedFishState({
      state: event.detail?.state || FishyMapBridge.getCurrentState(),
      inputState:
        event.detail?.inputState ||
        (typeof FishyMapBridge.getCurrentInputState === "function"
          ? FishyMapBridge.getCurrentInputState()
          : {}),
    });
  }

  function getLatestStateBundle(options = {}) {
    if (options.refresh !== true && latestStateBundle) {
      latestStateBundle = withSharedFishState(latestStateBundle);
      return latestStateBundle;
    }
    latestStateBundle = withSharedFishState(requestBridgeState(shell, options));
    return latestStateBundle;
  }

  function dispatchStatePatchAndRender(patch) {
    patchMapInputSignalState(patch);
  }

  function syncActiveDetailPaneState(activePaneId) {
    const normalizedActivePaneId = normalizeNullableString(activePaneId);
    const currentActivePaneId = normalizeNullableString(
      getLatestStateBundle().inputState?.ui?.activeDetailPaneId,
    );
    if (normalizedActivePaneId === currentActivePaneId) {
      return;
    }
    const patch = {
      version: 1,
      ui: {
        activeDetailPaneId: normalizedActivePaneId,
      },
    };
    patchMapInputSignalState(patch);
  }

  function activateBookmarkSelection(bookmark) {
    if (!bookmark) {
      return;
    }
    setSelectedBookmarkIds([bookmark.id]);
    renderBookmarkManager(elements, latestStateBundle, bookmarks, bookmarkUi);
    const command = buildFocusCommandForWorldPoint(
      bookmark.worldX,
      bookmark.worldZ,
      getLatestStateBundle(),
      currentViewportSize(),
      {
      pointKind: "bookmark",
      pointLabel: bookmarkDisplayLabel(bookmark),
        autoAdjustView: autoAdjustViewEnabled(),
      },
    );
    if (command) {
      dispatchMapCommand(shell, command);
    }
  }

  function setSelectedBookmarkIds(nextSelectedIds) {
    bookmarkUi.selectedIds = normalizeSelectedBookmarkIds(bookmarks, nextSelectedIds);
    patchMapUiSignalState({ bookmarks: bookmarkUi });
    patchMapInputSignalState({
      version: FISHYMAP_CONTRACT_VERSION,
      ui: {
        bookmarkSelectedIds: bookmarkUi.selectedIds,
      },
    });
  }

  function selectedBookmarksForCopy() {
    return selectedBookmarksInOrder(bookmarks, bookmarkUi.selectedIds);
  }

  function bookmarksForExport() {
    const selectedBookmarks = selectedBookmarksForCopy();
    return selectedBookmarks.length ? selectedBookmarks : normalizeBookmarks(bookmarks);
  }

  function syncBookmarksToBridge(nextBookmarks = bookmarks) {
    patchMapInputSignalState({
      version: FISHYMAP_CONTRACT_VERSION,
      ui: {
        bookmarkSelectedIds: normalizeSelectedBookmarkIds(nextBookmarks, bookmarkUi.selectedIds),
        bookmarks: normalizeBookmarks(nextBookmarks),
      },
    });
  }

  function setBookmarkPlacementActive(active, options = {}) {
    bookmarkUi.placing = Boolean(active);
    patchMapUiSignalState({ bookmarks: bookmarkUi });
    renderBookmarkManager(elements, latestStateBundle, bookmarks, bookmarkUi);
  }

  function persistBookmarksAndRender(nextBookmarks, statusMessage = "", options = {}) {
    bookmarks = normalizeBookmarks(nextBookmarks);
    patchMapBookmarksSignalState(bookmarks);
    setSelectedBookmarkIds(
      Array.isArray(options.selectedIds) ? options.selectedIds : bookmarkUi.selectedIds,
    );
    syncBookmarksToBridge(bookmarks);
    const normalizedStatusMessage = String(statusMessage || "").trim();
    if (normalizedStatusMessage) {
      showSiteToast(options.toastTone || "success", normalizedStatusMessage);
    }
    renderCurrentState(getLatestStateBundle());
  }

  function clearBookmarkMetadataRefresh() {
    if (!bookmarkMetadataRefreshTimer) {
      return;
    }
    globalThis.clearTimeout?.(bookmarkMetadataRefreshTimer);
    bookmarkMetadataRefreshTimer = 0;
  }

  function scheduleBookmarkMetadataRefresh() {
    if (!bookmarksNeedDerivedMetadata(bookmarks)) {
      bookmarkMetadataRefreshAttempts = 0;
      clearBookmarkMetadataRefresh();
      return;
    }
    if (bookmarkMetadataRefreshTimer || bookmarkMetadataRefreshAttempts >= 20) {
      return;
    }
    bookmarkMetadataRefreshTimer = globalThis.setTimeout(() => {
      bookmarkMetadataRefreshTimer = 0;
      bookmarkMetadataRefreshAttempts += 1;
      renderCurrentState(getLatestStateBundle());
    }, 150);
  }

  function persistWindowUiState() {
    patchMapUiSignalState({ windowUi: windowUiState });
  }

  function updateWindowUiEntry(windowId, patch) {
    if (!hasOwnKey(managedWindows, windowId)) {
      return false;
    }
    const currentEntry = windowUiState[windowId] || DEFAULT_WINDOW_UI_STATE[windowId];
    const nextEntry =
      windowId === "zoneInfo"
        ? normalizeZoneInfoWindowUiEntry(
            { ...currentEntry, ...patch },
            DEFAULT_WINDOW_UI_STATE.zoneInfo,
          )
        : windowId === "settings"
          ? normalizeSettingsWindowUiEntry(
              { ...currentEntry, ...patch },
              DEFAULT_WINDOW_UI_STATE.settings,
            )
        : normalizeWindowUiEntry(
            { ...currentEntry, ...patch },
            DEFAULT_WINDOW_UI_STATE[windowId],
          );
    if (windowUiEntriesEqual(currentEntry, nextEntry)) {
      return false;
    }
    windowUiState = {
      ...windowUiState,
      [windowId]: nextEntry,
    };
    patchMapUiSignalState({ windowUi: windowUiState });
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
    } else if (windowId === "bookmarks") {
      setBookmarkPlacementActive(false);
    }
    applyManagedWindows({ persist: true });
  }

  function setZoneInfoTab(nextTab) {
    const requestedTab = normalizeZoneInfoTab(nextTab);
    const current = getLatestStateBundle();
    const selection = current.state?.selection || null;
    const availableTabs = buildPointDetailViewModel(
      selection,
      current,
      windowUiState,
    ).panes.map((tab) => tab.id);
    if (!requestedTab || !availableTabs.includes(requestedTab)) {
      return false;
    }
    if (!updateWindowUiEntry("zoneInfo", { tab: requestedTab })) {
      return false;
    }
    persistWindowUiState();
    renderCurrentState(current);
    return true;
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

  function renderCurrentState(stateBundle = latestStateBundle || requestBridgeState(shell)) {
    latestStateBundle = withSharedFishState({
      ...(stateBundle || {}),
      zoneCatalog,
    });
    publishMapInputSignals(latestStateBundle.inputState);
    publishMapRuntimeSignals(latestStateBundle);
    const resolvedBookmarks = persistResolvedBookmarksFromStateBundle(
      latestStateBundle,
      bookmarks,
      bookmarkUi,
    );
    if (bookmarkListSignature(resolvedBookmarks) !== bookmarkListSignature(bookmarks)) {
      bookmarks = resolvedBookmarks;
      patchMapBookmarksSignalState(bookmarks);
    } else {
      bookmarks = resolvedBookmarks;
    }
    scheduleBookmarkMetadataRefresh();
    isRendering = true;
    try {
      renderPanel(elements, latestStateBundle, zoneCatalog, windowUiState, searchUiState);
      syncActiveDetailPaneState(
        resolveZoneInfoActiveTab(
          windowUiState,
          latestStateBundle.state?.selection || null,
          latestStateBundle,
        ),
      );
      renderBookmarkManager(elements, latestStateBundle, bookmarks, bookmarkUi);
      applyManagedWindows();
    } finally {
      isRendering = false;
    }
    if (latestStateBundle?.state?.ready === true) {
      scheduleWaypointFocusIndexPreload(globalThis.window?.location);
    }
  }

  function syncLocalUiStateFromSignals() {
    const nextSignalUiState = currentMapUiSignalState();
    const nextSignalBookmarks = currentMapBookmarksSignalState();
    const nextBookmarkSelectedIds = normalizeSelectedBookmarkIds(
      nextSignalBookmarks,
      nextSignalUiState.bookmarks.selectedIds,
    );
    const currentSignature = JSON.stringify({
      windowUi: windowUiState,
      search: searchUiState,
      bookmarksList: bookmarks,
      bookmarks: {
        placing: bookmarkUi.placing,
        selectedIds: bookmarkUi.selectedIds,
      },
    });
    const nextSignature = JSON.stringify({
      windowUi: nextSignalUiState.windowUi,
      search: nextSignalUiState.search,
      bookmarksList: nextSignalBookmarks,
      bookmarks: {
        placing: nextSignalUiState.bookmarks.placing,
        selectedIds: nextBookmarkSelectedIds,
      },
    });
    if (currentSignature === nextSignature) {
      return;
    }

    const previousWindowUiState = windowUiState;
    const previousBookmarkPlacing = bookmarkUi.placing;
    windowUiState = nextSignalUiState.windowUi;
    searchUiState.open = nextSignalUiState.search.open;
    bookmarks = nextSignalBookmarks;
    bookmarkUi.placing = nextSignalUiState.bookmarks.placing;
    bookmarkUi.selectedIds = nextBookmarkSelectedIds;
    syncBookmarksToBridge(bookmarks);

    for (const [toolbarWindowId, windowId] of Object.entries(toolbarTargetToWindowId)) {
      const previousOpen = previousWindowUiState?.[windowId]?.open !== false;
      const nextOpen = windowUiState?.[windowId]?.open !== false;
      if (!previousOpen && nextOpen) {
        bringManagedWindowToFront(windowId);
      }
      if (toolbarWindowId === "search" && previousOpen && !nextOpen) {
        elements.search?.blur?.();
      }
      if (toolbarWindowId === "bookmarks" && previousOpen && !nextOpen && previousBookmarkPlacing) {
        bookmarkUi.placing = false;
        patchMapUiSignalState({ bookmarks: bookmarkUi });
      }
    }

    renderCurrentState(getLatestStateBundle());
  }

  function syncBridgeInputStateFromSignals() {
    const nextSignalInputState = currentMapInputSignalState();
    const currentInputState = applyStatePatch(
      DEFAULT_MAP_INPUT_SIGNAL_STATE,
      getLatestStateBundle().inputState || {},
    );
    if (JSON.stringify(currentInputState) === JSON.stringify(nextSignalInputState)) {
      return;
    }

    FishyMapBridge.setState?.(nextSignalInputState);
    FishyMapBridge.flushPendingPatchNow?.();
    renderCurrentState({
      ...getLatestStateBundle(),
      inputState: nextSignalInputState,
    });
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

  function clearBookmarkDropState() {
    bookmarkDragState.overBookmarkId = null;
    bookmarkDragState.mode = null;
    elements.bookmarksList
      ?.querySelectorAll?.(".fishymap-bookmark-card[data-drop-position]")
      ?.forEach?.((card) => {
        delete card.dataset.dropPosition;
      });
  }

  function applyBookmarkDropState(targetBookmarkId, mode) {
    clearBookmarkDropState();
    bookmarkDragState.overBookmarkId = targetBookmarkId;
    bookmarkDragState.mode = mode;
    const card = Array.from(
      elements.bookmarksList?.querySelectorAll?.(".fishymap-bookmark-card") || [],
    ).find((candidate) => candidate.getAttribute("data-bookmark-id") === targetBookmarkId);
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
    layerOpacityInteraction.activeLayerId = null;
    layerOpacityInteraction.activeValue = null;
    renderCurrentState(getLatestStateBundle());
  }

  function setActiveLayerPointIconScale(slider) {
    if (!slider) {
      return;
    }
    const layerId = slider.getAttribute("data-layer-point-icon-scale");
    if (!layerId) {
      return;
    }
    layerPointIconScaleInteraction.activeLayerId = layerId;
    layerPointIconScaleInteraction.activeValue = clampPointIconScale(slider.value);
    syncLayerPointIconScaleControl(
      elements.layers,
      layerId,
      layerPointIconScaleInteraction.activeValue,
    );
  }

  function clearActiveLayerPointIconScale() {
    if (!layerPointIconScaleInteraction.activeLayerId) {
      return;
    }
    layerPointIconScaleInteraction.activeLayerId = null;
    layerPointIconScaleInteraction.activeValue = null;
    renderCurrentState(getLatestStateBundle());
  }

  function toggleLayerSettings(layerId) {
    if (!layerId) {
      return;
    }
    if (layerSettingsExpanded.has(layerId)) {
      layerSettingsExpanded.delete(layerId);
      if (layerOpacityInteraction.activeLayerId === layerId) {
        layerOpacityInteraction.activeLayerId = null;
        layerOpacityInteraction.activeValue = null;
      }
      if (layerPointIconScaleInteraction.activeLayerId === layerId) {
        layerPointIconScaleInteraction.activeLayerId = null;
        layerPointIconScaleInteraction.activeValue = null;
      }
    } else {
      layerSettingsExpanded.add(layerId);
    }
    renderCurrentState(getLatestStateBundle());
  }

  elements.canvas.addEventListener("pointermove", (event) => {
    elements.hoverPointerActive = true;
    setHoverTooltipPosition(elements, event.clientX, event.clientY);
  });

  elements.canvas.addEventListener("pointerleave", () => {
    elements.hoverPointerActive = false;
    renderHoverTooltip(elements, null, latestStateBundle);
  });

  elements.canvas.addEventListener("click", () => {
    const state = getLatestStateBundle().state;
    const hover = state.hover || null;
    if (!bookmarkUi.placing) {
      const hoveredBookmark = resolveHoveredBookmark(hover, latestStateBundle, bookmarks);
      if (hoveredBookmark) {
        activateBookmarkSelection(hoveredBookmark.bookmark);
      }
      return;
    }
    const worldX = normalizeBookmarkCoordinate(hover?.worldX);
    const worldZ = normalizeBookmarkCoordinate(hover?.worldZ);
    if (worldX == null || worldZ == null) {
      showSiteToast("warning", "Move the cursor over the ready 2D map and click again.");
      return;
    }
    const bookmark = createBookmarkFromPlacement(
      {
        worldX,
        worldZ,
        layerSamples: Array.isArray(hover?.layerSamples) ? hover.layerSamples : [],
        zoneRgb: hover?.zoneRgb ?? zoneRgbFromLayerSamples(hover?.layerSamples),
      },
      bookmarks,
      { stateBundle: latestStateBundle },
    );
    if (!bookmark) {
      showSiteToast("error", "Could not read world coordinates for that click.");
      return;
    }
    setBookmarkPlacementActive(false);
    persistBookmarksAndRender(bookmarks.concat(bookmark), `Saved ${bookmark.label}.`, {
      toastTone: "success",
    });
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

  function setSearchDropdownOpen(open) {
    const nextOpen = open === true;
    if (searchUiState.open === nextOpen) {
      return;
    }
    searchUiState.open = nextOpen;
    patchMapUiSignalState({ search: searchUiState });
    renderCurrentState(getLatestStateBundle());
  }

  function applySearchMatchAndClose(stateBundle, match) {
    applySearchMatchSelection(
      shell,
      elements,
      renderCurrentState,
      patchMapInputSignalState,
      stateBundle,
      match,
    );
    searchUiState.open = false;
    patchMapUiSignalState({ search: searchUiState });
    renderCurrentState(getLatestStateBundle());
  }

  function pushPatchRangePatch() {
    const current = getLatestStateBundle();
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
    patchMapInputSignalState({
      version: 1,
      filters: {
        patchId:
          patchRange.fromPatchId &&
          patchRange.toPatchId &&
          patchRange.fromPatchId === patchRange.toPatchId
            ? patchRange.fromPatchId
            : null,
        fromPatchId: patchRange.fromPatchId,
        toPatchId: patchRange.toPatchId,
      },
    });
  }

  elements.search.addEventListener("input", () => {
    if (isRendering) {
      return;
    }
    searchUiState.open = true;
    patchMapUiSignalState({ search: searchUiState });
  });

  elements.search.addEventListener("focus", () => {
    setSearchDropdownOpen(true);
  });

  elements.search.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      event.preventDefault();
      setSearchDropdownOpen(false);
      elements.search.blur();
      return;
    }
    if (event.key !== "Enter") {
      return;
    }
    const current = getLatestStateBundle();
    const matches = String(elements.search.value || "").trim()
      ? buildSearchMatches(current, elements.search.value, zoneCatalog)
      : buildDefaultFishFilterMatches(current);
    const top = matches[0];
    if (!top) {
      return;
    }
    event.preventDefault();
    applySearchMatchAndClose(current, top);
  });

  elements.searchResults.addEventListener("click", (event) => {
    if (
      event.target.closest("button[data-fish-focus-id]") ||
      event.target.closest("button[data-semantic-focus-code]") ||
      event.target.closest("button[data-zone-focus-rgb]")
    ) {
      return;
    }
    const button = event.target.closest(
      "[data-fish-filter-term], [data-fish-id], [data-zone-rgb], [data-semantic-layer-id][data-semantic-field-id]",
    );
    if (!button) {
      return;
    }
    const current = getLatestStateBundle();
    const fishFilterTerm = normalizeFishFilterTerm(button.getAttribute("data-fish-filter-term"));
    if (fishFilterTerm) {
      applySearchMatchAndClose(current, {
        kind: "fish-filter",
        term: fishFilterTerm,
      });
      return;
    }
    const zoneRgb = Number.parseInt(button.getAttribute("data-zone-rgb"), 10);
    if (Number.isFinite(zoneRgb)) {
      applySearchMatchAndClose(current, {
        kind: "zone",
        zoneRgb,
      });
      return;
    }
    const semanticLayerId = String(button.getAttribute("data-semantic-layer-id") || "").trim();
    const semanticFieldId = Number.parseInt(button.getAttribute("data-semantic-field-id"), 10);
    if (semanticLayerId && Number.isFinite(semanticFieldId)) {
      applySearchMatchAndClose(current, {
        kind: "semantic",
        layerId: semanticLayerId,
        fieldId: semanticFieldId,
        label: button.getAttribute("data-semantic-label") || "",
      });
      return;
    }
    const fishId = Number.parseInt(button.getAttribute("data-fish-id"), 10);
    if (!Number.isFinite(fishId)) {
      return;
    }
    applySearchMatchAndClose(current, {
      kind: "fish",
      fishId,
    });
  });

  elements.searchResults.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" && event.key !== " ") {
      return;
    }
    if (
      event.target.closest("button[data-fish-focus-id]") ||
      event.target.closest("button[data-semantic-focus-code]") ||
      event.target.closest("button[data-zone-focus-rgb]")
    ) {
      return;
    }
    const row = event.target.closest(
      "[data-fish-filter-term], [data-fish-id], [data-zone-rgb], [data-semantic-layer-id][data-semantic-field-id]",
    );
    if (!row) {
      return;
    }
    const current = getLatestStateBundle();
    const fishFilterTerm = normalizeFishFilterTerm(row.getAttribute("data-fish-filter-term"));
    if (fishFilterTerm) {
      event.preventDefault();
      applySearchMatchAndClose(current, {
        kind: "fish-filter",
        term: fishFilterTerm,
      });
      return;
    }
    const fishId = Number.parseInt(row.getAttribute("data-fish-id"), 10);
    if (Number.isFinite(fishId)) {
      event.preventDefault();
      applySearchMatchAndClose(current, {
        kind: "fish",
        fishId,
      });
      return;
    }
    const zoneRgb = Number.parseInt(row.getAttribute("data-zone-rgb"), 10);
    if (Number.isFinite(zoneRgb)) {
      event.preventDefault();
      applySearchMatchAndClose(current, {
        kind: "zone",
        zoneRgb,
      });
      return;
    }
    const semanticLayerId = String(row.getAttribute("data-semantic-layer-id") || "").trim();
    const semanticFieldId = Number.parseInt(row.getAttribute("data-semantic-field-id"), 10);
    if (!semanticLayerId || !Number.isFinite(semanticFieldId)) {
      return;
    }
    event.preventDefault();
    applySearchMatchAndClose(current, {
      kind: "semantic",
      layerId: semanticLayerId,
      fieldId: semanticFieldId,
      label: row.getAttribute("data-semantic-label") || "",
    });
  });

  elements.searchWindow?.addEventListener("focusout", () => {
    globalThis.setTimeout?.(() => {
      const activeElement = globalThis.document?.activeElement;
      if (activeElement instanceof Element && elements.searchWindow?.contains(activeElement)) {
        return;
      }
      setSearchDropdownOpen(false);
    }, 0);
  });

  elements.searchSelection.addEventListener("click", (event) => {
    const removeButton = event.target.closest(
      "button.fishymap-selection-remove[data-fish-filter-term], button.fishymap-selection-remove[data-fish-id], button.fishymap-selection-remove[data-zone-rgb], button.fishymap-selection-remove[data-semantic-layer-id][data-semantic-field-id]",
    );
    if (!removeButton) {
      return;
    }
    const current = getLatestStateBundle();
    const fishFilterTerm = normalizeFishFilterTerm(removeButton.getAttribute("data-fish-filter-term"));
    if (fishFilterTerm) {
      dispatchStatePatchAndRender({
        version: 1,
        filters: {
          fishFilterTerms: removeSelectedFishFilterTerm(
            resolveSelectedFishFilterTerms(current),
            fishFilterTerm,
          ),
        },
      });
      return;
    }
    const zoneRgb = Number.parseInt(removeButton.getAttribute("data-zone-rgb"), 10);
    if (Number.isFinite(zoneRgb)) {
      dispatchStatePatchAndRender({
        version: 1,
        filters: {
          semanticFieldIdsByLayer: updateSelectedSemanticFieldIdsByLayer(
            resolveSelectedSemanticFieldIdsByLayer(current),
            "zone_mask",
            removeSelectedZoneRgb(resolveSelectedZoneRgbs(current), zoneRgb),
          ),
        },
      });
      return;
    }
    const semanticLayerId = String(removeButton.getAttribute("data-semantic-layer-id") || "").trim();
    const semanticFieldId = Number.parseInt(
      removeButton.getAttribute("data-semantic-field-id"),
      10,
    );
    if (semanticLayerId && Number.isFinite(semanticFieldId)) {
      const selectedByLayer = resolveSelectedSemanticFieldIdsByLayer(current);
      dispatchStatePatchAndRender({
        version: 1,
        filters: {
          semanticFieldIdsByLayer: updateSelectedSemanticFieldIdsByLayer(
            selectedByLayer,
            semanticLayerId,
            removeSelectedSemanticFieldId(
              selectedByLayer[semanticLayerId] || [],
              semanticFieldId,
            ),
          ),
        },
      });
      return;
    }
    const fishId = Number.parseInt(removeButton.getAttribute("data-fish-id"), 10);
    dispatchStatePatchAndRender({
      version: 1,
      filters: {
        fishIds: removeSelectedFishId(resolveSelectedFishIds(current), fishId),
      },
    });
  });

  elements.zoneInfoBody?.addEventListener("click", (event) => {
    {
      if (event.target.closest("button[data-fish-focus-id]")) {
        return;
      }
      const row = event.target.closest("[data-zone-evidence-fish-id]");
      if (row) {
        const fishId = Number.parseInt(row.getAttribute("data-zone-evidence-fish-id"), 10);
        if (!Number.isFinite(fishId)) {
          return;
        }
        const current = getLatestStateBundle();
        dispatchStatePatchAndRender({
          version: 1,
          filters: {
            fishIds: moveFishIdToCurrent(resolveSelectedFishIds(current), fishId),
          },
        });
        return;
      }
    }
    {
      const button = event.target.closest("button[data-zone-info-target-world-x]");
      if (button) {
        const worldX = normalizeBookmarkCoordinate(
          button.getAttribute("data-zone-info-target-world-x"),
        );
        const worldZ = normalizeBookmarkCoordinate(
          button.getAttribute("data-zone-info-target-world-z"),
        );
        if (worldX == null || worldZ == null) {
          return;
        }
        const pointLabel = button.getAttribute("data-zone-info-target-label") || "";
        const command = buildFocusCommandForWorldPoint(
          worldX,
          worldZ,
          getLatestStateBundle(),
          currentViewportSize(),
          {
          pointKind: "waypoint",
          pointLabel,
            autoAdjustView: autoAdjustViewEnabled(),
          },
        );
        if (command) {
          dispatchMapCommand(shell, command);
        }
        return;
      }
    }
  });

  elements.zoneInfoBody?.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" && event.key !== " ") {
      return;
    }
    if (event.target.closest("button[data-fish-focus-id]")) {
      return;
    }
    const row = event.target.closest("[data-zone-evidence-fish-id]");
    if (!row) {
      return;
    }
    const fishId = Number.parseInt(row.getAttribute("data-zone-evidence-fish-id"), 10);
    if (!Number.isFinite(fishId)) {
      return;
    }
    event.preventDefault();
    const current = getLatestStateBundle();
    dispatchStatePatchAndRender({
      version: 1,
      filters: {
        fishIds: moveFishIdToCurrent(resolveSelectedFishIds(current), fishId),
      },
    });
  });

  shell.addEventListener("pointerdown", (event) => {
    const button = event.target.closest(
      "button[data-fish-focus-id], button[data-semantic-focus-code], button[data-zone-focus-rgb]",
    );
    if (!button) {
      return;
    }
    button.dataset.semanticFocusPointerHandled = "true";
    event.preventDefault();
    event.stopPropagation();
    const fishId = String(button.getAttribute("data-fish-focus-id") || "").trim();
    if (fishId) {
      void dispatchFishFocusFromId(fishId);
      return;
    }
    const semanticCode = String(button.getAttribute("data-semantic-focus-code") || "").trim();
    if (semanticCode) {
      void dispatchSemanticFocusFromCode(semanticCode);
      return;
    }
    void dispatchZoneFocusFromRgb(String(button.getAttribute("data-zone-focus-rgb") || "").trim());
  });

  shell.addEventListener("click", async (event) => {
    const button = event.target.closest(
      "button[data-fish-focus-id], button[data-semantic-focus-code], button[data-zone-focus-rgb]",
    );
    if (!button) {
      return;
    }
    if (button.dataset.semanticFocusPointerHandled === "true") {
      delete button.dataset.semanticFocusPointerHandled;
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    const fishId = String(button.getAttribute("data-fish-focus-id") || "").trim();
    if (fishId) {
      await dispatchFishFocusFromId(fishId);
      return;
    }
    const semanticCode = String(button.getAttribute("data-semantic-focus-code") || "").trim();
    if (semanticCode) {
      await dispatchSemanticFocusFromCode(semanticCode);
      return;
    }
    await dispatchZoneFocusFromRgb(String(button.getAttribute("data-zone-focus-rgb") || "").trim());
  });

  elements.zoneInfoTabs?.addEventListener("click", (event) => {
    const button = event.target.closest("button[data-zone-info-tab]");
    if (!button) {
      return;
    }
    setZoneInfoTab(button.getAttribute("data-zone-info-tab"));
  });

  elements.zoneInfoTabs?.addEventListener("keydown", (event) => {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) {
      return;
    }
    const buttons = Array.from(
      elements.zoneInfoTabs?.querySelectorAll?.("button[data-zone-info-tab]") || [],
    ).filter((button) => button && button.disabled !== true);
    if (!buttons.length) {
      return;
    }
    const currentButton = event.target.closest("button[data-zone-info-tab]");
    const currentIndex = Math.max(0, buttons.indexOf(currentButton));
    let nextButton = buttons[currentIndex] || buttons[0];
    if (event.key === "Home") {
      nextButton = buttons[0];
    } else if (event.key === "End") {
      nextButton = buttons[buttons.length - 1];
    } else if (event.key === "ArrowLeft") {
      nextButton = buttons[(currentIndex - 1 + buttons.length) % buttons.length];
    } else if (event.key === "ArrowRight") {
      nextButton = buttons[(currentIndex + 1) % buttons.length];
    }
    event.preventDefault();
    if (setZoneInfoTab(nextButton?.getAttribute("data-zone-info-tab"))) {
      nextButton?.focus?.();
    }
  });

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
      const current = getLatestStateBundle().state;
      const nextViewMode = current?.view?.viewMode === "3d" ? "2d" : "3d";
      dispatchMapCommand(shell, {
        setViewMode: nextViewMode,
      });
    });
  }

  elements.bookmarkPlace?.addEventListener("click", () => {
    const state = getLatestStateBundle().state;
    if (state.ready !== true) {
      showSiteToast("warning", "Wait for the map to finish loading before placing a bookmark.");
      return;
    }
    if (state.view?.viewMode === "3d") {
      showSiteToast("warning", "Switch back to 2D view to place a bookmark.");
      return;
    }
    bringManagedWindowToFront("bookmarks");
    setBookmarkPlacementActive(true);
  });

  elements.bookmarkCancel?.addEventListener("click", () => {
    setBookmarkPlacementActive(false);
  });

  elements.bookmarkCopySelected?.addEventListener("click", async () => {
    const selectedBookmarks = selectedBookmarksForCopy();
    if (!selectedBookmarks.length) {
      showSiteToast("warning", "Select one or more bookmarks to copy.");
      return;
    }
    try {
      await copyTextToClipboard(formatBookmarkClipboardText(selectedBookmarks));
      const message = `Copied XML for ${selectedBookmarks.length} ${pluralizeBookmarks(selectedBookmarks.length)}.`;
      showSiteToast("success", message);
    } catch (_) {
      const message = "Clipboard access is unavailable in this browser.";
      showSiteToast("error", message);
    }
  });

  elements.bookmarkSelectAll?.addEventListener("click", () => {
    setSelectedBookmarkIds(bookmarks.map((bookmark) => bookmark.id));
    renderBookmarkManager(elements, latestStateBundle, bookmarks, bookmarkUi);
  });

  elements.bookmarkDeleteSelected?.addEventListener("click", () => {
    const selectedBookmarks = selectedBookmarksForCopy();
    if (!selectedBookmarks.length) {
      showSiteToast("warning", "Select one or more bookmarks to delete.");
      return;
    }
    if (typeof globalThis.window?.confirm !== "function") {
      showSiteToast("error", "Bookmark deletion confirmation is unavailable in this browser.");
      return;
    }
    if (
      !globalThis.window.confirm(
        buildBookmarkDeletionPrompt(selectedBookmarks, { selection: true }),
      )
    ) {
      return;
    }
    const selectedIdSet = new Set(selectedBookmarks.map((bookmark) => bookmark.id));
    const nextBookmarks = bookmarks.filter((bookmark) => !selectedIdSet.has(bookmark.id));
    persistBookmarksAndRender(
      nextBookmarks,
      `Removed ${selectedBookmarks.length} selected ${pluralizeBookmarks(selectedBookmarks.length)}.`,
      {
        selectedIds: [],
      },
    );
  });

  elements.bookmarkClearSelection?.addEventListener("click", () => {
    setSelectedBookmarkIds([]);
    renderBookmarkManager(elements, latestStateBundle, bookmarks, bookmarkUi);
  });

  elements.bookmarkExport?.addEventListener("click", () => {
    const exportBookmarks = bookmarksForExport();
    const selectedCount = selectedBookmarksForCopy().length;
    if (!exportBookmarks.length) {
      showSiteToast("warning", "There are no bookmarks to export yet.");
      return;
    }
    try {
      downloadBookmarkExport(exportBookmarks, { stateBundle: latestStateBundle });
      const message =
        selectedCount
          ? `Exported ${exportBookmarks.length} selected ${pluralizeBookmarks(exportBookmarks.length)}.`
          : `Exported ${exportBookmarks.length} ${pluralizeBookmarks(exportBookmarks.length)}.`;
      showSiteToast("info", message);
    } catch (_) {
      const message = "Bookmark export is unavailable in this browser.";
      showSiteToast("error", message);
    }
  });

  elements.bookmarkImportTrigger?.addEventListener("click", () => {
    if (!elements.bookmarkImportInput) {
      showSiteToast("error", "Bookmark import is unavailable in this browser.");
      return;
    }
    if (bookmarkUi.placing) {
      setBookmarkPlacementActive(false);
    }
    elements.bookmarkImportInput.value = "";
    elements.bookmarkImportInput.click();
  });

  elements.bookmarkImportInput?.addEventListener("change", async () => {
    const file = elements.bookmarkImportInput?.files?.[0];
    if (!file) {
      return;
    }
    try {
      const importedBookmarks = parseImportedBookmarks(await readBookmarkImportFile(file));
      if (!importedBookmarks.length) {
        const message = "The selected file did not contain any bookmark XML.";
        showSiteToast("warning", message);
        return;
      }
      const existingBookmarkKeys = new Set(
        normalizeBookmarks(bookmarks).map((bookmark) => bookmarkMergeKey(bookmark)).filter(Boolean),
      );
      const importedBookmarkIds = importedBookmarks
        .filter((bookmark) => !existingBookmarkKeys.has(bookmarkMergeKey(bookmark)))
        .map((bookmark) => bookmark.id);
      const nextBookmarks = mergeImportedBookmarks(bookmarks, importedBookmarks);
      const importedCount = importedBookmarkIds.length;
      const skippedCount = importedBookmarks.length - importedCount;
      const message = importedCount
        ? `Imported ${importedCount} ${pluralizeBookmarks(importedCount)}${
            skippedCount ? `; skipped ${skippedCount} duplicate${skippedCount === 1 ? "" : "s"}.` : "."
          }`
        : "No new bookmarks were imported.";
      persistBookmarksAndRender(
        nextBookmarks,
        message,
        {
          toastTone: importedCount ? "success" : "info",
          selectedIds: importedCount
            ? normalizeSelectedBookmarkIds(nextBookmarks, bookmarkUi.selectedIds.concat(importedBookmarkIds))
            : bookmarkUi.selectedIds,
        },
      );
    } catch (error) {
      console.warn("Failed to import map bookmarks", error);
      const message = "Bookmark import failed. Choose a valid WorldmapBookMark XML file.";
      showSiteToast("error", message);
    } finally {
      elements.bookmarkImportInput.value = "";
    }
  });

  elements.bookmarksList?.addEventListener("change", (event) => {
    const selectionInput = event.target.closest("input[data-bookmark-select]");
    if (!selectionInput) {
      return;
    }
    const bookmarkId = selectionInput.getAttribute("data-bookmark-select");
    const nextSelectedIds = selectionInput.checked
      ? bookmarkUi.selectedIds.concat(bookmarkId)
      : bookmarkUi.selectedIds.filter((selectedId) => selectedId !== bookmarkId);
    setSelectedBookmarkIds(nextSelectedIds);
    renderBookmarkManager(elements, latestStateBundle, bookmarks, bookmarkUi);
  });

  elements.bookmarksList?.addEventListener("dragstart", (event) => {
    const handle = event.target.closest("button[data-bookmark-drag][draggable='true']");
    const card = handle?.closest(".fishymap-bookmark-card");
    if (!card || !handle || isRendering) {
      return;
    }
    const bookmarkId = card.getAttribute("data-bookmark-id");
    if (!bookmarkId) {
      return;
    }
    bookmarkDragState.draggingBookmarkId = bookmarkId;
    card.dataset.dragging = "true";
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", bookmarkId);
    }
  });

  elements.bookmarksList?.addEventListener("dragover", (event) => {
    if (!bookmarkDragState.draggingBookmarkId) {
      return;
    }
    event.preventDefault();
    updateDragAutoScroll(elements.bookmarksList, event.clientY);
    const card = event.target.closest(".fishymap-bookmark-card");
    if (!card) {
      clearBookmarkDropState();
      return;
    }
    const targetBookmarkId = card.getAttribute("data-bookmark-id");
    if (!targetBookmarkId || targetBookmarkId === bookmarkDragState.draggingBookmarkId) {
      clearBookmarkDropState();
      return;
    }
    const rect = card.getBoundingClientRect();
    const offsetY = event.clientY - rect.top;
    applyBookmarkDropState(targetBookmarkId, offsetY >= rect.height / 2 ? "after" : "before");
  });

  elements.bookmarksList?.addEventListener("drop", (event) => {
    if (
      !bookmarkDragState.draggingBookmarkId ||
      !bookmarkDragState.overBookmarkId ||
      !bookmarkDragState.mode
    ) {
      stopDragAutoScroll();
      clearBookmarkDropState();
      return;
    }
    event.preventDefault();
    const nextBookmarks = moveBookmarkBefore(
      bookmarks,
      bookmarkDragState.draggingBookmarkId,
      bookmarkDragState.overBookmarkId,
      bookmarkDragState.mode,
    );
    const movedBookmark = nextBookmarks.find(
      (bookmark) => bookmark.id === bookmarkDragState.draggingBookmarkId,
    );
    stopDragAutoScroll();
    clearBookmarkDropState();
    bookmarkDragState.draggingBookmarkId = null;
    persistBookmarksAndRender(
      nextBookmarks,
      movedBookmark ? `Moved ${movedBookmark.label}.` : "Reordered bookmarks.",
      { toastTone: "info" },
    );
  });

  elements.bookmarksList?.addEventListener("dragend", () => {
    stopDragAutoScroll();
    elements.bookmarksList
      ?.querySelectorAll?.(".fishymap-bookmark-card[data-dragging]")
      ?.forEach?.((card) => {
        delete card.dataset.dragging;
      });
    bookmarkDragState.draggingBookmarkId = null;
    clearBookmarkDropState();
  });

  elements.bookmarksList?.addEventListener("click", async (event) => {
    const activateButton = event.target.closest("button[data-bookmark-activate]");
    if (activateButton) {
      const bookmark = bookmarks.find(
        (entry) => entry.id === activateButton.getAttribute("data-bookmark-activate"),
      );
      activateBookmarkSelection(bookmark);
      return;
    }

    const renameButton = event.target.closest("button[data-bookmark-rename]");
    if (renameButton) {
      const bookmark = bookmarks.find(
        (entry) => entry.id === renameButton.getAttribute("data-bookmark-rename"),
      );
      if (!bookmark) {
        return;
      }
      if (typeof globalThis.window?.prompt !== "function") {
        showSiteToast("error", "Bookmark renaming is unavailable in this browser.");
        return;
      }
      const requestedLabel = globalThis.window.prompt("Bookmark name", bookmark.label);
      if (requestedLabel == null) {
        return;
      }
      const nextBookmarks = renameBookmark(bookmarks, bookmark.id, requestedLabel, {
        stateBundle: latestStateBundle,
      });
      const renamedBookmark =
        nextBookmarks.find((entry) => entry.id === bookmark.id) || bookmark;
      persistBookmarksAndRender(nextBookmarks, `Renamed bookmark to ${renamedBookmark.label}.`, {
        toastTone: "success",
      });
      return;
    }

    const copyButton = event.target.closest("button[data-bookmark-copy]");
    if (copyButton) {
      const bookmark = bookmarks.find(
        (entry) => entry.id === copyButton.getAttribute("data-bookmark-copy"),
      );
      if (!bookmark) {
        return;
      }
      try {
        await copyTextToClipboard(formatBookmarkClipboardText([bookmark]));
        const message = `Copied XML for ${bookmark.label}.`;
        showSiteToast("success", message);
      } catch (_) {
        const message = "Clipboard access is unavailable in this browser.";
        showSiteToast("error", message);
      }
      return;
    }
    const deleteButton = event.target.closest("button[data-bookmark-delete]");
    if (!deleteButton) {
      return;
    }
    const bookmark = bookmarks.find(
      (entry) => entry.id === deleteButton.getAttribute("data-bookmark-delete"),
    );
    if (!bookmark) {
      return;
    }
    if (typeof globalThis.window?.confirm !== "function") {
      showSiteToast("error", "Bookmark deletion confirmation is unavailable in this browser.");
      return;
    }
    if (!globalThis.window.confirm(buildBookmarkDeletionPrompt([bookmark]))) {
      return;
    }
    persistBookmarksAndRender(
      bookmarks.filter((entry) => entry.id !== bookmark.id),
      `Removed ${bookmark.label}.`,
      { toastTone: "info" },
    );
  });

  if (elements.autoAdjustView) {
    elements.autoAdjustView.addEventListener("change", () => {
      if (isRendering) {
        return;
      }
      if (!updateWindowUiEntry("settings", { autoAdjustView: elements.autoAdjustView.checked })) {
        return;
      }
      persistWindowUiState();
      renderCurrentState(getLatestStateBundle());
    });
  }

  elements.layers.addEventListener("click", (event) => {
    const settingsButton = event.target.closest("button[data-layer-settings-toggle]");
    if (!isRendering && settingsButton) {
      toggleLayerSettings(settingsButton.getAttribute("data-layer-settings-toggle"));
      return;
    }
    const button = event.target.closest("button[data-layer-visibility]");
    if (isRendering || !button) {
      return;
    }
    const layerId = button.getAttribute("data-layer-visibility");
    if (!layerId) {
      return;
    }
    const current = getLatestStateBundle();
    const visibleIds = new Set(resolveVisibleLayerIds(current));
    if (visibleIds.has(layerId)) {
      visibleIds.delete(layerId);
    } else {
      visibleIds.add(layerId);
    }
    dispatchStatePatchAndRender({
      version: 1,
      filters: {
        layerIdsVisible: resolveLayerEntries(current)
          .map((layer) => layer.layerId)
          .filter((candidateId) => visibleIds.has(candidateId)),
      },
    });
  });

  elements.layers.addEventListener("change", (event) => {
    const connectionToggle = event.target.closest("input[data-layer-waypoint-connections]");
    if (!isRendering && connectionToggle) {
      const layerId = connectionToggle.getAttribute("data-layer-waypoint-connections");
      if (!layerId) {
        return;
      }
      const current = getLatestStateBundle();
      dispatchStatePatchAndRender({
        version: 1,
        filters: {
          layerWaypointConnectionsVisible: buildLayerWaypointConnectionsPatch(
            current,
            layerId,
            connectionToggle.checked,
          ),
        },
      });
      return;
    }

    const labelToggle = event.target.closest("input[data-layer-waypoint-labels]");
    if (!isRendering && labelToggle) {
      const layerId = labelToggle.getAttribute("data-layer-waypoint-labels");
      if (!layerId) {
        return;
      }
      const current = getLatestStateBundle();
      dispatchStatePatchAndRender({
        version: 1,
        filters: {
          layerWaypointLabelsVisible: buildLayerWaypointLabelsPatch(
            current,
            layerId,
            labelToggle.checked,
          ),
        },
      });
      return;
    }

    const pointIconsToggle = event.target.closest("input[data-layer-point-icons]");
    if (isRendering || !pointIconsToggle) {
      return;
    }
    const layerId = pointIconsToggle.getAttribute("data-layer-point-icons");
    if (!layerId) {
      return;
    }
    const current = getLatestStateBundle();
    dispatchStatePatchAndRender({
      version: 1,
      filters: {
        layerPointIconsVisible: buildLayerPointIconsPatch(current, layerId, pointIconsToggle.checked),
      },
    });
  });

  elements.layers.addEventListener("input", (event) => {
    const pointIconScaleSlider = event.target.closest("input[data-layer-point-icon-scale]");
    if (!isRendering && pointIconScaleSlider) {
      setActiveLayerPointIconScale(pointIconScaleSlider);
      const layerId = pointIconScaleSlider.getAttribute("data-layer-point-icon-scale");
      if (!layerId) {
        return;
      }
      const patch = {
        version: 1,
        filters: {
          layerPointIconScales: buildLayerPointIconScalePatch(
            getLatestStateBundle(),
            layerId,
            pointIconScaleSlider.value,
          ),
        },
      };
      patchMapInputSignalState(patch);
      return;
    }

    const slider = event.target.closest("input[data-layer-opacity]");
    if (isRendering || !slider) {
      return;
    }
    setActiveLayerOpacity(slider);
    const layerId = slider.getAttribute("data-layer-opacity");
    if (!layerId) {
      return;
    }
    const current = getLatestStateBundle();
    const patch = {
      version: 1,
      filters: {
        layerOpacities: buildLayerOpacityPatch(current, layerId, slider.value),
      },
    };
    patchMapInputSignalState(patch);
  });

  elements.layers.addEventListener("pointerdown", (event) => {
    const pointIconScaleSlider = event.target.closest("input[data-layer-point-icon-scale]");
    if (pointIconScaleSlider) {
      setActiveLayerPointIconScale(pointIconScaleSlider);
      return;
    }
    const slider = event.target.closest("input[data-layer-opacity]");
    if (!slider) {
      return;
    }
    setActiveLayerOpacity(slider);
  });

  elements.layers.addEventListener("focusin", (event) => {
    const pointIconScaleSlider = event.target.closest("input[data-layer-point-icon-scale]");
    if (pointIconScaleSlider) {
      setActiveLayerPointIconScale(pointIconScaleSlider);
      return;
    }
    const slider = event.target.closest("input[data-layer-opacity]");
    if (!slider) {
      return;
    }
    setActiveLayerOpacity(slider);
  });

  elements.layers.addEventListener("change", (event) => {
    const pointIconScaleSlider = event.target.closest("input[data-layer-point-icon-scale]");
    if (pointIconScaleSlider) {
      setActiveLayerPointIconScale(pointIconScaleSlider);
      clearActiveLayerPointIconScale();
      return;
    }
    const slider = event.target.closest("input[data-layer-opacity]");
    if (slider) {
      setActiveLayerOpacity(slider);
      clearActiveLayerOpacity();
      return;
    }
  });

  elements.layers.addEventListener("focusout", (event) => {
    const pointIconScaleSlider = event.target.closest("input[data-layer-point-icon-scale]");
    if (pointIconScaleSlider) {
      queueMicrotask(() => {
        clearActiveLayerPointIconScale();
      });
      return;
    }
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
    event.preventDefault();
    updateDragAutoScroll(elements.layers, event.clientY);
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
      stopDragAutoScroll();
      clearLayerDropState();
      return;
    }
    event.preventDefault();
    const current = getLatestStateBundle();
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
    stopDragAutoScroll();
    clearLayerDropState();
    layerDragState.draggingLayerId = null;
    dispatchStatePatchAndRender({
      version: 1,
      filters: {
        layerIdsOrdered: nextOrder,
        layerClipMasks: nextClipMasks,
      },
    });
  });

  elements.layers.addEventListener("dragend", () => {
    stopDragAutoScroll();
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

  async function resetMapUiToInitialState() {
    const resetButton = elements.resetUi;
    if (!resetButton || resetButton.disabled) {
      return;
    }
    setBookmarkPlacementActive(false);
    const defaultWindowUiState = buildDefaultWindowUiStateSerialized();
    const remountOptions = buildMapUiResetMountOptions(getLatestStateBundle().state);
    const originalLabel = resetButton.textContent;

    setBooleanProperty(resetButton, "disabled", true);
    setTextContent(resetButton, "Resetting...");

    windowUiState = parseWindowUiState(defaultWindowUiState);
    searchUiState.open = false;
    bookmarkUi.placing = false;
    bookmarkUi.selectedIds = [];
    patchMapUiSignalState({
      windowUi: windowUiState,
      search: searchUiState,
      bookmarks: bookmarkUi,
    });
    applyManagedWindows({ persist: true });

    try {
      globalThis.sessionStorage?.removeItem?.(FISHYMAP_STORAGE_KEYS.session);
    } catch (_) {}
    try {
      globalThis.localStorage?.removeItem?.(FISHYMAP_STORAGE_KEYS.prefs);
    } catch (_) {}
    try {
      globalThis.localStorage?.removeItem?.(FISHYMAP_WINDOW_UI_STORAGE_KEY);
    } catch (_) {}

    try {
      FishyMapBridge.destroy?.();
      latestStateBundle = requestBridgeState(shell);
      renderCurrentState(latestStateBundle);
      await FishyMapBridge.mount(shell, {
        canvas: elements.canvas,
        ...remountOptions,
      });
      latestStateBundle = requestBridgeState(shell, { refresh: true });
      syncBookmarksToBridge(bookmarks);
      renderCurrentState(latestStateBundle);
    } catch (error) {
      console.error("Failed to reset map UI", error);
      globalThis.window?.location?.reload?.();
      return;
    } finally {
      setTextContent(resetButton, originalLabel || "Reset UI");
      setBooleanProperty(resetButton, "disabled", false);
    }
  }

  elements.resetUi?.addEventListener("click", () => {
    void resetMapUiToInitialState();
  });

  if (elements.legend) {
    elements.legend.addEventListener("toggle", () => {
      if (isRendering) {
        return;
      }
      dispatchStatePatchAndRender({
        version: 1,
        ui: {
          legendOpen: elements.legend.open,
        },
      });
    });
  }

  if (elements.diagnostics) {
    elements.diagnostics.addEventListener("toggle", () => {
      if (isRendering) {
        return;
      }
      patchMapInputSignalState({
        version: 1,
        ui: {
          diagnosticsOpen: elements.diagnostics.open,
        },
      });
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
    const nextBundle = stateBundleFromEvent(event);
    latestStateBundle = {
      inputState: nextBundle.inputState,
      state: {
        ...(latestStateBundle?.state || {}),
        ...(nextBundle.state || {}),
      },
    };
    publishMapRuntimeSignals(latestStateBundle);
    renderViewState(elements, latestStateBundle.state);
    renderBookmarkManager(elements, latestStateBundle, bookmarks, bookmarkUi);
  });

  shell.addEventListener(FISHYMAP_EVENTS.hoverChanged, (event) => {
    const hover = hoverFromEventDetail(event.detail || {});
    if (latestStateBundle?.state) {
      latestStateBundle.state = {
        ...latestStateBundle.state,
        hover,
      };
    }
    publishMapRuntimeSignals(latestStateBundle);
    renderHoverTooltip(elements, hover, latestStateBundle);
    renderBookmarkManager(elements, latestStateBundle, bookmarks, bookmarkUi);
  });

  document.addEventListener(DATASTAR_SIGNAL_PATCH_EVENT, syncLocalUiStateFromSignals);
  document.addEventListener(DATASTAR_SIGNAL_PATCH_EVENT, syncBridgeInputStateFromSignals);
  window.addEventListener("fishystuff:themechange", () => applyThemeToShell(elements.shell));
  window.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && bookmarkUi.placing) {
      setBookmarkPlacementActive(false);
    }
  });
  window.addEventListener("resize", () => {
    applyManagedWindows({ persist: true });
  });

  patchMapUiSignalState({
    windowUi: windowUiState,
    search: searchUiState,
    bookmarks: bookmarkUi,
  });
  syncBookmarksToBridge(bookmarks);
  renderCurrentState();
  return {
    setZoneCatalog(nextZoneCatalog) {
      zoneCatalog = normalizeZoneCatalog(nextZoneCatalog);
      renderCurrentState(getLatestStateBundle());
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
    searchWindowToggle: document.querySelector("[data-window-toggle='search']"),
    bookmarksWindowToggle: document.querySelector("[data-window-toggle='bookmarks']"),
    settingsWindowToggle: document.querySelector("[data-window-toggle='settings']"),
    zoneInfoWindowToggle: document.querySelector("[data-window-toggle='zone-info']"),
    layersWindowToggle: document.querySelector("[data-window-toggle='layers']"),
    searchWindow: document.getElementById("fishymap-search-window"),
    searchTitlebar: document.getElementById("fishymap-search-titlebar"),
    searchBody: document.getElementById("fishymap-search-body"),
    bookmarksWindow: document.getElementById("fishymap-bookmarks-window"),
    bookmarksTitlebar: document.getElementById("fishymap-bookmarks-titlebar"),
    bookmarksBody: document.getElementById("fishymap-bookmarks-body"),
    bookmarksControls: document.getElementById("fishymap-bookmarks-controls"),
    bookmarkPlace: document.getElementById("fishymap-bookmark-place"),
    bookmarkPlaceLabel: document.getElementById("fishymap-bookmark-place-label"),
    bookmarkCopySelected: document.getElementById("fishymap-bookmark-copy-selected"),
    bookmarkExport: document.getElementById("fishymap-bookmark-export"),
    bookmarkImportTrigger: document.getElementById("fishymap-bookmark-import-trigger"),
    bookmarkImportInput: document.getElementById("fishymap-bookmark-import-input"),
    bookmarkSelectAll: document.getElementById("fishymap-bookmark-select-all"),
    bookmarkDeleteSelected: document.getElementById("fishymap-bookmark-delete-selected"),
    bookmarkClearSelection: document.getElementById("fishymap-bookmark-clear-selection"),
    bookmarkClearSelectionLabel: document.getElementById("fishymap-bookmark-clear-selection-label"),
    bookmarkCancel: document.getElementById("fishymap-bookmark-cancel"),
    bookmarksList: document.getElementById("fishymap-bookmarks-list"),
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
    patchFromPicker: document.getElementById("fishymap-patch-from-picker"),
    patchFrom: document.getElementById("fishymap-patch-from"),
    patchToPicker: document.getElementById("fishymap-patch-to-picker"),
    patchTo: document.getElementById("fishymap-patch-to"),
    viewToggle: document.getElementById("fishymap-view-toggle"),
    viewToggleIcon: document.getElementById("fishymap-view-toggle-icon"),
    autoAdjustView: document.getElementById("fishymap-auto-adjust-view"),
    layers: document.getElementById("fishymap-layers"),
    layersWindow: document.getElementById("fishymap-layers-window"),
    layersTitlebar: document.getElementById("fishymap-layers-titlebar"),
    layersBody: document.getElementById("fishymap-layers-body"),
    layersCount: document.getElementById("fishymap-layers-count"),
    resetView: document.getElementById("fishymap-reset-view"),
    resetUi: document.getElementById("fishymap-reset-ui"),
    legend: document.getElementById("fishymap-legend"),
    diagnostics: document.getElementById("fishymap-diagnostics"),
    statusLines: document.getElementById("fishymap-status-lines"),
    diagnosticJson: document.getElementById("fishymap-diagnostic-json"),
    zoneInfoWindow: document.getElementById("fishymap-zone-info-window"),
    zoneInfoTitlebar: document.getElementById("fishymap-zone-info-titlebar"),
    zoneInfoBody: document.getElementById("fishymap-zone-info-body"),
    zoneInfoTitle: document.getElementById("fishymap-zone-info-title"),
    zoneInfoTitleIcon: document.getElementById("fishymap-zone-info-title-icon"),
    zoneInfoTabs: document.getElementById("fishymap-zone-info-tabs"),
    zoneInfoPanel: document.getElementById("fishymap-zone-info-panel"),
    zoneInfoStatus: document.getElementById("fishymap-zone-info-status"),
    zoneInfoStatusIcon: document.getElementById("fishymap-zone-info-status-icon"),
    zoneInfoStatusText: document.getElementById("fishymap-zone-info-status-text"),
    hoverTooltip: document.getElementById("fishymap-hover-tooltip"),
    hoverLayers: document.getElementById("fishymap-hover-layers"),
    viewReadout: document.getElementById("fishymap-view-readout"),
    errorOverlay: document.getElementById("fishymap-error-overlay"),
    errorMessage: document.getElementById("fishymap-error-message"),
    canvas,
  };

  ensureZoneInfoElements(elements);

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
