const THEME_EVENT = "fishystuff:themechange";
const THEME_PROBE_ID = "fishystuff-theme-probe";
const DEFAULT_PATCH_DEBOUNCE_MS = 48;
const DEFAULT_SESSION_SAVE_MS = 180;
const DEFAULT_BOOTSTRAP_SYNC_MS = 96;
const MIN_BOOTSTRAP_SYNC_PASSES = 4;
const MAX_BOOTSTRAP_SYNC_PASSES = 120;

export const FISHYMAP_CONTRACT_VERSION = 1;
export const FISHYMAP_POINT_ICON_SCALE_MIN = 1;
export const FISHYMAP_POINT_ICON_SCALE_MAX = 3;

export const FISHYMAP_EVENTS = Object.freeze({
  setState: "fishymap:set-state",
  command: "fishymap:command",
  requestState: "fishymap:request-state",
  ready: "fishymap:ready",
  stateChanged: "fishymap:state-changed",
  viewChanged: "fishymap:view-changed",
  selectionChanged: "fishymap:selection-changed",
  hoverChanged: "fishymap:hover-changed",
  diagnostic: "fishymap:diagnostic",
});

export const FISHYMAP_STORAGE_KEYS = Object.freeze({
  session: "fishystuff.map.session.v1",
  prefs: "fishystuff.map.prefs.v1",
  bookmarks: "fishystuff.map.bookmarks.v1",
  caught: "fishystuff.pokedex.caught.v1",
});

/**
 * @typedef {{
 *   base100?: string,
 *   base200?: string,
 *   base300?: string,
 *   baseContent?: string,
 *   primary?: string,
 *   primaryContent?: string,
 *   secondary?: string,
 *   secondaryContent?: string,
 *   accent?: string,
 *   accentContent?: string,
 *   neutral?: string,
 *   neutralContent?: string,
 *   info?: string,
 *   success?: string,
 *   warning?: string,
 *   error?: string
 * }} FishyMapThemeColors
 */

/**
 * @typedef {{
 *   version?: number,
 *   theme?: { name?: string, colors?: FishyMapThemeColors },
 *   filters?: {
 *     fishIds?: number[],
 *     zoneRgbs?: number[],
 *     searchText?: string,
 *     patchId?: string | null,
 *     fromPatchId?: string | null,
 *     toPatchId?: string | null,
 *     layerIdsVisible?: string[],
 *     layerIdsOrdered?: string[],
 *     layerOpacities?: Record<string, number>,
 *     layerClipMasks?: Record<string, string>
 *   },
 *   ui?: {
 *     diagnosticsOpen?: boolean,
 *     legendOpen?: boolean,
 *     leftPanelOpen?: boolean,
 *     showPoints?: boolean,
 *     showPointIcons?: boolean,
 *     pointIconScale?: number,
 *     bookmarkSelectedIds?: string[],
 *     bookmarks?: Array<{
 *       id?: string,
 *       label?: string | null,
 *       worldX?: number,
 *       worldZ?: number,
 *       rows?: Array<{
 *         key?: string,
 *         icon?: string,
 *         label?: string,
 *         value?: string,
 *         hideLabel?: boolean,
 *         statusIcon?: string | null,
 *         statusIconTone?: string | null
 *       }>,
 *       zoneRgb?: number | null,
 *       createdAt?: string | null
 *     }>
 *   },
 *   commands?: {
 *     resetView?: boolean,
 *     setViewMode?: "2d" | "3d",
 *     selectZoneRgb?: number,
 *     selectSemanticField?: {
 *       layerId?: string,
 *       fieldId?: number,
 *       targetKey?: string | null
 *     },
 *     selectWorldPoint?: {
 *       worldX?: number,
 *       worldZ?: number
 *     },
 *     restoreView?: {
 *       viewMode: "2d" | "3d",
 *       camera?: {
 *         centerWorldX?: number,
 *         centerWorldZ?: number,
 *         zoom?: number,
 *         pivotWorldX?: number,
 *         pivotWorldY?: number,
 *         pivotWorldZ?: number,
 *         yaw?: number,
 *         pitch?: number,
 *         distance?: number
 *       }
 *     }
 *   }
 * }} FishyMapStatePatch
 */

export function createEmptyInputState() {
  return {
    version: FISHYMAP_CONTRACT_VERSION,
    theme: {
      name: undefined,
      colors: {},
    },
    filters: {
      fishIds: [],
      zoneRgbs: [],
      searchText: "",
      patchId: null,
      fromPatchId: null,
      toPatchId: null,
      layerIdsVisible: undefined,
      layerIdsOrdered: undefined,
      layerOpacities: undefined,
      layerClipMasks: undefined,
    },
    ui: {
      diagnosticsOpen: false,
      legendOpen: false,
      leftPanelOpen: true,
      showPoints: true,
      showPointIcons: true,
      pointIconScale: FISHYMAP_POINT_ICON_SCALE_MIN,
      bookmarkSelectedIds: [],
      bookmarks: [],
    },
  };
}

export function createEmptySnapshot() {
  return {
    version: FISHYMAP_CONTRACT_VERSION,
    ready: false,
    theme: {
      name: undefined,
      colors: {},
    },
    filters: {
      fishIds: [],
      zoneRgbs: [],
      searchText: "",
      patchId: null,
      fromPatchId: null,
      toPatchId: null,
      layerIdsVisible: undefined,
      layerIdsOrdered: undefined,
      layerOpacities: undefined,
      layerClipMasks: undefined,
    },
    ui: {
      diagnosticsOpen: false,
      legendOpen: false,
      leftPanelOpen: true,
      showPoints: true,
      showPointIcons: true,
      pointIconScale: FISHYMAP_POINT_ICON_SCALE_MIN,
      bookmarkSelectedIds: [],
      bookmarks: [],
    },
    view: {
      viewMode: "2d",
      camera: {},
    },
    selection: {},
    hover: {},
    catalog: {
      capabilities: [],
      layers: [],
      patches: [],
      fish: [],
    },
    statuses: {
      metaStatus: "",
      layersStatus: "",
      zonesStatus: "",
      pointsStatus: "",
      fishStatus: "",
      zoneStatsStatus: "",
    },
    lastDiagnostic: null,
  };
}

export function zoneRgbFromLayerSamples(layerSamples) {
  if (!Array.isArray(layerSamples)) {
    return null;
  }
  const zoneSample = layerSamples.find(
    (sample) => String(sample?.layerId || "").trim() === "zone_mask",
  );
  const zoneRgb = Number(zoneSample?.rgbU32);
  return Number.isFinite(zoneRgb) ? zoneRgb : null;
}

function normalizeWorldCoordinate(value) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function normalizeHoverSnapshotValue(value) {
  const layerSamples = Array.isArray(value?.layerSamples) ? cloneJson(value.layerSamples) : [];
  return {
    worldX: normalizeWorldCoordinate(value?.worldX),
    worldZ: normalizeWorldCoordinate(value?.worldZ),
    layerSamples,
  };
}

function normalizeSelectionSnapshotValue(value) {
  const layerSamples = Array.isArray(value?.layerSamples) ? cloneJson(value.layerSamples) : [];
  return {
    ...(isPlainObject(value) ? cloneJson(value) : {}),
    worldX: normalizeWorldCoordinate(value?.worldX),
    worldZ: normalizeWorldCoordinate(value?.worldZ),
    layerSamples,
  };
}

function performanceNowMs() {
  if (typeof globalThis.performance?.now === "function") {
    return globalThis.performance.now();
  }
  return Date.now();
}

function createPerformanceCollector(scenario = "browser") {
  return {
    scenario: String(scenario || "browser"),
    startedAtMs: performanceNowMs(),
    spanSamples: new Map(),
    counters: new Map(),
  };
}

function normalizePerformanceOptions(options = {}) {
  const warmupFrames = Number.parseInt(options.warmupFrames, 10);
  return {
    scenario: String(options.scenario || "browser").trim() || "browser",
    warmupFrames: Number.isFinite(warmupFrames) && warmupFrames >= 0 ? warmupFrames : 0,
    captureTrace: options.captureTrace === true,
  };
}

function addPerformanceSpanSample(collector, name, durationMs) {
  if (!collector || !name) {
    return;
  }
  const samples = collector.spanSamples.get(name) || [];
  samples.push(Math.max(0, Number(durationMs) || 0));
  collector.spanSamples.set(name, samples);
}

function addPerformanceCounter(collector, name, delta = 1) {
  if (!collector || !name) {
    return;
  }
  const nextValue = (collector.counters.get(name) || 0) + Number(delta || 0);
  collector.counters.set(name, nextValue);
}

function summarizeSamples(samples) {
  if (!Array.isArray(samples) || samples.length === 0) {
    return {
      count: 0,
      avg_ms: 0,
      p50_ms: 0,
      p95_ms: 0,
      p99_ms: 0,
      max_ms: 0,
      total_ms: 0,
    };
  }
  const sorted = samples.slice().sort((left, right) => left - right);
  const totalMs = sorted.reduce((sum, value) => sum + value, 0);
  const pick = (p) => sorted[Math.min(sorted.length - 1, Math.round((sorted.length - 1) * p))];
  return {
    count: sorted.length,
    avg_ms: totalMs / sorted.length,
    p50_ms: pick(0.5),
    p95_ms: pick(0.95),
    p99_ms: pick(0.99),
    max_ms: sorted[sorted.length - 1],
    total_ms: totalMs,
  };
}

function snapshotPerformanceCollector(collector) {
  const namedSpans = {};
  for (const [name, samples] of collector?.spanSamples || []) {
    namedSpans[name] = summarizeSamples(samples);
  }
  const counters = {};
  for (const [name, value] of collector?.counters || []) {
    counters[name] = value;
  }
  return {
    scenario: collector?.scenario || "browser",
    elapsed_ms: Math.max(0, performanceNowMs() - (collector?.startedAtMs || performanceNowMs())),
    named_spans: namedSpans,
    counters,
  };
}

function emptyQuantileSummary() {
  return {
    avg: 0,
    p50: 0,
    p95: 0,
    p99: 0,
    max: 0,
  };
}

function emptyProfileSummary(scenario = "browser", warmupFrames = 0) {
  return {
    scenario,
    bevy_version: null,
    git_revision: null,
    build_profile: "browser",
    frames: 0,
    warmup_frames: warmupFrames,
    wall_clock_ms: 0,
    frame_time_ms: emptyQuantileSummary(),
    named_spans: {},
    counters: {},
  };
}

function normalizePerfCounterSuffix(value) {
  return String(value || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
}

function cloneJson(value) {
  if (value == null) {
    return value;
  }
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

function hasOwn(object, key) {
  return !!object && Object.prototype.hasOwnProperty.call(object, key);
}

function normalizeStringList(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  const out = [];
  const seen = new Set();
  for (const value of values) {
    const normalized = String(value ?? "").trim();
    if (!normalized || seen.has(normalized)) {
      continue;
    }
    seen.add(normalized);
    out.push(normalized);
  }
  return out;
}

function readPersistedLayerVisibility(filters) {
  if (!isPlainObject(filters) || !hasOwn(filters, "layerIdsVisible")) {
    return undefined;
  }
  const layerIdsVisible = normalizeStringList(filters.layerIdsVisible);
  if (filters.layerVisibilityExplicit === true || layerIdsVisible.length > 0) {
    return layerIdsVisible;
  }
  return undefined;
}

function readPersistedLayerOrder(filters) {
  if (!isPlainObject(filters) || !hasOwn(filters, "layerIdsOrdered")) {
    return undefined;
  }
  const layerIdsOrdered = normalizeStringList(filters.layerIdsOrdered);
  return layerIdsOrdered.length ? layerIdsOrdered : undefined;
}

function normalizeLayerOpacity(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return undefined;
  }
  return Math.min(1, Math.max(0, number));
}

function normalizeLayerOpacityMap(values) {
  if (!isPlainObject(values)) {
    return {};
  }
  const out = {};
  for (const [key, value] of Object.entries(values)) {
    const layerId = String(key ?? "").trim();
    if (!layerId) {
      continue;
    }
    const opacity = normalizeLayerOpacity(value);
    if (opacity === undefined) {
      continue;
    }
    out[layerId] = opacity;
  }
  return out;
}

function readPersistedLayerOpacities(filters) {
  if (!isPlainObject(filters) || !hasOwn(filters, "layerOpacities")) {
    return undefined;
  }
  const layerOpacities = normalizeLayerOpacityMap(filters.layerOpacities);
  return Object.keys(layerOpacities).length ? layerOpacities : undefined;
}

function normalizeLayerClipMaskMap(values) {
  if (!isPlainObject(values)) {
    return {};
  }
  const out = {};
  for (const [key, value] of Object.entries(values)) {
    const layerId = String(key ?? "").trim();
    const maskLayerId = String(value ?? "").trim();
    if (
      !layerId ||
      !maskLayerId ||
      layerId === maskLayerId ||
      layerId === "minimap" ||
      maskLayerId === "minimap"
    ) {
      continue;
    }
    out[layerId] = maskLayerId;
  }
  const flattened = {};
  for (const layerId of Object.keys(out)) {
    const seen = new Set([layerId]);
    let cursor = out[layerId];
    let topMaskLayerId = "";
    while (cursor) {
      if (seen.has(cursor) || cursor === "minimap") {
        topMaskLayerId = "";
        break;
      }
      seen.add(cursor);
      const nextMaskLayerId = out[cursor];
      if (!nextMaskLayerId || nextMaskLayerId === cursor || nextMaskLayerId === "minimap") {
        topMaskLayerId = cursor;
        break;
      }
      cursor = nextMaskLayerId;
    }
    if (!topMaskLayerId || topMaskLayerId === layerId) {
      continue;
    }
    flattened[layerId] = topMaskLayerId;
  }
  return flattened;
}

function readPersistedLayerClipMasks(filters) {
  if (!isPlainObject(filters) || !hasOwn(filters, "layerClipMasks")) {
    return undefined;
  }
  const layerClipMasks = normalizeLayerClipMaskMap(filters.layerClipMasks);
  return Object.keys(layerClipMasks).length ? layerClipMasks : undefined;
}

function normalizeFishIds(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  const out = [];
  const seen = new Set();
  for (const value of values) {
    const number = Number.parseInt(value, 10);
    if (!Number.isFinite(number) || seen.has(number)) {
      continue;
    }
    seen.add(number);
    out.push(number);
  }
  return out;
}

function normalizeZoneRgbs(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  const out = [];
  const seen = new Set();
  for (const value of values) {
    const number = Number(value);
    if (!Number.isFinite(number) || !Number.isInteger(number) || seen.has(number)) {
      continue;
    }
    seen.add(number);
    out.push(number);
  }
  return out;
}

function normalizeNullableString(value) {
  if (value == null) {
    return null;
  }
  const normalized = String(value).trim();
  return normalized || null;
}

function normalizeCssColor(value, doc = globalThis.document) {
  if (typeof value !== "string") {
    return "";
  }
  const trimmed = value.trim();
  if (!trimmed) {
    return "";
  }

  const probe = doc?.createElement?.("span") || null;
  if (!probe?.style) {
    return trimmed;
  }

  try {
    probe.style.color = "";
    probe.style.color = trimmed;
    const parsed = String(probe.style.color || "").trim();
    if (!parsed) {
      return trimmed;
    }

    const context = doc?.createElement?.("canvas")?.getContext?.("2d") || null;
    if (!context) {
      return parsed;
    }
    context.fillStyle = parsed;
    return String(context.fillStyle || "").trim() || parsed;
  } catch (_) {
    return trimmed;
  }
}

function normalizeThemeColors(colors, doc = globalThis.document) {
  if (!isPlainObject(colors)) {
    return undefined;
  }
  const out = {};
  for (const [key, value] of Object.entries(colors)) {
    if (typeof value !== "string") {
      continue;
    }
    const normalized = normalizeCssColor(value, doc);
    if (!normalized) {
      continue;
    }
    out[key] = normalized;
  }
  return Object.keys(out).length ? out : undefined;
}

function normalizeRestoreView(value) {
  if (!isPlainObject(value)) {
    return undefined;
  }
  const viewMode = value.viewMode === "3d" ? "3d" : "2d";
  const camera = isPlainObject(value.camera) ? { ...value.camera } : {};
  const numericKeys = [
    "centerWorldX",
    "centerWorldZ",
    "zoom",
    "pivotWorldX",
    "pivotWorldY",
    "pivotWorldZ",
    "yaw",
    "pitch",
    "distance",
  ];
  for (const key of numericKeys) {
    if (!hasOwn(camera, key)) {
      continue;
    }
    const number = Number(camera[key]);
    if (!Number.isFinite(number)) {
      delete camera[key];
      continue;
    }
    camera[key] = number;
  }
  return {
    viewMode,
    camera,
  };
}

function normalizePointIconScale(value) {
  if (value == null || value === "") {
    return undefined;
  }
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return undefined;
  }
  return Math.min(FISHYMAP_POINT_ICON_SCALE_MAX, Math.max(FISHYMAP_POINT_ICON_SCALE_MIN, number));
}

function normalizeBookmarkCoordinate(value) {
  if (value == null || value === "") {
    return undefined;
  }
  const number = Number(value);
  return Number.isFinite(number) ? number : undefined;
}

function normalizeBookmarkRowsState(values) {
  const entries = Array.isArray(values) ? values : [];
  const normalized = [];
  for (const row of entries) {
    const icon = String(row?.icon || "").trim();
    const value = String(row?.value || "").trim();
    const label = String(row?.label || "").trim();
    const hideLabel = row?.hideLabel === true;
    if (!icon || !value || (!hideLabel && !label)) {
      continue;
    }
    const key = String(row?.key || "").trim();
    const statusIcon = normalizeNullableString(row?.statusIcon);
    const statusIconTone = normalizeNullableString(row?.statusIconTone);
    normalized.push({
      key,
      icon,
      label,
      value,
      hideLabel,
      ...(statusIcon != null ? { statusIcon } : {}),
      ...(statusIconTone != null ? { statusIconTone } : {}),
    });
  }
  return normalized;
}

function normalizeWorldPointCommand(value) {
  if (!isPlainObject(value)) {
    return undefined;
  }
  const worldX = normalizeBookmarkCoordinate(value.worldX);
  const worldZ = normalizeBookmarkCoordinate(value.worldZ);
  if (worldX === undefined || worldZ === undefined) {
    return undefined;
  }
  return { worldX, worldZ };
}

function normalizeSemanticFieldCommand(value) {
  if (!isPlainObject(value)) {
    return undefined;
  }
  const layerId = String(value.layerId || "").trim();
  const fieldId = Number.parseInt(value.fieldId, 10);
  const targetKey = normalizeNullableString(value.targetKey);
  if (!layerId || !Number.isFinite(fieldId)) {
    return undefined;
  }
  return {
    layerId,
    fieldId,
    ...(targetKey != null ? { targetKey } : {}),
  };
}

function normalizeBookmarksState(values) {
  const entries = Array.isArray(values) ? values : [];
  const normalized = [];
  const seen = new Set();
  for (const entry of entries) {
    const id = String(entry?.id || "").trim();
    const worldX = normalizeBookmarkCoordinate(entry?.worldX);
    const worldZ = normalizeBookmarkCoordinate(entry?.worldZ);
    if (!id || worldX === undefined || worldZ === undefined || seen.has(id)) {
      continue;
    }
    seen.add(id);
    const label = normalizeNullableString(entry?.label);
    const rows = normalizeBookmarkRowsState(entry?.rows);
    const zoneRgb = Number.parseInt(entry?.zoneRgb, 10);
    const createdAt = normalizeNullableString(entry?.createdAt);
    normalized.push({
      id,
      ...(label != null ? { label } : {}),
      worldX,
      worldZ,
      ...(rows.length ? { rows } : {}),
      ...(Number.isFinite(zoneRgb) ? { zoneRgb } : {}),
      ...(createdAt != null ? { createdAt } : {}),
    });
  }
  return normalized;
}

export function normalizeStatePatch(patch = {}) {
  /** @type {FishyMapStatePatch} */
  const normalized = {
    version: FISHYMAP_CONTRACT_VERSION,
  };

  if (isPlainObject(patch.theme)) {
    normalized.theme = {};
    if (typeof patch.theme.name === "string" && patch.theme.name.trim()) {
      normalized.theme.name = patch.theme.name.trim();
    }
    const colors = normalizeThemeColors(patch.theme.colors);
    if (colors) {
      normalized.theme.colors = colors;
    }
    if (!normalized.theme.name && !normalized.theme.colors) {
      delete normalized.theme;
    }
  }

  if (isPlainObject(patch.filters)) {
    normalized.filters = {};
    if (hasOwn(patch.filters, "fishIds")) {
      normalized.filters.fishIds = normalizeFishIds(patch.filters.fishIds);
    }
    if (hasOwn(patch.filters, "zoneRgbs")) {
      normalized.filters.zoneRgbs = normalizeZoneRgbs(patch.filters.zoneRgbs);
    }
    if (hasOwn(patch.filters, "searchText")) {
      normalized.filters.searchText = String(patch.filters.searchText ?? "");
    }
    const hasPatchId = hasOwn(patch.filters, "patchId");
    const hasFromPatchId = hasOwn(patch.filters, "fromPatchId");
    const hasToPatchId = hasOwn(patch.filters, "toPatchId");
    if (hasPatchId && !hasFromPatchId && !hasToPatchId) {
      const patchId = normalizeNullableString(patch.filters.patchId);
      normalized.filters.patchId = patchId;
      normalized.filters.fromPatchId = patchId;
      normalized.filters.toPatchId = patchId;
    } else if (hasPatchId || hasFromPatchId || hasToPatchId) {
      if (hasFromPatchId) {
        normalized.filters.fromPatchId = normalizeNullableString(patch.filters.fromPatchId);
      }
      if (hasToPatchId) {
        normalized.filters.toPatchId = normalizeNullableString(patch.filters.toPatchId);
      }
      normalized.filters.patchId =
        hasFromPatchId &&
        hasToPatchId &&
        normalized.filters.fromPatchId != null &&
        normalized.filters.fromPatchId === normalized.filters.toPatchId
          ? normalized.filters.fromPatchId
          : null;
    }
    if (hasOwn(patch.filters, "layerIdsVisible")) {
      normalized.filters.layerIdsVisible = normalizeStringList(patch.filters.layerIdsVisible);
    }
    if (hasOwn(patch.filters, "layerIdsOrdered")) {
      normalized.filters.layerIdsOrdered = normalizeStringList(patch.filters.layerIdsOrdered);
    }
    if (hasOwn(patch.filters, "layerOpacities")) {
      normalized.filters.layerOpacities = normalizeLayerOpacityMap(patch.filters.layerOpacities);
    }
    if (hasOwn(patch.filters, "layerClipMasks")) {
      normalized.filters.layerClipMasks = normalizeLayerClipMaskMap(patch.filters.layerClipMasks);
    }
    if (!Object.keys(normalized.filters).length) {
      delete normalized.filters;
    }
  }

  if (isPlainObject(patch.ui)) {
    normalized.ui = {};
    for (const key of [
      "diagnosticsOpen",
      "legendOpen",
      "leftPanelOpen",
      "showPoints",
      "showPointIcons",
    ]) {
      if (typeof patch.ui[key] === "boolean") {
        normalized.ui[key] = patch.ui[key];
      }
    }
    if (hasOwn(patch.ui, "pointIconScale")) {
      const pointIconScale = normalizePointIconScale(patch.ui.pointIconScale);
      if (pointIconScale !== undefined) {
        normalized.ui.pointIconScale = pointIconScale;
      }
    }
    if (hasOwn(patch.ui, "bookmarkSelectedIds")) {
      normalized.ui.bookmarkSelectedIds = normalizeStringList(patch.ui.bookmarkSelectedIds);
    }
    if (hasOwn(patch.ui, "bookmarks")) {
      normalized.ui.bookmarks = normalizeBookmarksState(patch.ui.bookmarks);
    }
    if (!Object.keys(normalized.ui).length) {
      delete normalized.ui;
    }
  }

  if (isPlainObject(patch.commands)) {
    normalized.commands = {};
    if (typeof patch.commands.resetView === "boolean") {
      normalized.commands.resetView = patch.commands.resetView;
    }
    if (patch.commands.setViewMode === "2d" || patch.commands.setViewMode === "3d") {
      normalized.commands.setViewMode = patch.commands.setViewMode;
    }
    if (hasOwn(patch.commands, "selectZoneRgb")) {
      const selectZoneRgb = Number.parseInt(patch.commands.selectZoneRgb, 10);
      if (Number.isFinite(selectZoneRgb)) {
        normalized.commands.selectZoneRgb = selectZoneRgb;
      }
    }
    if (hasOwn(patch.commands, "selectSemanticField")) {
      const selectSemanticField = normalizeSemanticFieldCommand(patch.commands.selectSemanticField);
      if (selectSemanticField) {
        normalized.commands.selectSemanticField = selectSemanticField;
      }
    }
    if (hasOwn(patch.commands, "selectWorldPoint")) {
      const selectWorldPoint = normalizeWorldPointCommand(patch.commands.selectWorldPoint);
      if (selectWorldPoint) {
        normalized.commands.selectWorldPoint = selectWorldPoint;
      }
    }
    if (hasOwn(patch.commands, "restoreView")) {
      const restoreView = normalizeRestoreView(patch.commands.restoreView);
      if (restoreView) {
        normalized.commands.restoreView = restoreView;
      }
    }
    if (normalized.commands.restoreView && normalized.commands.setViewMode) {
      normalized.commands.restoreView.viewMode = normalized.commands.setViewMode;
    }
    if (!Object.keys(normalized.commands).length) {
      delete normalized.commands;
    }
  }

  return normalized;
}

function patchHasStateFields(patch) {
  return !!(patch.theme || patch.filters || patch.ui);
}

function patchHasCommands(patch) {
  return !!(patch.commands && Object.keys(patch.commands).length);
}

function mergePatchBranch(target, source) {
  const out = { ...(target || {}) };
  for (const [key, value] of Object.entries(source || {})) {
    if (Array.isArray(value)) {
      out[key] = value.slice();
      continue;
    }
    if (isPlainObject(value)) {
      out[key] = mergePatchBranch(out[key], value);
      continue;
    }
    out[key] = value;
  }
  return out;
}

export function mergeStatePatch(left, right) {
  const base = normalizeStatePatch(left);
  const patch = normalizeStatePatch(right);
  /** @type {FishyMapStatePatch} */
  const out = {
    version: FISHYMAP_CONTRACT_VERSION,
  };
  if (base.theme || patch.theme) {
    out.theme = mergePatchBranch(base.theme, patch.theme);
  }
  if (base.filters || patch.filters) {
    out.filters = mergePatchBranch(base.filters, patch.filters);
    if (patch.filters && hasOwn(patch.filters, "layerOpacities")) {
      out.filters.layerOpacities = normalizeLayerOpacityMap(patch.filters.layerOpacities);
    } else if (base.filters && hasOwn(base.filters, "layerOpacities")) {
      out.filters.layerOpacities = normalizeLayerOpacityMap(base.filters.layerOpacities);
    }
    if (patch.filters && hasOwn(patch.filters, "layerClipMasks")) {
      out.filters.layerClipMasks = normalizeLayerClipMaskMap(patch.filters.layerClipMasks);
    } else if (base.filters && hasOwn(base.filters, "layerClipMasks")) {
      out.filters.layerClipMasks = normalizeLayerClipMaskMap(base.filters.layerClipMasks);
    }
  }
  if (base.ui || patch.ui) {
    out.ui = mergePatchBranch(base.ui, patch.ui);
  }
  if (base.commands || patch.commands) {
    out.commands = mergePatchBranch(base.commands, patch.commands);
  }
  return normalizeStatePatch(out);
}

export function applyStatePatch(inputState, patch) {
  const current = cloneJson(inputState || createEmptyInputState());
  const normalized = normalizeStatePatch(patch);
  const next = createEmptyInputState();
  next.theme = {
    name: current.theme?.name,
    colors: { ...(current.theme?.colors || {}) },
  };
  next.filters = {
    fishIds: normalizeFishIds(current.filters?.fishIds),
    zoneRgbs: normalizeZoneRgbs(current.filters?.zoneRgbs),
    searchText: String(current.filters?.searchText || ""),
    patchId: current.filters?.patchId ?? null,
    fromPatchId: current.filters?.fromPatchId ?? null,
    toPatchId: current.filters?.toPatchId ?? null,
    layerIdsVisible: Array.isArray(current.filters?.layerIdsVisible)
      ? normalizeStringList(current.filters.layerIdsVisible)
      : undefined,
    layerIdsOrdered: Array.isArray(current.filters?.layerIdsOrdered)
      ? normalizeStringList(current.filters.layerIdsOrdered)
      : undefined,
    layerOpacities: isPlainObject(current.filters?.layerOpacities)
      ? normalizeLayerOpacityMap(current.filters.layerOpacities)
      : undefined,
    layerClipMasks: isPlainObject(current.filters?.layerClipMasks)
      ? normalizeLayerClipMaskMap(current.filters.layerClipMasks)
      : undefined,
  };
  next.ui = {
    diagnosticsOpen: Boolean(current.ui?.diagnosticsOpen),
    legendOpen: Boolean(current.ui?.legendOpen),
    leftPanelOpen: current.ui?.leftPanelOpen !== false,
    showPoints: current.ui?.showPoints !== false,
    showPointIcons: current.ui?.showPointIcons !== false,
    pointIconScale:
      normalizePointIconScale(current.ui?.pointIconScale) ?? FISHYMAP_POINT_ICON_SCALE_MIN,
    bookmarkSelectedIds: normalizeStringList(current.ui?.bookmarkSelectedIds),
    bookmarks: normalizeBookmarksState(current.ui?.bookmarks),
  };

  if (normalized.theme) {
    if (hasOwn(normalized.theme, "name")) {
      next.theme.name = normalized.theme.name;
    }
    if (normalized.theme.colors) {
      next.theme.colors = {
        ...next.theme.colors,
        ...normalized.theme.colors,
      };
    }
  }

  if (normalized.filters) {
    if (hasOwn(normalized.filters, "fishIds")) {
      next.filters.fishIds = normalizeFishIds(normalized.filters.fishIds);
    }
    if (hasOwn(normalized.filters, "zoneRgbs")) {
      next.filters.zoneRgbs = normalizeZoneRgbs(normalized.filters.zoneRgbs);
    }
    if (hasOwn(normalized.filters, "searchText")) {
      next.filters.searchText = normalized.filters.searchText || "";
    }
    if (hasOwn(normalized.filters, "patchId")) {
      next.filters.patchId = normalized.filters.patchId ?? null;
    }
    if (hasOwn(normalized.filters, "fromPatchId")) {
      next.filters.fromPatchId = normalized.filters.fromPatchId ?? null;
    }
    if (hasOwn(normalized.filters, "toPatchId")) {
      next.filters.toPatchId = normalized.filters.toPatchId ?? null;
    }
    if (hasOwn(normalized.filters, "layerIdsVisible")) {
      next.filters.layerIdsVisible = normalizeStringList(normalized.filters.layerIdsVisible);
    }
    if (hasOwn(normalized.filters, "layerIdsOrdered")) {
      next.filters.layerIdsOrdered = normalizeStringList(normalized.filters.layerIdsOrdered);
    }
    if (hasOwn(normalized.filters, "layerOpacities")) {
      next.filters.layerOpacities = normalizeLayerOpacityMap(normalized.filters.layerOpacities);
    }
    if (hasOwn(normalized.filters, "layerClipMasks")) {
      next.filters.layerClipMasks = normalizeLayerClipMaskMap(normalized.filters.layerClipMasks);
    }
    if (
      hasOwn(normalized.filters, "patchId") ||
      hasOwn(normalized.filters, "fromPatchId") ||
      hasOwn(normalized.filters, "toPatchId")
    ) {
      next.filters.patchId =
        next.filters.fromPatchId &&
        next.filters.toPatchId &&
        next.filters.fromPatchId === next.filters.toPatchId
          ? next.filters.fromPatchId
          : null;
    }
  }

  if (normalized.ui) {
    if (hasOwn(normalized.ui, "diagnosticsOpen")) {
      next.ui.diagnosticsOpen = Boolean(normalized.ui.diagnosticsOpen);
    }
    if (hasOwn(normalized.ui, "legendOpen")) {
      next.ui.legendOpen = Boolean(normalized.ui.legendOpen);
    }
    if (hasOwn(normalized.ui, "leftPanelOpen")) {
      next.ui.leftPanelOpen = Boolean(normalized.ui.leftPanelOpen);
    }
    if (hasOwn(normalized.ui, "showPoints")) {
      next.ui.showPoints = Boolean(normalized.ui.showPoints);
    }
    if (hasOwn(normalized.ui, "showPointIcons")) {
      next.ui.showPointIcons = Boolean(normalized.ui.showPointIcons);
    }
    if (hasOwn(normalized.ui, "pointIconScale")) {
      next.ui.pointIconScale =
        normalizePointIconScale(normalized.ui.pointIconScale) ?? next.ui.pointIconScale;
    }
    if (hasOwn(normalized.ui, "bookmarkSelectedIds")) {
      next.ui.bookmarkSelectedIds = normalizeStringList(normalized.ui.bookmarkSelectedIds);
    }
    if (hasOwn(normalized.ui, "bookmarks")) {
      next.ui.bookmarks = normalizeBookmarksState(normalized.ui.bookmarks);
    }
  }

  return next;
}

function patchWithoutCommands(patch) {
  const normalized = normalizeStatePatch(patch);
  delete normalized.commands;
  return normalized;
}

function createCustomEvent(type, detail) {
  if (typeof CustomEvent === "function") {
    return new CustomEvent(type, { detail });
  }
  const event = new Event(type);
  event.detail = detail;
  return event;
}

function runtimeConfigBaseUrl(key) {
  return normalizeBaseUrl(globalThis.window?.__fishystuffRuntimeConfig?.[key]);
}

function runtimeConfigValue(key) {
  const normalized = String(globalThis.window?.__fishystuffRuntimeConfig?.[key] ?? "").trim();
  return normalized || "";
}

function isLoopbackHost(hostname) {
  return hostname === "127.0.0.1" || hostname === "localhost";
}

function harmonizeLoopbackBaseUrl(value, locationLike = globalThis.location) {
  const normalized = normalizeBaseUrl(value);
  if (!normalized) {
    return "";
  }
  try {
    const url = new URL(normalized, locationLike?.href);
    if (
      locationLike
      && isLoopbackHost(url.hostname)
      && isLoopbackHost(locationLike.hostname)
      && url.hostname !== locationLike.hostname
    ) {
      url.hostname = locationLike.hostname;
    }
    return normalizeBaseUrl(url.toString());
  } catch (_) {
    return normalized;
  }
}

export function resolveApiBaseUrl(locationLike = globalThis.location) {
  const explicit = harmonizeLoopbackBaseUrl(globalThis.window?.__fishystuffApiBaseUrl, locationLike);
  if (explicit) {
    return explicit;
  }
  const configured = harmonizeLoopbackBaseUrl(runtimeConfigBaseUrl("apiBaseUrl"), locationLike);
  if (configured) {
    return configured;
  }
  if (isLoopbackHost(locationLike?.hostname)) {
    const protocol = locationLike?.protocol === "https:" ? "https:" : "http:";
    return `${protocol}//${locationLike.hostname}:8080`;
  }
  return "https://api.fishystuff.fish";
}

function normalizeBaseUrl(value) {
  const normalized = String(value ?? "").trim();
  if (!normalized) {
    return "";
  }
  return normalized.replace(/\/+$/, "");
}

export function resolveCdnBaseUrl(
  locationLike = globalThis.location,
  explicitBaseUrl = globalThis.window?.__fishystuffCdnBaseUrl,
) {
  const explicit = harmonizeLoopbackBaseUrl(explicitBaseUrl, locationLike);
  if (explicit) {
    return explicit;
  }
  const configured = harmonizeLoopbackBaseUrl(runtimeConfigBaseUrl("cdnBaseUrl"), locationLike);
  if (configured) {
    return configured;
  }
  if (isLoopbackHost(locationLike?.hostname)) {
    const protocol = locationLike?.protocol === "https:" ? "https:" : "http:";
    return `${protocol}//${locationLike.hostname}:4040`;
  }
  return "https://cdn.fishystuff.fish";
}

export function resolveMapRuntimeBaseUrl(
  locationLike = globalThis.location,
  explicitBaseUrl = globalThis.window?.__fishystuffCdnBaseUrl,
) {
  return `${resolveCdnBaseUrl(locationLike, explicitBaseUrl)}/map/`;
}

function normalizeMapRuntimeManifestCacheKey(cacheKey) {
  const normalized = String(cacheKey ?? "").trim();
  if (!normalized) {
    return "";
  }
  return normalized.replace(/[^A-Za-z0-9._-]+/g, "-").replace(/^-+|-+$/g, "");
}

export function resolveMapRuntimeManifestUrl(
  locationLike = globalThis.location,
  cacheKey = runtimeConfigValue("mapAssetCacheKey"),
  explicitBaseUrl = globalThis.window?.__fishystuffCdnBaseUrl,
) {
  const normalizedCacheKey = normalizeMapRuntimeManifestCacheKey(cacheKey);
  const manifestFileName = normalizedCacheKey
    ? `runtime-manifest.${normalizedCacheKey}.json`
    : "runtime-manifest.json";
  return new URL(manifestFileName, resolveMapRuntimeBaseUrl(locationLike, explicitBaseUrl)).toString();
}

async function loadMapRuntimeManifest({
  locationLike = globalThis.location,
  cdnBaseUrl = globalThis.window?.__fishystuffCdnBaseUrl,
  cacheKey = runtimeConfigValue("mapAssetCacheKey"),
  fetchImpl = globalThis.fetch?.bind(globalThis),
} = {}) {
  if (typeof fetchImpl !== "function") {
    throw new Error("FishyMapBridge requires fetch() to load the runtime manifest");
  }
  const manifestUrl = resolveMapRuntimeManifestUrl(locationLike, cacheKey, cdnBaseUrl);
  const response = await fetchImpl(manifestUrl, { cache: "no-store" });
  if (!response?.ok) {
    throw new Error(`failed to load map runtime manifest: ${manifestUrl} (${response?.status || "unknown"})`);
  }
  const manifest = await response.json();
  const modulePath = String(manifest?.module || "").trim();
  if (!modulePath) {
    throw new Error(`invalid map runtime manifest: missing module in ${manifestUrl}`);
  }
  return {
    manifestUrl,
    moduleUrl: new URL(modulePath, manifestUrl).toString(),
    manifest,
  };
}

async function loadMapRuntimeModule(options = {}) {
  const { moduleUrl } = await loadMapRuntimeManifest({
    locationLike: options.locationLike ?? globalThis.location,
    cdnBaseUrl: options.cdnBaseUrl,
    cacheKey: options.runtimeManifestCacheKey ?? runtimeConfigValue("mapAssetCacheKey"),
    fetchImpl: options.fetchImpl ?? globalThis.fetch?.bind(globalThis),
  });
  return import(moduleUrl);
}

function ensureThemeProbe(doc) {
  if (!doc?.body) {
    return null;
  }
  let probe = doc.getElementById(THEME_PROBE_ID);
  if (probe) {
    return probe;
  }
  probe = doc.createElement("div");
  probe.id = THEME_PROBE_ID;
  probe.setAttribute("aria-hidden", "true");
  probe.style.position = "fixed";
  probe.style.width = "0";
  probe.style.height = "0";
  probe.style.overflow = "hidden";
  probe.style.opacity = "0";
  probe.style.pointerEvents = "none";
  probe.innerHTML = [
    '<div data-role="base" class="bg-base-100 text-base-content border border-base-300"></div>',
    '<div data-role="base-200" class="bg-base-200"></div>',
    '<div data-role="base-300" class="bg-base-300"></div>',
    '<div data-role="primary" class="bg-primary text-primary-content"></div>',
    '<div data-role="secondary" class="bg-secondary text-secondary-content"></div>',
    '<div data-role="accent" class="bg-accent text-accent-content"></div>',
    '<div data-role="neutral" class="bg-neutral text-neutral-content"></div>',
    '<div data-role="info" class="bg-info"></div>',
    '<div data-role="success" class="bg-success"></div>',
    '<div data-role="warning" class="bg-warning"></div>',
    '<div data-role="error" class="bg-error"></div>',
  ].join("");
  doc.body.appendChild(probe);
  return probe;
}

function readComputedColor(win, element, property) {
  if (!win || !element) {
    return "";
  }
  return String(win.getComputedStyle(element).getPropertyValue(property) || "").trim();
}

export function extractThemeSnapshot({
  doc = globalThis.document,
  win = globalThis.window,
} = {}) {
  const externalTheme = win?.__fishystuffTheme;
  if (externalTheme?.colors) {
    return {
      name:
        String(
          doc?.documentElement?.getAttribute?.("data-theme") ||
            externalTheme.resolvedTheme ||
            externalTheme.theme ||
            "",
        ).trim() || undefined,
      colors: normalizeThemeColors(externalTheme.colors, doc) || {},
    };
  }

  const probe = ensureThemeProbe(doc);
  const base = probe?.querySelector?.('[data-role="base"]') || null;
  const base200 = probe?.querySelector?.('[data-role="base-200"]') || null;
  const base300 = probe?.querySelector?.('[data-role="base-300"]') || null;
  const primary = probe?.querySelector?.('[data-role="primary"]') || null;
  const secondary = probe?.querySelector?.('[data-role="secondary"]') || null;
  const accent = probe?.querySelector?.('[data-role="accent"]') || null;
  const neutral = probe?.querySelector?.('[data-role="neutral"]') || null;
  const info = probe?.querySelector?.('[data-role="info"]') || null;
  const success = probe?.querySelector?.('[data-role="success"]') || null;
  const warning = probe?.querySelector?.('[data-role="warning"]') || null;
  const error = probe?.querySelector?.('[data-role="error"]') || null;

  return {
    name:
      String(doc?.documentElement?.getAttribute?.("data-theme") || "").trim() || undefined,
    colors: normalizeThemeColors({
      base100: readComputedColor(win, base, "background-color"),
      base200: readComputedColor(win, base200, "background-color"),
      base300: readComputedColor(win, base300, "background-color"),
      baseContent: readComputedColor(win, base, "color"),
      primary: readComputedColor(win, primary, "background-color"),
      primaryContent: readComputedColor(win, primary, "color"),
      secondary: readComputedColor(win, secondary, "background-color"),
      secondaryContent: readComputedColor(win, secondary, "color"),
      accent: readComputedColor(win, accent, "background-color"),
      accentContent: readComputedColor(win, accent, "color"),
      neutral: readComputedColor(win, neutral, "background-color"),
      neutralContent: readComputedColor(win, neutral, "color"),
      info: readComputedColor(win, info, "background-color"),
      success: readComputedColor(win, success, "background-color"),
      warning: readComputedColor(win, warning, "background-color"),
      error: readComputedColor(win, error, "background-color"),
    }, doc) || {},
  };
}

export function buildThemePatch(env) {
  const theme = extractThemeSnapshot(env);
  if (!theme.name && !Object.keys(theme.colors || {}).length) {
    return undefined;
  }
  return normalizeStatePatch({
    version: FISHYMAP_CONTRACT_VERSION,
    theme,
  });
}

function parseBooleanParam(value) {
  if (value == null) {
    return undefined;
  }
  const normalized = String(value).trim().toLowerCase();
  if (["1", "true", "yes", "on"].includes(normalized)) {
    return true;
  }
  if (["0", "false", "no", "off"].includes(normalized)) {
    return false;
  }
  return undefined;
}

function parseIntegerParam(value) {
  if (value == null || value === "") {
    return undefined;
  }
  const parsed = Number.parseInt(String(value), 10);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function parseLayerSetParam(value) {
  const normalized = String(value ?? "").trim();
  if (!normalized || normalized === "default") {
    return undefined;
  }
  return normalizeStringList(normalized.split(/[,+]/g));
}

export function parseQueryState(locationHref = globalThis.location?.href) {
  if (!locationHref) {
    return normalizeStatePatch({});
  }
  const url = new URL(locationHref, "https://fishystuff.fish");
  const params = url.searchParams;
  /** @type {FishyMapStatePatch} */
  const patch = {
    version: FISHYMAP_CONTRACT_VERSION,
  };

  const fishId =
    parseIntegerParam(params.get("focusFish")) ?? parseIntegerParam(params.get("fish"));
  const patchId = params.get("patch");
  const fromPatchId = params.get("fromPatch") ?? params.get("patchFrom");
  const toPatchId =
    params.get("toPatch") ?? params.get("untilPatch") ?? params.get("patchTo");
  const searchText = params.get("search");
  const diagnosticsOpen = parseBooleanParam(params.get("diagnostics"));
  const legendOpen = parseBooleanParam(params.get("legend"));
  const layers =
    parseLayerSetParam(params.get("layers")) ?? parseLayerSetParam(params.get("layerSet"));
  const viewMode = params.get("view") === "3d" || params.get("mode") === "3d" ? "3d" : undefined;
  const zoneRgb = parseIntegerParam(params.get("zone"));
  const worldX = normalizeBookmarkCoordinate(params.get("worldX") ?? params.get("x"));
  const worldZ = normalizeBookmarkCoordinate(params.get("worldZ") ?? params.get("z"));

  if (
    fishId != null ||
    patchId != null ||
    fromPatchId != null ||
    toPatchId != null ||
    searchText != null ||
    layers != null
  ) {
    patch.filters = {};
  }
  if (fishId != null) {
    patch.filters.fishIds = [fishId];
  }
  if (fromPatchId != null || toPatchId != null) {
    if (fromPatchId != null) {
      patch.filters.fromPatchId = fromPatchId || null;
    }
    if (toPatchId != null) {
      patch.filters.toPatchId = toPatchId || null;
    }
  } else if (patchId != null) {
    patch.filters.patchId = patchId || null;
  }
  if (searchText != null) {
    patch.filters.searchText = searchText;
  }
  if (layers != null) {
    patch.filters.layerIdsVisible = layers;
  }

  if (diagnosticsOpen != null || legendOpen != null) {
    patch.ui = {};
  }
  if (diagnosticsOpen != null) {
    patch.ui.diagnosticsOpen = diagnosticsOpen;
  }
  if (legendOpen != null) {
    patch.ui.legendOpen = legendOpen;
  }

  if (viewMode || zoneRgb != null || (worldX !== undefined && worldZ !== undefined)) {
    patch.commands = { ...(patch.commands || {}) };
  }
  if (viewMode) {
    patch.commands.setViewMode = viewMode;
  }
  if (worldX !== undefined && worldZ !== undefined) {
    patch.commands.selectWorldPoint = { worldX, worldZ };
  } else if (zoneRgb != null) {
    patch.commands.selectZoneRgb = zoneRgb;
  }

  return normalizeStatePatch(patch);
}

function readJsonStorage(storage, key) {
  if (!storage) {
    return null;
  }
  try {
    const raw = storage.getItem(key);
    if (!raw) {
      return null;
    }
    return JSON.parse(raw);
  } catch (_) {
    return null;
  }
}

export function snapshotToRestorePatch(snapshot) {
  if (!isPlainObject(snapshot)) {
    return normalizeStatePatch({});
  }
  /** @type {FishyMapStatePatch} */
  const patch = {
    version: FISHYMAP_CONTRACT_VERSION,
  };
  if (isPlainObject(snapshot.filters) || isPlainObject(snapshot.ui)) {
    patch.filters = {};
    patch.ui = {};
  }
  if (isPlainObject(snapshot.filters)) {
    if (hasOwn(snapshot.filters, "fishIds")) {
      patch.filters.fishIds = normalizeFishIds(snapshot.filters.fishIds);
    }
    if (hasOwn(snapshot.filters, "zoneRgbs")) {
      patch.filters.zoneRgbs = normalizeZoneRgbs(snapshot.filters.zoneRgbs);
    }
    if (hasOwn(snapshot.filters, "searchText")) {
      patch.filters.searchText = String(snapshot.filters.searchText || "");
    }
    if (hasOwn(snapshot.filters, "patchId")) {
      patch.filters.patchId = snapshot.filters.patchId ?? null;
    }
    if (hasOwn(snapshot.filters, "fromPatchId")) {
      patch.filters.fromPatchId = snapshot.filters.fromPatchId ?? null;
    }
    if (hasOwn(snapshot.filters, "toPatchId")) {
      patch.filters.toPatchId = snapshot.filters.toPatchId ?? null;
    }
    const layerIdsVisible = readPersistedLayerVisibility(snapshot.filters);
    if (layerIdsVisible !== undefined) {
      patch.filters.layerIdsVisible = layerIdsVisible;
    }
    const layerIdsOrdered = readPersistedLayerOrder(snapshot.filters);
    if (layerIdsOrdered !== undefined) {
      patch.filters.layerIdsOrdered = layerIdsOrdered;
    }
    const layerOpacities = readPersistedLayerOpacities(snapshot.filters);
    if (layerOpacities !== undefined) {
      patch.filters.layerOpacities = layerOpacities;
    }
    const layerClipMasks = readPersistedLayerClipMasks(snapshot.filters);
    if (layerClipMasks !== undefined) {
      patch.filters.layerClipMasks = layerClipMasks;
    }
  }
  if (isPlainObject(snapshot.ui)) {
    if (hasOwn(snapshot.ui, "diagnosticsOpen")) {
      patch.ui.diagnosticsOpen = Boolean(snapshot.ui.diagnosticsOpen);
    }
    if (hasOwn(snapshot.ui, "legendOpen")) {
      patch.ui.legendOpen = Boolean(snapshot.ui.legendOpen);
    }
    if (hasOwn(snapshot.ui, "leftPanelOpen")) {
      patch.ui.leftPanelOpen = Boolean(snapshot.ui.leftPanelOpen);
    }
    if (hasOwn(snapshot.ui, "showPoints")) {
      patch.ui.showPoints = Boolean(snapshot.ui.showPoints);
    }
    if (hasOwn(snapshot.ui, "showPointIcons")) {
      patch.ui.showPointIcons = Boolean(snapshot.ui.showPointIcons);
    }
    if (hasOwn(snapshot.ui, "pointIconScale")) {
      const pointIconScale = normalizePointIconScale(snapshot.ui.pointIconScale);
      if (pointIconScale !== undefined) {
        patch.ui.pointIconScale = pointIconScale;
      }
    }
  }

  if (patch.filters && !Object.keys(patch.filters).length) {
    delete patch.filters;
  }
  if (patch.ui && !Object.keys(patch.ui).length) {
    delete patch.ui;
  }

  const selectionFishId = parseIntegerParam(snapshot.selection?.fishId);
  const selectionZoneRgb = parseIntegerParam(snapshot.selection?.zoneRgb);
  const selectionWorldX = normalizeBookmarkCoordinate(snapshot.selection?.worldX);
  const selectionWorldZ = normalizeBookmarkCoordinate(snapshot.selection?.worldZ);
  const restoreView = normalizeRestoreView(snapshot.view);
  if (
    selectionFishId != null ||
    selectionZoneRgb != null ||
    (selectionWorldX !== undefined && selectionWorldZ !== undefined) ||
    restoreView
  ) {
    patch.commands = {};
  }
  if (selectionFishId != null) {
    const restoredFishIds = normalizeFishIds(patch.filters?.fishIds);
    patch.filters = patch.filters || {};
    patch.filters.fishIds = restoredFishIds.length
      ? restoredFishIds.includes(selectionFishId)
        ? restoredFishIds
        : restoredFishIds.concat(selectionFishId)
      : [selectionFishId];
  }
  if (selectionWorldX !== undefined && selectionWorldZ !== undefined) {
    patch.commands.selectWorldPoint = {
      worldX: selectionWorldX,
      worldZ: selectionWorldZ,
    };
  } else if (selectionZoneRgb != null) {
    patch.commands.selectZoneRgb = selectionZoneRgb;
  }
  if (restoreView) {
    patch.commands.restoreView = restoreView;
    patch.commands.setViewMode = restoreView.viewMode;
  }

  return normalizeStatePatch(patch);
}

export function loadSessionRestorePatch(storage = globalThis.sessionStorage) {
  return snapshotToRestorePatch(readJsonStorage(storage, FISHYMAP_STORAGE_KEYS.session));
}

export function loadLocalPrefsPatch(storage = globalThis.localStorage) {
  return snapshotToRestorePatch(readJsonStorage(storage, FISHYMAP_STORAGE_KEYS.prefs));
}

export function buildInitialRestorePatch({
  locationHref = globalThis.location?.href,
  sessionStorage = globalThis.sessionStorage,
  localStorage = globalThis.localStorage,
  defaults,
} = {}) {
  let merged = normalizeStatePatch(defaults || {});
  merged = mergeStatePatch(merged, loadLocalPrefsPatch(localStorage));
  merged = mergeStatePatch(merged, loadSessionRestorePatch(sessionStorage));
  merged = mergeStatePatch(merged, parseQueryState(locationHref));
  return merged;
}

function stripUndefined(obj) {
  if (!isPlainObject(obj)) {
    return obj;
  }
  const out = {};
  for (const [key, value] of Object.entries(obj)) {
    if (value === undefined) {
      continue;
    }
    if (isPlainObject(value)) {
      const nested = stripUndefined(value);
      if (nested && Object.keys(nested).length) {
        out[key] = nested;
      }
      continue;
    }
    out[key] = value;
  }
  return out;
}

function stableStringify(value) {
  return JSON.stringify(stripUndefined(cloneJson(value)));
}

function bootstrapStateSignature(state) {
  const filters = state?.filters || {};
  const ui = state?.ui || {};
  const statuses = state?.statuses || {};
  const catalog = state?.catalog || {};
  return stableStringify({
    ready: Boolean(state?.ready),
    viewMode: state?.view?.viewMode || null,
    selectionZoneRgb: zoneRgbFromLayerSamples(state?.selection?.layerSamples),
    filters: {
      patchId: filters.patchId ?? null,
      fromPatchId: filters.fromPatchId ?? null,
      toPatchId: filters.toPatchId ?? null,
      fishIdsCount: Array.isArray(filters.fishIds) ? filters.fishIds.length : 0,
      visibleLayerCount: Array.isArray(filters.layerIdsVisible)
        ? filters.layerIdsVisible.length
        : null,
    },
      ui: {
        diagnosticsOpen: Boolean(ui.diagnosticsOpen),
        legendOpen: Boolean(ui.legendOpen),
        leftPanelOpen: Boolean(ui.leftPanelOpen),
        showPoints: Boolean(ui.showPoints),
        showPointIcons: Boolean(ui.showPointIcons),
        pointIconScale: Number(ui.pointIconScale || 0),
      },
    statuses: {
      metaStatus: statuses.metaStatus ?? null,
      layersStatus: statuses.layersStatus ?? null,
      zonesStatus: statuses.zonesStatus ?? null,
      pointsStatus: statuses.pointsStatus ?? null,
      fishStatus: statuses.fishStatus ?? null,
      zoneStatsStatus: statuses.zoneStatsStatus ?? null,
    },
    catalog: {
      capabilityCount: Array.isArray(catalog.capabilities) ? catalog.capabilities.length : 0,
      layerCount: Array.isArray(catalog.layers) ? catalog.layers.length : 0,
      patchCount: Array.isArray(catalog.patches) ? catalog.patches.length : 0,
      fishCount: Array.isArray(catalog.fish) ? catalog.fish.length : 0,
    },
  });
}

function fishCatalogPending(state) {
  return String(state?.statuses?.fishStatus || "")
    .trim()
    .toLowerCase() === "fish: pending";
}

function mergeBootstrapSnapshot(currentState, bootstrapState) {
  const current = currentState || createEmptySnapshot();
  const parsed = bootstrapState || {};
  return {
    ...current,
    version: Number(parsed.version || current.version || FISHYMAP_CONTRACT_VERSION),
    ready: parsed.ready === true,
    statuses: {
      ...createEmptySnapshot().statuses,
      ...(current.statuses || {}),
      ...(parsed.statuses || {}),
    },
  };
}

function isMeaningfulPatch(patch) {
  const normalized = normalizeStatePatch(patch);
  return patchHasStateFields(normalized) || patchHasCommands(normalized);
}

class FishyMapBridgeImpl {
  constructor() {
    this.eventTarget = new EventTarget();
    this.container = null;
    this.canvas = null;
    this.wasmModule = null;
    this.wasmReady = false;
    this.inputState = createEmptyInputState();
    this.currentState = createEmptySnapshot();
    this.pendingStatePatch = normalizeStatePatch({});
    this.pendingCommands = [];
    this.patchDebounceMs = DEFAULT_PATCH_DEBOUNCE_MS;
    this.sessionSaveDebounceMs = DEFAULT_SESSION_SAVE_MS;
    this.flushPatchTimer = 0;
    this.flushSessionTimer = 0;
    this.bootstrapSyncTimer = 0;
    this.bootstrapSyncPasses = 0;
    this.resizeObserver = null;
    this.themeObserver = null;
    this.boundEventSink = (json) => this.handleWasmEvent(json);
    this.boundSetState = (event) => {
      this.setState(event?.detail || {});
    };
    this.boundCommand = (event) => {
      this.sendCommand(event?.detail || {});
    };
    this.boundRequestState = (event) => {
      const detail = event?.detail || {};
      detail.state = cloneJson(this.refreshCurrentStateFromWasm());
      detail.inputState = this.getCurrentInputState();
      if (typeof detail.callback === "function") {
        detail.callback(detail.state, detail.inputState);
      }
    };
    this.boundThemeSync = () => {
      this.syncTheme();
    };
    this.boundResize = () => {
      this.syncCanvasSize();
    };
    this.boundPageHide = () => {
      this.flushSessionStateSave();
      this.saveLocalPrefsNow();
    };
    this.boundVisibilityChange = () => {
      if (globalThis.document?.visibilityState === "hidden") {
        this.flushSessionStateSave();
        this.saveLocalPrefsNow();
      }
    };
    this.performanceOptions = normalizePerformanceOptions({ scenario: "load_map" });
    this.performanceCollector = createPerformanceCollector(this.performanceOptions.scenario);
  }

  measurePerformanceSpan(name, callback) {
    const startedAtMs = performanceNowMs();
    try {
      return callback();
    } finally {
      addPerformanceSpanSample(this.performanceCollector, name, performanceNowMs() - startedAtMs);
    }
  }

  async measurePerformanceSpanAsync(name, callback) {
    const startedAtMs = performanceNowMs();
    try {
      return await callback();
    } finally {
      addPerformanceSpanSample(this.performanceCollector, name, performanceNowMs() - startedAtMs);
    }
  }

  addPerformanceCounter(name, delta = 1) {
    addPerformanceCounter(this.performanceCollector, name, delta);
  }

  syncWasmProfiling() {
    if (!this.wasmReady || !this.wasmModule?.fishymap_reset_profiling_json) {
      return;
    }
    this.wasmModule.fishymap_reset_profiling_json(
      JSON.stringify({
        scenario: this.performanceOptions.scenario,
        warmupFrames: this.performanceOptions.warmupFrames,
        captureTrace: this.performanceOptions.captureTrace,
      }),
    );
  }

  async mount(container, options = {}) {
    return this.measurePerformanceSpanAsync("host.mount", async () => {
      if (!container) {
        throw new Error("FishyMapBridge.mount requires a container element");
      }
      if (this.container) {
        this.destroy();
      }
      this.resetPerformanceSnapshot({ scenario: options.profileScenario || "load_map" });
      this.patchDebounceMs =
        Number.isFinite(options.debounceMs) && options.debounceMs >= 0
          ? options.debounceMs
          : DEFAULT_PATCH_DEBOUNCE_MS;
      this.sessionSaveDebounceMs =
        Number.isFinite(options.sessionSaveDebounceMs) && options.sessionSaveDebounceMs >= 0
          ? options.sessionSaveDebounceMs
          : DEFAULT_SESSION_SAVE_MS;

      this.container = container;
      this.canvas =
        options.canvas ||
        container.querySelector?.("canvas") ||
        globalThis.document?.getElementById?.("bevy") ||
        null;

      this.attachDomListeners();
      this.installCanvasSizeSync();
      this.installThemeSync();
      this.installPersistenceHooks();

      const wasmModule = options.wasmModule || (await loadMapRuntimeModule(options));
      try {
        await wasmModule.default();
      } catch (error) {
        const message =
          error && typeof error === "object" && "message" in error
            ? String(error.message)
            : String(error);
        if (!message.includes("Using exceptions for control flow")) {
          throw error;
        }
      }
      this.wasmModule = wasmModule;
      if (typeof wasmModule.fishymap_set_event_sink === "function") {
        wasmModule.fishymap_set_event_sink(this.boundEventSink);
      }
      if (typeof wasmModule.fishymap_mount === "function") {
        wasmModule.fishymap_mount();
      }
      this.wasmReady = true;
      this.syncWasmProfiling();
      this.refreshCurrentStateFromWasm();

      const initialRestorePatch = mergeStatePatch(
        options.initialState,
        buildInitialRestorePatch(options),
      );
      if (isMeaningfulPatch(initialRestorePatch)) {
        this.setState(initialRestorePatch);
      }
      this.syncTheme();
      this.flushPendingPatchNow();
      this.flushQueuedCommands();
      this.scheduleBootstrapStateSync();
      return this.getCurrentState();
    });
  }

  destroy() {
    this.measurePerformanceSpan("host.destroy", () => {
      globalThis.clearTimeout(this.flushPatchTimer);
      globalThis.clearTimeout(this.flushSessionTimer);
      globalThis.clearTimeout(this.bootstrapSyncTimer);
      this.flushPatchTimer = 0;
      this.flushSessionTimer = 0;
      this.bootstrapSyncTimer = 0;
      this.bootstrapSyncPasses = 0;
      this.detachDomListeners();
      this.teardownCanvasSizeSync();
      this.teardownThemeSync();
      this.teardownPersistenceHooks();

      if (this.wasmModule?.fishymap_destroy) {
        this.wasmModule.fishymap_destroy();
      } else if (this.wasmModule?.fishymap_clear_event_sink) {
        this.wasmModule.fishymap_clear_event_sink();
      }

      this.wasmModule = null;
      this.wasmReady = false;
      this.container = null;
      this.canvas = null;
      this.pendingStatePatch = normalizeStatePatch({});
      this.pendingCommands = [];
      this.inputState = createEmptyInputState();
      this.currentState = createEmptySnapshot();
    });
  }

  setState(patch) {
    this.measurePerformanceSpan("host.set_state", () => {
      const normalized = normalizeStatePatch(patch);
      if (!patchHasStateFields(normalized) && !patchHasCommands(normalized)) {
        return;
      }

      if (patchHasStateFields(normalized)) {
        const nextInputState = applyStatePatch(this.inputState, normalized);
        if (stableStringify(nextInputState) !== stableStringify(this.inputState)) {
          this.inputState = nextInputState;
          this.pendingStatePatch = mergeStatePatch(
            this.pendingStatePatch,
            patchWithoutCommands(normalized),
          );
          this.addPerformanceCounter("host.patches.queued");
          this.saveLocalPrefsNow();
          this.schedulePatchFlush();
        }
      }

      if (patchHasCommands(normalized)) {
        this.sendCommand(normalized.commands);
      }
    });
  }

  sendCommand(command) {
    this.measurePerformanceSpan("host.send_command", () => {
      const normalized = normalizeStatePatch({ commands: command }).commands;
      if (!normalized || !Object.keys(normalized).length) {
        return;
      }
      this.addPerformanceCounter("host.commands.sent");
      if (!this.wasmReady || !this.wasmModule?.fishymap_send_command_json) {
        this.pendingCommands.push(normalized);
        this.addPerformanceCounter("host.commands.buffered");
        return;
      }
      this.wasmModule.fishymap_send_command_json(JSON.stringify(normalized));
    });
  }

  getCurrentState() {
    return cloneJson(this.currentState);
  }

  refreshCurrentStateNow() {
    return cloneJson(this.refreshCurrentStateFromWasm());
  }

  getCurrentInputState() {
    return cloneJson(this.inputState);
  }

  resetPerformanceSnapshot(options = {}) {
    this.performanceOptions = normalizePerformanceOptions({
      ...this.performanceOptions,
      ...options,
    });
    this.performanceCollector = createPerformanceCollector(this.performanceOptions.scenario);
    this.syncWasmProfiling();
    return this.getPerformanceSnapshot();
  }

  readWasmPerformanceSummary() {
    if (!this.wasmReady || !this.wasmModule?.fishymap_get_profiling_summary_json) {
      return null;
    }
    try {
      return JSON.parse(this.wasmModule.fishymap_get_profiling_summary_json());
    } catch (_) {
      return null;
    }
  }

  getPerformanceTraceJson() {
    if (!this.wasmReady || !this.wasmModule?.fishymap_get_profiling_trace_json) {
      return "";
    }
    try {
      return String(this.wasmModule.fishymap_get_profiling_trace_json() || "");
    } catch (_) {
      return "";
    }
  }

  getPerformanceSnapshot() {
    const host = snapshotPerformanceCollector(this.performanceCollector);
    const wasm =
      this.readWasmPerformanceSummary() ||
      emptyProfileSummary(host.scenario, this.performanceOptions.warmupFrames);
    return {
      scenario: wasm.scenario || host.scenario,
      bevy_version: wasm.bevy_version ?? null,
      git_revision: wasm.git_revision ?? null,
      build_profile: wasm.build_profile ?? "browser",
      frames: wasm.frames ?? 0,
      warmup_frames: wasm.warmup_frames ?? this.performanceOptions.warmupFrames,
      wall_clock_ms: wasm.wall_clock_ms ?? host.elapsed_ms,
      browser_elapsed_ms: host.elapsed_ms,
      frame_time_ms: wasm.frame_time_ms || emptyQuantileSummary(),
      named_spans: {
        ...host.named_spans,
        ...(wasm.named_spans || {}),
      },
      counters: {
        ...host.counters,
        ...(wasm.counters || {}),
      },
      bridge_state: this.getCurrentState(),
      host,
      wasm,
    };
  }

  on(type, handler) {
    this.eventTarget.addEventListener(type, handler);
  }

  off(type, handler) {
    this.eventTarget.removeEventListener(type, handler);
  }

  emit(type, detail) {
    this.measurePerformanceSpan("host.emit", () => {
      const suffix = normalizePerfCounterSuffix(type) || "unknown";
      this.addPerformanceCounter(`host.events.emitted.${suffix}`);
      const event = createCustomEvent(type, detail);
      this.eventTarget.dispatchEvent(event);
      if (this.container?.dispatchEvent) {
        this.container.dispatchEvent(createCustomEvent(type, detail));
      }
    });
  }

  attachDomListeners() {
    if (!this.container) {
      return;
    }
    this.container.addEventListener(FISHYMAP_EVENTS.setState, this.boundSetState);
    this.container.addEventListener(FISHYMAP_EVENTS.command, this.boundCommand);
    this.container.addEventListener(FISHYMAP_EVENTS.requestState, this.boundRequestState);
  }

  detachDomListeners() {
    if (!this.container) {
      return;
    }
    this.container.removeEventListener(FISHYMAP_EVENTS.setState, this.boundSetState);
    this.container.removeEventListener(FISHYMAP_EVENTS.command, this.boundCommand);
    this.container.removeEventListener(FISHYMAP_EVENTS.requestState, this.boundRequestState);
  }

  installCanvasSizeSync() {
    this.syncCanvasSize();
    if (globalThis.window?.addEventListener) {
      globalThis.window.addEventListener("resize", this.boundResize, { passive: true });
    }
    const resizeTarget = this.container || this.canvas?.parentElement || null;
    if (resizeTarget && typeof ResizeObserver !== "undefined") {
      this.resizeObserver = new ResizeObserver(() => this.syncCanvasSize());
      this.resizeObserver.observe(resizeTarget);
    }
  }

  teardownCanvasSizeSync() {
    globalThis.window?.removeEventListener?.("resize", this.boundResize);
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
      this.resizeObserver = null;
    }
  }

  syncCanvasSize() {
    if (!this.canvas) {
      return;
    }
    const measurementTarget = this.container || this.canvas.parentElement || this.canvas;
    const targetRect = measurementTarget?.getBoundingClientRect?.() || {};
    const canvasRect = this.canvas.getBoundingClientRect?.() || {};
    const logicalWidth = Math.max(
      1,
      Math.round(
        targetRect.width ||
          canvasRect.width ||
          measurementTarget?.clientWidth ||
          this.canvas.clientWidth ||
          0,
      ),
    );
    const logicalHeight = Math.max(
      1,
      Math.round(
        targetRect.height ||
          canvasRect.height ||
          measurementTarget?.clientHeight ||
          this.canvas.clientHeight ||
          0,
      ),
    );
    const dpr = Math.max(1, globalThis.window?.devicePixelRatio || 1);
    const physicalWidth = Math.max(1, Math.round(logicalWidth * dpr));
    const physicalHeight = Math.max(1, Math.round(logicalHeight * dpr));
    this.canvas.style.width = `${logicalWidth}px`;
    this.canvas.style.height = `${logicalHeight}px`;
    if (this.canvas.width !== physicalWidth) {
      this.canvas.width = physicalWidth;
    }
    if (this.canvas.height !== physicalHeight) {
      this.canvas.height = physicalHeight;
    }
  }

  installThemeSync() {
    if (globalThis.window?.addEventListener) {
      globalThis.window.addEventListener(THEME_EVENT, this.boundThemeSync);
    }
    if (globalThis.document?.documentElement && typeof MutationObserver !== "undefined") {
      this.themeObserver = new MutationObserver(() => this.syncTheme());
      this.themeObserver.observe(globalThis.document.documentElement, {
        attributes: true,
        attributeFilter: ["data-theme"],
      });
    }
  }

  teardownThemeSync() {
    if (globalThis.window?.removeEventListener) {
      globalThis.window.removeEventListener(THEME_EVENT, this.boundThemeSync);
    }
    if (this.themeObserver) {
      this.themeObserver.disconnect();
      this.themeObserver = null;
    }
  }

  installPersistenceHooks() {
    globalThis.window?.addEventListener?.("pagehide", this.boundPageHide);
    globalThis.document?.addEventListener?.("visibilitychange", this.boundVisibilityChange);
  }

  teardownPersistenceHooks() {
    globalThis.window?.removeEventListener?.("pagehide", this.boundPageHide);
    globalThis.document?.removeEventListener?.("visibilitychange", this.boundVisibilityChange);
  }

  syncTheme() {
    const themePatch = buildThemePatch();
    if (themePatch) {
      this.setState(themePatch);
    }
  }

  schedulePatchFlush() {
    if (!this.wasmReady) {
      return;
    }
    if (this.flushPatchTimer) {
      return;
    }
    this.flushPatchTimer = globalThis.setTimeout(() => {
      this.flushPatchTimer = 0;
      this.flushPendingPatchNow();
    }, this.patchDebounceMs);
  }

  flushPendingPatchNow() {
    this.measurePerformanceSpan("host.patch_flush", () => {
      if (!this.wasmReady || !this.wasmModule?.fishymap_apply_state_patch_json) {
        return;
      }
      if (!patchHasStateFields(this.pendingStatePatch)) {
        return;
      }
      const patch = this.pendingStatePatch;
      this.pendingStatePatch = normalizeStatePatch({});
      this.addPerformanceCounter("host.patches.flushed");
      this.wasmModule.fishymap_apply_state_patch_json(JSON.stringify(patch));
    });
  }

  flushQueuedCommands() {
    this.measurePerformanceSpan("host.command_flush", () => {
      if (!this.wasmReady || !this.wasmModule?.fishymap_send_command_json) {
        return;
      }
      const commands = this.pendingCommands.splice(0, this.pendingCommands.length);
      this.addPerformanceCounter("host.commands.flushed", commands.length);
      for (const command of commands) {
        this.wasmModule.fishymap_send_command_json(JSON.stringify(command));
      }
    });
  }

  scheduleBootstrapStateSync() {
    if (!this.wasmReady || this.bootstrapSyncTimer) {
      return;
    }
    this.bootstrapSyncTimer = globalThis.setTimeout(() => {
      this.bootstrapSyncTimer = 0;
      this.runBootstrapStateSync();
    }, DEFAULT_BOOTSTRAP_SYNC_MS);
  }

  runBootstrapStateSync() {
    this.measurePerformanceSpan("host.bootstrap_sync", () => {
      if (!this.wasmReady) {
        return;
      }
      this.bootstrapSyncPasses += 1;
      this.addPerformanceCounter("host.bootstrap.passes");
      const previousState = this.currentState;
      const previousSignature = bootstrapStateSignature(previousState);
      const wasReady = previousState.ready === true;
      const fishWasPending = fishCatalogPending(previousState);

      this.syncCanvasSize();
      this.refreshBootstrapStateFromWasm();

      const becameReady = !wasReady && this.currentState.ready;
      const fishFinishedLoading =
        this.currentState.ready && fishWasPending && !fishCatalogPending(this.currentState);

      if (becameReady || fishFinishedLoading) {
        this.refreshCurrentStateFromWasm();
      }

      if (bootstrapStateSignature(this.currentState) !== previousSignature) {
        if (becameReady) {
          this.scheduleSessionStateSave();
          this.emit(FISHYMAP_EVENTS.ready, {
            type: "ready",
            version: this.currentState.version || FISHYMAP_CONTRACT_VERSION,
            capabilities: cloneJson(this.currentState.catalog?.capabilities || []),
            state: this.getCurrentState(),
            inputState: this.getCurrentInputState(),
          });
        } else if (wasReady && this.currentState.ready) {
          this.emit(FISHYMAP_EVENTS.stateChanged, {
            type: "state-changed",
            version: this.currentState.version || FISHYMAP_CONTRACT_VERSION,
            state: this.getCurrentState(),
            inputState: this.getCurrentInputState(),
          });
        }
      }

      const shouldContinue =
        this.bootstrapSyncPasses < MIN_BOOTSTRAP_SYNC_PASSES ||
        ((!this.currentState.ready || fishCatalogPending(this.currentState)) &&
          this.bootstrapSyncPasses < MAX_BOOTSTRAP_SYNC_PASSES);
      if (shouldContinue) {
        this.scheduleBootstrapStateSync();
        return;
      }
      this.bootstrapSyncPasses = 0;
    });
  }

  refreshCurrentStateFromWasm() {
    return this.measurePerformanceSpan("host.state_pull", () => {
      if (!this.wasmReady || !this.wasmModule?.fishymap_get_current_state_json) {
        return this.currentState;
      }
      this.addPerformanceCounter("host.wasm.state_reads");
      try {
        const parsed = JSON.parse(this.wasmModule.fishymap_get_current_state_json());
        const nextState = {
          ...createEmptySnapshot(),
          ...parsed,
        };
        this.currentState = {
          ...nextState,
          selection: normalizeSelectionSnapshotValue(nextState.selection),
          hover: normalizeHoverSnapshotValue(nextState.hover),
        };
      } catch (_) {
        this.currentState = createEmptySnapshot();
      }
      return this.currentState;
    });
  }

  refreshBootstrapStateFromWasm() {
    return this.measurePerformanceSpan("host.bootstrap_pull", () => {
      if (!this.wasmReady) {
        return this.currentState;
      }
      if (!this.wasmModule?.fishymap_get_bootstrap_state_json) {
        return this.refreshCurrentStateFromWasm();
      }
      this.addPerformanceCounter("host.wasm.bootstrap_reads");
      try {
        const parsed = JSON.parse(this.wasmModule.fishymap_get_bootstrap_state_json());
        this.currentState = mergeBootstrapSnapshot(this.currentState, parsed);
      } catch (_) {
        this.currentState = mergeBootstrapSnapshot(this.currentState, createEmptySnapshot());
      }
      return this.currentState;
    });
  }

  handleWasmEvent(json) {
    this.measurePerformanceSpan("host.handle_event", () => {
      let payload;
      try {
        payload = JSON.parse(json);
      } catch (error) {
        this.addPerformanceCounter("host.events.invalid");
        this.emit(FISHYMAP_EVENTS.diagnostic, {
          type: "diagnostic",
          version: FISHYMAP_CONTRACT_VERSION,
          payload: {
            bridgeError: `invalid event payload: ${error}`,
            raw: String(json),
          },
          state: this.getCurrentState(),
        });
        return;
      }

      const type = String(payload.type || "");
      const suffix = normalizePerfCounterSuffix(type) || "unknown";
      this.addPerformanceCounter(`host.events.handled.${suffix}`);
      if (type === "hover-changed") {
        const hover = normalizeHoverSnapshotValue({
          worldX: payload.worldX,
          worldZ: payload.worldZ,
          layerSamples: Array.isArray(payload.layerSamples) ? payload.layerSamples : [],
        });
        this.currentState = {
          ...this.currentState,
          hover,
        };
        this.emit(FISHYMAP_EVENTS.hoverChanged, {
          ...payload,
          hover: cloneJson(this.currentState.hover),
        });
        return;
      }

      if (type === "view-changed") {
        this.currentState = {
          ...this.currentState,
          view: {
            ...this.currentState.view,
            viewMode: payload.viewMode ?? this.currentState.view?.viewMode ?? "2d",
            camera: payload.camera ? cloneJson(payload.camera) : this.currentState.view?.camera,
          },
        };
        this.scheduleSessionStateSave();
        this.emit(FISHYMAP_EVENTS.viewChanged, {
          ...payload,
          state: {
            view: cloneJson(this.currentState.view),
          },
          inputState: this.getCurrentInputState(),
        });
        return;
      }

      this.refreshCurrentStateFromWasm();

      const detail = {
        ...payload,
        state: this.getCurrentState(),
        inputState: this.getCurrentInputState(),
      };

      if (type === "selection-changed") {
        this.scheduleSessionStateSave();
        this.emit(FISHYMAP_EVENTS.selectionChanged, detail);
        return;
      }
      if (type === "ready") {
        this.scheduleSessionStateSave();
        this.emit(FISHYMAP_EVENTS.ready, detail);
        return;
      }
      if (type === "diagnostic") {
        this.emit(FISHYMAP_EVENTS.diagnostic, detail);
      }
    });
  }

  createSessionSnapshot() {
    const state = this.currentState || createEmptySnapshot();
    const fromPatchId =
      this.inputState.filters.fromPatchId ?? state.filters?.fromPatchId ?? null;
    const toPatchId =
      this.inputState.filters.toPatchId ?? state.filters?.toPatchId ?? null;
    const hasExplicitLayerVisibility = Array.isArray(this.inputState.filters.layerIdsVisible);
    const hasExplicitLayerOrder = Array.isArray(this.inputState.filters.layerIdsOrdered);
    const hasExplicitLayerOpacities =
      isPlainObject(this.inputState.filters.layerOpacities) &&
      Object.keys(this.inputState.filters.layerOpacities).length > 0;
    const hasExplicitLayerClipMasks =
      isPlainObject(this.inputState.filters.layerClipMasks) &&
      Object.keys(this.inputState.filters.layerClipMasks).length > 0;
    return {
      version: FISHYMAP_CONTRACT_VERSION,
      savedAt: new Date().toISOString(),
      view: state.view,
      selection: {
        fishId: state.selection?.fishId ?? state.filters?.fishIds?.[0] ?? null,
        zoneRgb: zoneRgbFromLayerSamples(state.selection?.layerSamples),
        worldX: state.selection?.worldX ?? null,
        worldZ: state.selection?.worldZ ?? null,
      },
      filters: {
        fishIds: this.inputState.filters.fishIds,
        zoneRgbs: this.inputState.filters.zoneRgbs,
        searchText: this.inputState.filters.searchText,
        patchId:
          fromPatchId && toPatchId && fromPatchId === toPatchId ? fromPatchId : null,
        fromPatchId,
        toPatchId,
        ...(hasExplicitLayerVisibility
          ? {
              layerIdsVisible: normalizeStringList(this.inputState.filters.layerIdsVisible),
              layerVisibilityExplicit: true,
            }
          : {}),
        ...(hasExplicitLayerOrder
          ? {
              layerIdsOrdered: normalizeStringList(this.inputState.filters.layerIdsOrdered),
            }
          : {}),
        ...(hasExplicitLayerOpacities
          ? {
              layerOpacities: normalizeLayerOpacityMap(this.inputState.filters.layerOpacities),
            }
          : {}),
        ...(hasExplicitLayerClipMasks
          ? {
              layerClipMasks: normalizeLayerClipMaskMap(this.inputState.filters.layerClipMasks),
            }
          : {}),
      },
      ui: {
        diagnosticsOpen: this.inputState.ui.diagnosticsOpen,
        legendOpen: this.inputState.ui.legendOpen,
        leftPanelOpen: this.inputState.ui.leftPanelOpen,
        showPoints: this.inputState.ui.showPoints,
        showPointIcons: this.inputState.ui.showPointIcons,
        pointIconScale: this.inputState.ui.pointIconScale,
      },
    };
  }

  createPrefsSnapshot() {
    const state = this.currentState || createEmptySnapshot();
    const fromPatchId =
      this.inputState.filters.fromPatchId ?? state.filters?.fromPatchId ?? null;
    const toPatchId =
      this.inputState.filters.toPatchId ?? state.filters?.toPatchId ?? null;
    const hasExplicitLayerVisibility = Array.isArray(this.inputState.filters.layerIdsVisible);
    const hasExplicitLayerOrder = Array.isArray(this.inputState.filters.layerIdsOrdered);
    const hasExplicitLayerOpacities =
      isPlainObject(this.inputState.filters.layerOpacities) &&
      Object.keys(this.inputState.filters.layerOpacities).length > 0;
    const hasExplicitLayerClipMasks =
      isPlainObject(this.inputState.filters.layerClipMasks) &&
      Object.keys(this.inputState.filters.layerClipMasks).length > 0;
    return {
      version: FISHYMAP_CONTRACT_VERSION,
      filters: {
        patchId:
          fromPatchId && toPatchId && fromPatchId === toPatchId ? fromPatchId : null,
        fromPatchId,
        toPatchId,
        ...(hasExplicitLayerVisibility
          ? {
              layerIdsVisible: normalizeStringList(this.inputState.filters.layerIdsVisible),
              layerVisibilityExplicit: true,
            }
          : {}),
        ...(hasExplicitLayerOrder
          ? {
              layerIdsOrdered: normalizeStringList(this.inputState.filters.layerIdsOrdered),
            }
          : {}),
        ...(hasExplicitLayerOpacities
          ? {
              layerOpacities: normalizeLayerOpacityMap(this.inputState.filters.layerOpacities),
            }
          : {}),
        ...(hasExplicitLayerClipMasks
          ? {
              layerClipMasks: normalizeLayerClipMaskMap(this.inputState.filters.layerClipMasks),
            }
          : {}),
      },
      ui: {
        legendOpen: this.inputState.ui.legendOpen,
        leftPanelOpen: this.inputState.ui.leftPanelOpen,
        showPoints: this.inputState.ui.showPoints,
        showPointIcons: this.inputState.ui.showPointIcons,
        pointIconScale: this.inputState.ui.pointIconScale,
      },
    };
  }

  scheduleSessionStateSave() {
    if (this.flushSessionTimer) {
      return;
    }
    this.flushSessionTimer = globalThis.setTimeout(() => {
      this.flushSessionTimer = 0;
      this.flushSessionStateSave();
    }, this.sessionSaveDebounceMs);
  }

  flushSessionStateSave() {
    if (this.flushSessionTimer) {
      globalThis.clearTimeout(this.flushSessionTimer);
      this.flushSessionTimer = 0;
    }
    try {
      globalThis.sessionStorage?.setItem(
        FISHYMAP_STORAGE_KEYS.session,
        JSON.stringify(this.createSessionSnapshot()),
      );
    } catch (_) {}
  }

  saveLocalPrefsNow() {
    try {
      globalThis.localStorage?.setItem(
        FISHYMAP_STORAGE_KEYS.prefs,
        JSON.stringify(this.createPrefsSnapshot()),
      );
    } catch (_) {}
  }
}

export function createFishyMapBridge() {
  return new FishyMapBridgeImpl();
}

const FishyMapBridge = createFishyMapBridge();

if (typeof window !== "undefined") {
  window.FishyMapBridge = FishyMapBridge;
}

export default FishyMapBridge;
