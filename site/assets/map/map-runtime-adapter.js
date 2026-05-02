import {
  FISHYMAP_CONTRACT_VERSION,
  FISHYMAP_POINT_ICON_SCALE_DEFAULT,
  createEmptySnapshot,
} from "./map-host.js";
import {
  DEFAULT_ENABLED_LAYER_IDS,
} from "./map-signal-contract.js";
import { buildLayerSearchEffects } from "./map-layer-search-effects.js";
import { resolveSearchExpression } from "./map-search-contract.js";
import { resolveSearchProjection } from "./map-search-projection.js";

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function finiteActionCoordinate(value) {
  if (value == null || (typeof value === "string" && !value.trim())) {
    return null;
  }
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function normalizeActionHistoryBehavior(value) {
  const normalized = String(value ?? "").trim().toLowerCase();
  return normalized === "navigate" ? "navigate" : "append";
}

function normalizeStringList(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  const next = [];
  const seen = new Set();
  for (const value of values) {
    const normalized = String(value ?? "").trim();
    if (!normalized || seen.has(normalized)) {
      continue;
    }
    seen.add(normalized);
    next.push(normalized);
  }
  return next;
}

function normalizeLayerStringListMap(values) {
  if (!isPlainObject(values)) {
    return {};
  }
  const next = {};
  for (const [layerIdRaw, bindingIdsRaw] of Object.entries(values)) {
    const layerId = String(layerIdRaw ?? "").trim();
    if (!layerId) {
      continue;
    }
    const bindingIds = normalizeStringList(bindingIdsRaw);
    if (!bindingIds.length) {
      continue;
    }
    next[layerId] = bindingIds;
  }
  return next;
}

function normalizeIntegerList(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  const next = [];
  const seen = new Set();
  for (const value of values) {
    const normalized = Number.parseInt(value, 10);
    if (!Number.isInteger(normalized) || seen.has(normalized)) {
      continue;
    }
    seen.add(normalized);
    next.push(normalized);
  }
  return next;
}

function normalizeBookmarkEntries(bookmarks) {
  if (!Array.isArray(bookmarks)) {
    return [];
  }
  return bookmarks.flatMap((bookmark) => {
    if (!isPlainObject(bookmark)) {
      return [];
    }
    const id = String(bookmark.id ?? "").trim();
    const worldX = Number(bookmark.worldX);
    const worldZ = Number(bookmark.worldZ);
    if (!id || !Number.isFinite(worldX) || !Number.isFinite(worldZ)) {
      return [];
    }
    const normalized = { id, worldX, worldZ };
    if (typeof bookmark.label === "string" && bookmark.label.trim()) {
      normalized.label = bookmark.label.trim();
    }
    if (typeof bookmark.pointLabel === "string" && bookmark.pointLabel.trim()) {
      normalized.pointLabel = bookmark.pointLabel.trim();
    }
    if (Array.isArray(bookmark.layerSamples)) {
      const layerSamples = bookmark.layerSamples
        .filter((sample) => isPlainObject(sample))
        .map((sample) => cloneJson(sample));
      if (layerSamples.length) {
        normalized.layerSamples = layerSamples;
      }
    }
    const zoneRgb = Number.parseInt(bookmark.zoneRgb, 10);
    if (Number.isInteger(zoneRgb)) {
      normalized.zoneRgb = zoneRgb;
    }
    if (typeof bookmark.createdAt === "string" && bookmark.createdAt.trim()) {
      normalized.createdAt = bookmark.createdAt.trim();
    }
    return [normalized];
  });
}

function normalizeBookmarkSelectedIds(bookmarks, selectedIds) {
  const allowedIds = new Set(normalizeBookmarkEntries(bookmarks).map((bookmark) => bookmark.id));
  return normalizeStringList(selectedIds).filter((bookmarkId) => allowedIds.has(bookmarkId));
}

function normalizeRecordObject(value) {
  return isPlainObject(value) ? cloneJson(value) : {};
}

function hasActiveSearchExpression(expression) {
  if (!isPlainObject(expression)) {
    return false;
  }
  if (expression.type === "term") {
    return true;
  }
  return Array.isArray(expression.children) && expression.children.length > 0;
}

function normalizeBridgedFilters(signals) {
  const bridged = isPlainObject(signals?._map_bridged?.filters) ? signals._map_bridged.filters : {};
  const search = isPlainObject(signals?._map_ui?.search) ? signals._map_ui.search : {};
  const searchProjection = resolveSearchProjection(signals);
  const searchExpression = resolveSearchExpression(search.expression, search.selectedTerms);
  return {
    fishIds: cloneJson(searchProjection.fishIds),
    zoneRgbs: cloneJson(searchProjection.zoneRgbs),
    semanticFieldIdsByLayer: cloneJson(searchProjection.semanticFieldIdsByLayer),
    fishFilterTerms: cloneJson(searchProjection.fishFilterTerms),
    searchExpression: cloneJson(searchExpression),
    patchId: searchProjection.patchId,
    fromPatchId: searchProjection.fromPatchId,
    toPatchId: searchProjection.toPatchId,
    layerIdsVisible: normalizeStringList(bridged.layerIdsVisible).length
      ? normalizeStringList(bridged.layerIdsVisible)
      : cloneJson(DEFAULT_ENABLED_LAYER_IDS),
    layerIdsOrdered: normalizeStringList(bridged.layerIdsOrdered),
    layerFilterBindingIdsDisabledByLayer: normalizeLayerStringListMap(
      bridged.layerFilterBindingIdsDisabledByLayer,
    ),
    layerOpacities: normalizeRecordObject(bridged.layerOpacities),
    layerClipMasks: normalizeRecordObject(bridged.layerClipMasks),
    layerWaypointConnectionsVisible: normalizeRecordObject(
      bridged.layerWaypointConnectionsVisible,
    ),
    layerWaypointLabelsVisible: normalizeRecordObject(bridged.layerWaypointLabelsVisible),
    layerPointIconsVisible: normalizeRecordObject(bridged.layerPointIconsVisible),
    layerPointIconScales: normalizeRecordObject(bridged.layerPointIconScales),
  };
}

function normalizeBridgedUi(signals) {
  const bridged = isPlainObject(signals?._map_bridged?.ui) ? signals._map_bridged.ui : {};
  return {
    diagnosticsOpen: bridged.diagnosticsOpen === true,
    showPoints: bridged.showPoints !== false,
    showPointIcons: bridged.showPointIcons !== false,
    viewMode: bridged.viewMode === "3d" ? "3d" : "2d",
    pointIconScale: Number.isFinite(bridged.pointIconScale)
      ? Number(bridged.pointIconScale)
      : FISHYMAP_POINT_ICON_SCALE_DEFAULT,
  };
}

export function normalizeMapActionState(raw) {
  const source = isPlainObject(raw) ? raw : {};
  const focusWorldPoint = isPlainObject(source.focusWorldPoint) ? source.focusWorldPoint : null;
  const focusWorldPointWorldX = finiteActionCoordinate(focusWorldPoint?.worldX);
  const focusWorldPointWorldZ = finiteActionCoordinate(focusWorldPoint?.worldZ);
  return {
    resetViewToken: Number.isFinite(Number(source.resetViewToken))
      ? Number(source.resetViewToken)
      : 0,
    resetUiToken: Number.isFinite(Number(source.resetUiToken))
      ? Number(source.resetUiToken)
      : 0,
    saveMapPresetToken: Number.isFinite(Number(source.saveMapPresetToken))
      ? Number(source.saveMapPresetToken)
      : 0,
    discardMapPresetToken: Number.isFinite(Number(source.discardMapPresetToken))
      ? Number(source.discardMapPresetToken)
      : 0,
    focusWorldPointToken: Number.isFinite(Number(source.focusWorldPointToken))
      ? Number(source.focusWorldPointToken)
      : 0,
    focusWorldPoint:
      focusWorldPoint &&
      focusWorldPointWorldX != null &&
      focusWorldPointWorldZ != null
        ? {
            worldX: focusWorldPointWorldX,
            worldZ: focusWorldPointWorldZ,
            ...(typeof focusWorldPoint.elementKind === "string" && focusWorldPoint.elementKind.trim()
              ? { elementKind: focusWorldPoint.elementKind.trim() }
              : {}),
            ...(typeof focusWorldPoint.pointKind === "string" && focusWorldPoint.pointKind.trim()
              ? { pointKind: focusWorldPoint.pointKind.trim() }
              : {}),
            ...(typeof focusWorldPoint.pointLabel === "string" && focusWorldPoint.pointLabel.trim()
              ? { pointLabel: focusWorldPoint.pointLabel.trim() }
              : {}),
            historyBehavior: normalizeActionHistoryBehavior(focusWorldPoint.historyBehavior),
          }
        : null,
  };
}

export function buildBridgeInputPatchFromSignals(signals) {
  const filters = normalizeBridgedFilters(signals);
  const ui = normalizeBridgedUi(signals);
  const hasSearchExpression = hasActiveSearchExpression(filters.searchExpression);
  const projectedSemanticFieldIdsByLayer = cloneJson(filters.semanticFieldIdsByLayer);
  if (filters.zoneRgbs.length) {
    projectedSemanticFieldIdsByLayer.zone_mask = cloneJson(filters.zoneRgbs);
  } else {
    delete projectedSemanticFieldIdsByLayer.zone_mask;
  }
  const projectedFilters = {
    ...filters,
    semanticFieldIdsByLayer: projectedSemanticFieldIdsByLayer,
  };
  const outboundFilters = hasSearchExpression
    ? {
        ...filters,
        fishIds: [],
        zoneRgbs: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: [],
        patchId: null,
        fromPatchId: null,
        toPatchId: null,
      }
    : {
        ...projectedFilters,
      };
  const effectiveFilters = {
    ...outboundFilters,
    searchExpression: cloneJson(filters.searchExpression),
  };
  const layerSearchEffects = buildLayerSearchEffects(projectedFilters);
  const bookmarks = normalizeBookmarkEntries(signals?._map_bookmarks?.entries);
  const bookmarkSelectedIds = normalizeBookmarkSelectedIds(
    bookmarks,
    signals?._map_ui?.bookmarks?.selectedIds,
  );
  const sharedFishState = {
    caughtIds: normalizeIntegerList(signals?._shared_fish?.caughtIds),
    favouriteIds: normalizeIntegerList(signals?._shared_fish?.favouriteIds),
  };

  return {
    version: FISHYMAP_CONTRACT_VERSION,
    filters: {
      fishIds: cloneJson(effectiveFilters.fishIds),
      zoneRgbs: cloneJson(effectiveFilters.zoneRgbs),
      semanticFieldIdsByLayer: cloneJson(effectiveFilters.semanticFieldIdsByLayer),
      fishFilterTerms: cloneJson(effectiveFilters.fishFilterTerms),
      searchExpression: cloneJson(effectiveFilters.searchExpression),
      patchId: effectiveFilters.patchId,
      fromPatchId: effectiveFilters.fromPatchId,
      toPatchId: effectiveFilters.toPatchId,
      layerIdsVisible: cloneJson(effectiveFilters.layerIdsVisible),
      layerIdsOrdered: cloneJson(effectiveFilters.layerIdsOrdered),
      layerFilterBindingIdsDisabledByLayer: cloneJson(
        effectiveFilters.layerFilterBindingIdsDisabledByLayer,
      ),
      layerOpacities: cloneJson(effectiveFilters.layerOpacities),
      layerClipMasks: cloneJson(layerSearchEffects.effectiveLayerClipMasks),
      layerWaypointConnectionsVisible: cloneJson(effectiveFilters.layerWaypointConnectionsVisible),
      layerWaypointLabelsVisible: cloneJson(effectiveFilters.layerWaypointLabelsVisible),
      layerPointIconsVisible: cloneJson(effectiveFilters.layerPointIconsVisible),
      layerPointIconScales: cloneJson(effectiveFilters.layerPointIconScales),
    },
    ui: {
      diagnosticsOpen: ui.diagnosticsOpen,
      showPoints: ui.showPoints,
      showPointIcons: ui.showPointIcons,
      viewMode: ui.viewMode,
      pointIconScale: ui.pointIconScale,
      sharedFishState: cloneJson(sharedFishState),
      bookmarkSelectedIds: cloneJson(bookmarkSelectedIds),
      bookmarks: cloneJson(bookmarks),
    },
  };
}

export function buildBridgeCommandPatchFromSignals(signals, previousActionState = {}) {
  const current = normalizeMapActionState(signals?._map_actions);
  const previous = normalizeMapActionState(previousActionState);
  const commands = {};
  if (current.resetViewToken > previous.resetViewToken) {
    commands.resetView = true;
  }
  if (
    current.focusWorldPointToken > previous.focusWorldPointToken &&
    current.focusWorldPoint
  ) {
    commands.selectWorldPoint = cloneJson(current.focusWorldPoint);
  }
  return Object.keys(commands).length ? { version: FISHYMAP_CONTRACT_VERSION, commands } : null;
}

export function projectRuntimeSnapshotToSignals(snapshot) {
  const current = isPlainObject(snapshot) ? snapshot : createEmptySnapshot();
  return {
    _map_runtime: {
      ready: current.ready === true,
      theme: cloneJson(current.theme || {}),
      effectiveFilters: cloneJson(current.effectiveFilters || createEmptySnapshot().effectiveFilters),
      ui: {
        bookmarks: cloneJson(current.ui?.bookmarks || []),
      },
      view: cloneJson(current.view || {}),
      selection: cloneJson(current.selection || {}),
      catalog: cloneJson(current.catalog || {}),
      statuses: cloneJson(current.statuses || {}),
      lastDiagnostic: cloneJson(current.lastDiagnostic || null),
    },
  };
}

export function projectSessionSnapshotToSignals(snapshot) {
  const current = isPlainObject(snapshot) ? snapshot : createEmptySnapshot();
  return {
    _map_session: {
      view: cloneJson(current.view || {}),
      selection: cloneJson(current.selection || {}),
    },
  };
}
