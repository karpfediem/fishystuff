import {
  FISHYMAP_POINT_ICON_SCALE_MAX,
  FISHYMAP_POINT_ICON_SCALE_MIN,
} from "./map-host.js";
import { DEFAULT_ENABLED_LAYER_IDS } from "./map-signal-contract.js";

const FIXED_GROUND_LAYER_IDS = new Set();

function hasOwnKey(object, key) {
  return !!object && Object.prototype.hasOwnProperty.call(object, key);
}

function isPlainObject(value) {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

export function clampPointIconScale(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return FISHYMAP_POINT_ICON_SCALE_MIN;
  }
  return Math.min(FISHYMAP_POINT_ICON_SCALE_MAX, Math.max(FISHYMAP_POINT_ICON_SCALE_MIN, number));
}

export function pointIconScaleValue(scale) {
  return String(Math.round(clampPointIconScale(scale) * 100) / 100);
}

export function pointIconScaleLabel(scale) {
  return `${Math.round(clampPointIconScale(scale) * 100)}%`;
}

export function clampLayerOpacity(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return 1;
  }
  return Math.min(1, Math.max(0, number));
}

export function layerOpacityValue(opacity) {
  return String(Math.round(clampLayerOpacity(opacity) * 100) / 100);
}

export function layerOpacityLabel(opacity) {
  return `${Math.round(clampLayerOpacity(opacity) * 100)}%`;
}

export function isFixedGroundLayer(layerId) {
  return FIXED_GROUND_LAYER_IDS.has(String(layerId || "").trim());
}

export function resolveLayerEntries(stateBundle) {
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

export function resolveVisibleLayerIds(stateBundle) {
  return resolveLayerEntries(stateBundle)
    .filter((layer) => layer.visible)
    .map((layer) => layer.layerId);
}

export function moveLayerIdBefore(entries, draggedLayerId, targetLayerId, position) {
  const movableIds = entries.filter((layer) => !layer.locked).map((layer) => layer.layerId);
  const fromIndex = movableIds.indexOf(draggedLayerId);
  const targetIndex = movableIds.indexOf(targetLayerId);
  if (fromIndex < 0 || targetIndex < 0) {
    return movableIds;
  }
  const [dragged] = movableIds.splice(fromIndex, 1);
  const insertIndex = position === "after"
    ? targetIndex + (fromIndex < targetIndex ? 0 : 1)
    : targetIndex + (fromIndex < targetIndex ? -1 : 0);
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

export function buildLayerOpacityPatch(stateBundle, targetLayerId, opacity) {
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

export function buildLayerClipMaskPatch(stateBundle, targetLayerId, maskLayerId) {
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

export function buildLayerWaypointConnectionsPatch(stateBundle, targetLayerId, visible) {
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

export function buildLayerWaypointLabelsPatch(stateBundle, targetLayerId, visible) {
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

export function buildLayerPointIconsPatch(stateBundle, targetLayerId, visible) {
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

export function buildLayerPointIconScalePatch(stateBundle, targetLayerId, scale) {
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
