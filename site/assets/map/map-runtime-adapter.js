import {
  FISHYMAP_CONTRACT_VERSION,
  FISHYMAP_POINT_ICON_SCALE_MIN,
  createEmptySnapshot,
  resolveEffectiveFishIdsForWasm,
} from "./map-host.js";
import {
  DEFAULT_ENABLED_LAYER_IDS,
} from "./map-signal-contract.js";
import { buildLayerSearchEffects } from "./map-layer-search-effects.js";
import { resolveSearchProjection } from "./map-search-projection.js";

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
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

function normalizeBridgedFilters(signals) {
  const bridged = isPlainObject(signals?._map_bridged?.filters) ? signals._map_bridged.filters : {};
  const searchProjection = resolveSearchProjection(signals);
  return {
    fishIds: cloneJson(searchProjection.fishIds),
    zoneRgbs: cloneJson(searchProjection.zoneRgbs),
    semanticFieldIdsByLayer: cloneJson(searchProjection.semanticFieldIdsByLayer),
    fishFilterTerms: cloneJson(searchProjection.fishFilterTerms),
    patchId: bridged.patchId ?? null,
    fromPatchId: bridged.fromPatchId ?? null,
    toPatchId: bridged.toPatchId ?? null,
    layerIdsVisible: normalizeStringList(bridged.layerIdsVisible).length
      ? normalizeStringList(bridged.layerIdsVisible)
      : cloneJson(DEFAULT_ENABLED_LAYER_IDS),
    layerIdsOrdered: normalizeStringList(bridged.layerIdsOrdered),
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
      : FISHYMAP_POINT_ICON_SCALE_MIN,
  };
}

export function normalizeMapActionState(raw) {
  const source = isPlainObject(raw) ? raw : {};
  const focusWorldPoint = isPlainObject(source.focusWorldPoint) ? source.focusWorldPoint : null;
  const focusWorldPointWorldX = Number(focusWorldPoint?.worldX);
  const focusWorldPointWorldZ = Number(focusWorldPoint?.worldZ);
  return {
    resetViewToken: Number.isFinite(Number(source.resetViewToken))
      ? Number(source.resetViewToken)
      : 0,
    resetUiToken: Number.isFinite(Number(source.resetUiToken))
      ? Number(source.resetUiToken)
      : 0,
    focusWorldPointToken: Number.isFinite(Number(source.focusWorldPointToken))
      ? Number(source.focusWorldPointToken)
      : 0,
    focusWorldPoint:
      focusWorldPoint &&
      Number.isFinite(focusWorldPointWorldX) &&
      Number.isFinite(focusWorldPointWorldZ)
        ? {
            worldX: focusWorldPointWorldX,
            worldZ: focusWorldPointWorldZ,
            ...(typeof focusWorldPoint.pointKind === "string" && focusWorldPoint.pointKind.trim()
              ? { pointKind: focusWorldPoint.pointKind.trim() }
              : {}),
            ...(typeof focusWorldPoint.pointLabel === "string" && focusWorldPoint.pointLabel.trim()
              ? { pointLabel: focusWorldPoint.pointLabel.trim() }
              : {}),
          }
        : null,
  };
}

export function buildBridgeInputPatchFromSignals(signals, options = {}) {
  const filters = normalizeBridgedFilters(signals);
  const ui = normalizeBridgedUi(signals);
  const layerSearchEffects = buildLayerSearchEffects(filters);
  const bookmarks = normalizeBookmarkEntries(signals?._map_bookmarks?.entries);
  const bookmarkSelectedIds = normalizeBookmarkSelectedIds(
    bookmarks,
    signals?._map_ui?.bookmarks?.selectedIds,
  );
  const sharedFishState = {
    caughtIds: normalizeIntegerList(signals?._shared_fish?.caughtIds),
    favouriteIds: normalizeIntegerList(signals?._shared_fish?.favouriteIds),
  };
  const currentState = isPlainObject(options.currentState) ? options.currentState : createEmptySnapshot();
  const effectiveFishIds = resolveEffectiveFishIdsForWasm(
    {
      filters: {
        fishIds: filters.fishIds,
        fishFilterTerms: filters.fishFilterTerms,
      },
      ui: {
        sharedFishState,
      },
    },
    currentState,
  );

  return {
    version: FISHYMAP_CONTRACT_VERSION,
    filters: {
      fishIds: cloneJson(effectiveFishIds),
      zoneRgbs: cloneJson(filters.zoneRgbs),
      semanticFieldIdsByLayer: cloneJson(filters.semanticFieldIdsByLayer),
      fishFilterTerms: cloneJson(filters.fishFilterTerms),
      patchId: filters.patchId,
      fromPatchId: filters.fromPatchId,
      toPatchId: filters.toPatchId,
      layerIdsVisible: cloneJson(filters.layerIdsVisible),
      layerIdsOrdered: cloneJson(filters.layerIdsOrdered),
      layerOpacities: cloneJson(filters.layerOpacities),
      layerClipMasks: cloneJson(layerSearchEffects.effectiveLayerClipMasks),
      zoneMembershipLayerIds: cloneJson(layerSearchEffects.zoneMembershipLayerIds),
      layerWaypointConnectionsVisible: cloneJson(filters.layerWaypointConnectionsVisible),
      layerWaypointLabelsVisible: cloneJson(filters.layerWaypointLabelsVisible),
      layerPointIconsVisible: cloneJson(filters.layerPointIconsVisible),
      layerPointIconScales: cloneJson(filters.layerPointIconScales),
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
