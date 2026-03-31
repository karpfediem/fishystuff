import { applyStatePatch, resolveEffectiveFishIdsForWasm } from "./map-host.js";
import {
  DEFAULT_MAP_CONTROL_SIGNAL_STATE,
  MAP_BRIDGE_SHARED_SIGNAL_WHITELIST,
} from "./map-signal-contract.js";

function cloneJsonValue(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function normalizeSharedFishIds(values) {
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

function normalizeSelectedBookmarkIds(bookmarks, selectedIds) {
  const bookmarkIds = new Set(
    Array.isArray(bookmarks)
      ? bookmarks
          .map((bookmark) => String(bookmark?.id ?? "").trim())
          .filter(Boolean)
      : [],
  );
  if (!Array.isArray(selectedIds) || !bookmarkIds.size) {
    return [];
  }
  const next = [];
  const seen = new Set();
  for (const value of selectedIds) {
    const normalized = String(value ?? "").trim();
    if (!normalized || seen.has(normalized) || !bookmarkIds.has(normalized)) {
      continue;
    }
    seen.add(normalized);
    next.push(normalized);
  }
  return next;
}

function projectBridgeBookmarkEntries(bookmarks) {
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

export const MAP_CONTROL_BRIDGE_RELEVANT_PATCH_PATHS = Object.freeze([
  ["filters", "fishIds"],
  ["filters", "zoneRgbs"],
  ["filters", "semanticFieldIdsByLayer"],
  ["filters", "fishFilterTerms"],
  ["filters", "patchId"],
  ["filters", "layerIdsVisible"],
  ["filters", "layerIdsOrdered"],
  ["filters", "layerOpacities"],
  ["filters", "layerClipMasks"],
  ["filters", "layerWaypointConnectionsVisible"],
  ["filters", "layerWaypointLabelsVisible"],
  ["filters", "layerPointIconsVisible"],
  ["filters", "layerPointIconScales"],
]);

export function projectBridgeSharedInputState(controlState, options = {}) {
  const current = applyStatePatch(
    DEFAULT_MAP_CONTROL_SIGNAL_STATE,
    controlState && typeof controlState === "object" ? controlState : {},
  );
  const sharedFishState = options.sharedFishState || {};
  const currentState = options.currentState && typeof options.currentState === "object"
    ? options.currentState
    : {};
  const bookmarks = Array.isArray(options.bookmarks) ? options.bookmarks : [];
  const bookmarkSelectedIds = normalizeSelectedBookmarkIds(
    bookmarks,
    Array.isArray(options.bookmarkSelectedIds) ? options.bookmarkSelectedIds : [],
  );
  const effectiveFishIds = resolveEffectiveFishIdsForWasm(
    {
      filters: {
        fishIds: current.filters?.fishIds || [],
        fishFilterTerms: current.filters?.fishFilterTerms || [],
      },
      ui: {
        sharedFishState: {
          caughtIds: normalizeSharedFishIds(sharedFishState?.caughtIds),
          favouriteIds: normalizeSharedFishIds(sharedFishState?.favouriteIds),
        },
      },
    },
    currentState,
  );
  return {
    filters: {
      ...(MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.bridged.filters.includes("fishIds")
        ? { fishIds: cloneJsonValue(effectiveFishIds) }
        : {}),
      ...(MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.bridged.filters.includes("zoneRgbs")
        ? { zoneRgbs: cloneJsonValue(current.filters?.zoneRgbs || []) }
        : {}),
      ...(MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.bridged.filters.includes("semanticFieldIdsByLayer")
        ? {
            semanticFieldIdsByLayer: cloneJsonValue(
              current.filters?.semanticFieldIdsByLayer || {},
            ),
          }
        : {}),
      ...(MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.bridged.filters.includes("patchId")
        ? { patchId: current.filters?.patchId ?? null }
        : {}),
      ...(MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.bridged.filters.includes("layerIdsVisible")
        && Array.isArray(current.filters?.layerIdsVisible)
        ? { layerIdsVisible: cloneJsonValue(current.filters.layerIdsVisible) }
        : {}),
      ...(MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.bridged.filters.includes("layerIdsOrdered")
        && Array.isArray(current.filters?.layerIdsOrdered)
        ? { layerIdsOrdered: cloneJsonValue(current.filters.layerIdsOrdered) }
        : {}),
      ...(MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.bridged.filters.includes("layerOpacities")
        && current.filters?.layerOpacities
        && typeof current.filters.layerOpacities === "object"
        ? { layerOpacities: cloneJsonValue(current.filters.layerOpacities) }
        : {}),
      ...(MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.bridged.filters.includes("layerClipMasks")
        && current.filters?.layerClipMasks
        && typeof current.filters.layerClipMasks === "object"
        ? { layerClipMasks: cloneJsonValue(current.filters.layerClipMasks) }
        : {}),
      ...(MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.bridged.filters.includes(
        "layerWaypointConnectionsVisible",
      )
        && current.filters?.layerWaypointConnectionsVisible
        && typeof current.filters.layerWaypointConnectionsVisible === "object"
        ? {
            layerWaypointConnectionsVisible: cloneJsonValue(
              current.filters.layerWaypointConnectionsVisible,
            ),
          }
        : {}),
      ...(MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.bridged.filters.includes("layerWaypointLabelsVisible")
        && current.filters?.layerWaypointLabelsVisible
        && typeof current.filters.layerWaypointLabelsVisible === "object"
        ? {
            layerWaypointLabelsVisible: cloneJsonValue(
              current.filters.layerWaypointLabelsVisible,
            ),
          }
        : {}),
      ...(MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.bridged.filters.includes("layerPointIconsVisible")
        && current.filters?.layerPointIconsVisible
        && typeof current.filters.layerPointIconsVisible === "object"
        ? {
            layerPointIconsVisible: cloneJsonValue(current.filters.layerPointIconsVisible),
          }
        : {}),
      ...(MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.bridged.filters.includes("layerPointIconScales")
        && current.filters?.layerPointIconScales
        && typeof current.filters.layerPointIconScales === "object"
        ? {
            layerPointIconScales: cloneJsonValue(current.filters.layerPointIconScales),
          }
        : {}),
    },
    ui: {
      ...(MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.bridged.ui.includes("bookmarkSelectedIds")
        ? { bookmarkSelectedIds: cloneJsonValue(bookmarkSelectedIds) }
        : {}),
      ...(MAP_BRIDGE_SHARED_SIGNAL_WHITELIST.bridged.ui.includes("bookmarks")
        ? { bookmarks: cloneJsonValue(projectBridgeBookmarkEntries(bookmarks)) }
        : {}),
    },
  };
}
