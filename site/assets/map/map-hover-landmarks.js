import { mapText } from "./map-i18n.js";
import { buildPointSampleRows } from "./map-hover-facts.js";

const DEFAULT_LANDMARK_ICON = "information-circle";
const TRADE_NPC_TARGET_KEY = "trade_npc";
const HOTSPOT_TARGET_KEY = "hotspot";

const LANDMARK_HOVER_TARGET_PRESENTATION = Object.freeze({
  bookmark: {
    icon: "bookmark",
    labelKey: "info.status.bookmark",
    swatchRgb: "239 92 31",
  },
  [TRADE_NPC_TARGET_KEY]: {
    icon: "trade-origin",
    label: "NPC",
    swatchRgb: "255 196 66",
  },
  [HOTSPOT_TARGET_KEY]: {
    icon: "map-pin",
    label: "Hotspot",
    swatchRgb: "255 179 56",
  },
  waypoint: {
    icon: "map-pin",
    labelKey: "info.status.waypoint",
    swatchRgb: "244 240 232",
  },
});

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function trimString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function coordinateKey(value) {
  const number = Number(value);
  return Number.isFinite(number) ? number.toFixed(3) : "";
}

function targetWorldX(target) {
  return target?.worldX ?? target?.world_x;
}

function targetWorldZ(target) {
  return target?.worldZ ?? target?.world_z;
}

function landmarkHoverPresentation(targetKey) {
  return LANDMARK_HOVER_TARGET_PRESENTATION[trimString(targetKey)] || null;
}

function buildTargetLandmarkRows(hover) {
  const layerSamples = Array.isArray(hover?.layerSamples) ? hover.layerSamples : [];
  const seen = new Set();
  return layerSamples.flatMap((sample) => {
    const layerId = trimString(sample?.layerId);
    const targets = Array.isArray(sample?.targets) ? sample.targets : [];
    return targets.flatMap((target, targetIndex) => {
      if (!isPlainObject(target)) {
        return [];
      }
      const targetKey = trimString(target.key);
      const presentation = landmarkHoverPresentation(targetKey);
      const value = trimString(target.label);
      if (!presentation || !value) {
        return [];
      }
      const worldX = targetWorldX(target);
      const worldZ = targetWorldZ(target);
      const rowKey = [
        "landmark-hover",
        layerId,
        targetKey,
        coordinateKey(worldX),
        coordinateKey(worldZ),
        value,
      ].join(":");
      if (seen.has(rowKey)) {
        return [];
      }
      seen.add(rowKey);
      return [
        {
          kind: "landmark-hover",
          key: rowKey || `landmark-hover:${targetIndex}`,
          layerId,
          targetKey,
          label: presentation.label || mapText(presentation.labelKey),
          value,
          icon: presentation.icon || DEFAULT_LANDMARK_ICON,
          ...(presentation.swatchRgb ? { swatchRgb: presentation.swatchRgb } : {}),
          worldX,
          worldZ,
        },
      ];
    });
  });
}

export function patchTouchesLandmarkHoverSignals(patch) {
  if (!isPlainObject(patch)) {
    return false;
  }
  return Boolean(
    patch._map_runtime?.catalog?.fish != null ||
      patch._map_ui?.layers?.sampleHoverVisibleByLayer != null,
  );
}

export function buildLandmarkHoverRows({
  hover = null,
  stateBundle = null,
  pointSamplesEnabled = true,
  zoneCatalog = [],
} = {}) {
  const pointRows =
    pointSamplesEnabled === false
      ? []
      : buildPointSampleRows({ source: hover, stateBundle, zoneCatalog });
  return [...pointRows, ...buildTargetLandmarkRows(hover)];
}
