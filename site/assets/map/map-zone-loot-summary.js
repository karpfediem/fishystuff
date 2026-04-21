import { resolveApiBaseUrl } from "./map-host.js";
import { mapText } from "./map-i18n.js";
import { zoneRgbFromSample } from "./map-overview-facts.js";

const DEFAULT_ZONE_LOOT_SUMMARY_PATH = "/api/v1/zone_loot_summary";

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function trimString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function rgbTripletString(rgbU32) {
  return [(rgbU32 >> 16) & 0xff, (rgbU32 >> 8) & 0xff, rgbU32 & 0xff].join(",");
}

function currentUserOverlaySignals(explicitOverlaySignals) {
  if (isPlainObject(explicitOverlaySignals)) {
    return cloneJson(explicitOverlaySignals);
  }
  const helper = globalThis.window?.__fishystuffUserOverlays;
  if (helper && typeof helper.overlaySignals === "function") {
    return helper.overlaySignals();
  }
  return { zones: {} };
}

export function zoneRgbFromSelection(selection) {
  const zoneRgb = Number.parseInt(selection?.zoneStats?.zoneRgb, 10);
  if (Number.isInteger(zoneRgb) && zoneRgb >= 0) {
    return zoneRgb;
  }
  if (Array.isArray(selection?.layerSamples)) {
    const zoneSample = selection.layerSamples.find(
      (sample) => trimString(sample?.layerId) === "zone_mask",
    );
    return Number.isInteger(zoneRgbFromSample(zoneSample)) ? zoneRgbFromSample(zoneSample) : null;
  }
  return null;
}

export function normalizeZoneLootSummary(raw) {
  const source = isPlainObject(raw) ? raw : {};
  return {
    available: source.available === true,
    zoneName: trimString(source.zoneName),
    note: trimString(source.note),
    profileLabel: trimString(source.profileLabel),
    groups: Array.isArray(source.groups)
      ? source.groups
          .filter((group) => isPlainObject(group))
          .map((group) => ({
            slotIdx: Number.parseInt(group.slotIdx, 10) || 0,
            label: trimString(group.label),
            fillColor: trimString(group.fillColor),
            strokeColor: trimString(group.strokeColor),
            textColor: trimString(group.textColor),
            dropRateText: trimString(group.dropRateText),
            dropRateSourceKind: trimString(group.dropRateSourceKind),
            dropRateTooltip: trimString(group.dropRateTooltip),
          }))
      : [],
    speciesRows: Array.isArray(source.speciesRows)
      ? source.speciesRows
          .filter((row) => isPlainObject(row))
          .map((row) => ({
            slotIdx: Number.parseInt(row.slotIdx, 10) || 0,
            groupLabel: trimString(row.groupLabel),
            label: trimString(row.label),
            iconUrl: trimString(row.iconUrl),
            iconGradeTone: trimString(row.iconGradeTone),
            fillColor: trimString(row.fillColor),
            strokeColor: trimString(row.strokeColor),
            textColor: trimString(row.textColor),
            dropRateText: trimString(row.dropRateText),
            dropRateSourceKind: trimString(row.dropRateSourceKind),
            dropRateTooltip: trimString(row.dropRateTooltip),
            presenceText: trimString(row.presenceText),
            presenceSourceKind: trimString(row.presenceSourceKind),
            presenceTooltip: trimString(row.presenceTooltip),
          }))
      : [],
  };
}

export async function loadZoneLootSummary(
  zoneRgb,
  {
    fetchImpl = globalThis.fetch,
    locationLike = globalThis.window?.location,
    overlaySignals = null,
  } = {},
) {
  const normalizedZoneRgb = Number.parseInt(zoneRgb, 10);
  if (!Number.isInteger(normalizedZoneRgb) || normalizedZoneRgb < 0) {
    throw new Error(mapText("zone_loot.error.invalid_zone_rgb"));
  }
  const baseUrl = resolveApiBaseUrl(locationLike);
  const response = await fetchImpl(`${baseUrl}${DEFAULT_ZONE_LOOT_SUMMARY_PATH}`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify({
      rgb: rgbTripletString(normalizedZoneRgb),
      overlay: currentUserOverlaySignals(overlaySignals),
    }),
  });
  if (!response.ok) {
    if (response.status === 404) {
      throw new Error(mapText("zone_loot.error.missing_endpoint"));
    }
    throw new Error(mapText("zone_loot.error.request_failed", { status: response.status }));
  }
  return normalizeZoneLootSummary(await response.json());
}
