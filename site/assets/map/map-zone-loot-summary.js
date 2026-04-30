import { resolveApiBaseUrl } from "./map-host.js";
import { currentDataLanguage, currentLocale, languageReady, mapText } from "./map-i18n.js";
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

function normalizeCatchMethods(value) {
  if (!Array.isArray(value)) {
    return [];
  }
  const methods = [];
  for (const rawMethod of value) {
    const normalized = trimString(rawMethod).toLowerCase();
    if ((normalized === "rod" || normalized === "harpoon") && !methods.includes(normalized)) {
      methods.push(normalized);
    }
  }
  return methods;
}

function normalizeZoneLootSpeciesRow(row) {
  return {
    slotIdx: Number.parseInt(row?.slotIdx, 10) || 0,
    groupLabel: trimString(row?.groupLabel),
    label: trimString(row?.label),
    iconUrl: trimString(row?.iconUrl),
    iconGradeTone: trimString(row?.iconGradeTone),
    fillColor: trimString(row?.fillColor),
    strokeColor: trimString(row?.strokeColor),
    textColor: trimString(row?.textColor),
    dropRateText: trimString(row?.dropRateText),
    dropRateSourceKind: trimString(row?.dropRateSourceKind),
    dropRateTooltip: trimString(row?.dropRateTooltip),
    rawDropRateText: trimString(row?.rawDropRateText),
    rawDropRateTooltip: trimString(row?.rawDropRateTooltip),
    normalizedDropRateText: trimString(row?.normalizedDropRateText),
    normalizedDropRateTooltip: trimString(row?.normalizedDropRateTooltip),
    presenceText: trimString(row?.presenceText),
    presenceSourceKind: trimString(row?.presenceSourceKind),
    presenceTooltip: trimString(row?.presenceTooltip),
    catchMethods: normalizeCatchMethods(row?.catchMethods),
  };
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

function appendQueryParam(path, key, value) {
  const normalizedValue = trimString(value);
  if (!normalizedValue) {
    return path;
  }
  const separator = path.includes("?") ? "&" : "?";
  return `${path}${separator}${encodeURIComponent(key)}=${encodeURIComponent(normalizedValue)}`;
}

function localizedZoneLootSummaryPath() {
  let path = DEFAULT_ZONE_LOOT_SUMMARY_PATH;
  path = appendQueryParam(path, "lang", currentDataLanguage());
  path = appendQueryParam(path, "locale", currentLocale());
  return path;
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
    dataQualityNote: trimString(source.dataQualityNote),
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
            rawDropRateText: trimString(group.rawDropRateText),
            rawDropRateTooltip: trimString(group.rawDropRateTooltip),
            normalizedDropRateText: trimString(group.normalizedDropRateText),
            normalizedDropRateTooltip: trimString(group.normalizedDropRateTooltip),
            conditionText: trimString(group.conditionText),
            conditionTooltip: trimString(group.conditionTooltip),
            catchMethods: normalizeCatchMethods(group.catchMethods),
            conditionOptions: Array.isArray(group.conditionOptions)
              ? group.conditionOptions
                  .filter((option) => isPlainObject(option))
                  .map((option) => ({
                    conditionText: trimString(option.conditionText),
                    conditionTooltip: trimString(option.conditionTooltip),
                    dropRateText: trimString(option.dropRateText),
                    dropRateSourceKind: trimString(option.dropRateSourceKind),
                    dropRateTooltip: trimString(option.dropRateTooltip),
                    presenceText: trimString(option.presenceText),
                    presenceSourceKind: trimString(option.presenceSourceKind),
                    presenceTooltip: trimString(option.presenceTooltip),
                    rawDropRateText: trimString(option.rawDropRateText),
                    rawDropRateTooltip: trimString(option.rawDropRateTooltip),
                    normalizedDropRateText: trimString(option.normalizedDropRateText),
                    normalizedDropRateTooltip: trimString(option.normalizedDropRateTooltip),
                    active: option.active === true,
                    speciesRows: Array.isArray(option.speciesRows)
                      ? option.speciesRows
                          .filter((row) => isPlainObject(row))
                          .map((row) => normalizeZoneLootSpeciesRow(row))
                      : [],
                  }))
              : [],
          }))
      : [],
    speciesRows: Array.isArray(source.speciesRows)
      ? source.speciesRows
          .filter((row) => isPlainObject(row))
          .map((row) => normalizeZoneLootSpeciesRow(row))
      : [],
  };
}

export async function loadZoneLootSummary(
  zoneRgb,
  {
    fetchImpl = globalThis.fetch,
    locationLike = globalThis.window?.location,
    overlaySignals = null,
    normalizeRates = null,
  } = {},
) {
  const normalizedZoneRgb = Number.parseInt(zoneRgb, 10);
  if (!Number.isInteger(normalizedZoneRgb) || normalizedZoneRgb < 0) {
    throw new Error(mapText("zone_loot.error.invalid_zone_rgb"));
  }
  await languageReady();
  const baseUrl = resolveApiBaseUrl(locationLike);
  const requestBody = {
    rgb: rgbTripletString(normalizedZoneRgb),
    overlay: currentUserOverlaySignals(overlaySignals),
  };
  if (typeof normalizeRates === "boolean") {
    requestBody.showNormalizedSelectRates = normalizeRates;
  }
  const response = await fetchImpl(`${baseUrl}${localizedZoneLootSummaryPath()}`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify(requestBody),
  });
  if (!response.ok) {
    if (response.status === 404) {
      throw new Error(mapText("zone_loot.error.missing_endpoint"));
    }
    throw new Error(mapText("zone_loot.error.request_failed", { status: response.status }));
  }
  return normalizeZoneLootSummary(await response.json());
}
