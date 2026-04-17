import { resolveApiBaseUrl } from "./map-host.js";

const DEFAULT_ZONE_CATALOG_PATH = "/api/v1/zones";

function hasOwnKey(object, key) {
  return !!object && Object.prototype.hasOwnProperty.call(object, key);
}

function rgbTripletToU32(r, g, b) {
  return (((r & 0xff) << 16) | ((g & 0xff) << 8) | (b & 0xff)) >>> 0;
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

function normalizeSearchText(value) {
  return String(value || "")
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
}

function compactSearchText(value) {
  return normalizeSearchText(value).replace(/[^a-z0-9]/g, "");
}

function scoreTermMatch(haystack, term, baseScore) {
  const haystackCompact = compactSearchText(haystack);
  const termCompact = compactSearchText(term);
  if (!termCompact || !haystackCompact) {
    return Number.NEGATIVE_INFINITY;
  }

  const directIndex = haystackCompact.indexOf(termCompact);
  if (directIndex >= 0) {
    return baseScore - Math.min(directIndex, baseScore - 1);
  }

  if (termCompact.length <= 1) {
    return Number.NEGATIVE_INFINITY;
  }

  let previousIndex = -1;
  let gapPenalty = 0;
  let firstIndex = -1;
  for (const char of termCompact) {
    const foundIndex = haystackCompact.indexOf(char, previousIndex + 1);
    if (foundIndex < 0) {
      return Number.NEGATIVE_INFINITY;
    }
    if (firstIndex < 0) {
      firstIndex = foundIndex;
    } else {
      gapPenalty += Math.max(0, foundIndex - previousIndex - 1);
    }
    previousIndex = foundIndex;
  }
  const spanPenalty = Math.max(0, previousIndex - firstIndex);
  const scorePenalty = Math.min(spanPenalty + gapPenalty, baseScore - 1);
  const startPenalty = Math.min(firstIndex, baseScore - 1);
  return baseScore - Math.min(baseScore - 1, startPenalty + scorePenalty * 2);
}

function parseZoneRgbSearch(searchText) {
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
  const components = sanitized.match(/[+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?/g) || [];
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

function scoreZoneMatch(zone, queryTerms, parsedZoneRgb, rawQuery = "") {
  if (parsedZoneRgb != null && zone.zoneRgb === parsedZoneRgb) {
    return 500;
  }
  if (!queryTerms.length) {
    return 0;
  }
  let score = 0;
  const normalizedQuery = String(rawQuery || "").trim().toLowerCase();
  const nameSearch = String(zone._nameSearch || "");
  const nameSearchCompact = String(zone._nameSearchCompact || "");
  const normalizedCompactQuery = compactSearchText(rawQuery);
  if (normalizedQuery && nameSearch.includes(normalizedQuery)) {
    score += 320;
  }
  if (normalizedCompactQuery) {
    const fullCompactMatch = scoreTermMatch(nameSearchCompact, normalizedCompactQuery, 280);
    if (Number.isFinite(fullCompactMatch)) {
      score += fullCompactMatch;
    }
  }
  for (const term of queryTerms) {
    const nameScore = scoreTermMatch(nameSearch, term, 120);
    const best =
      parsedZoneRgb != null
        ? Math.max(
            nameScore,
            scoreTermMatch(zone.rgbKey, term, 220),
            scoreTermMatch(zone.rgbSpaced, term, 220),
            scoreTermMatch(zone.normalizedKey, term, 240),
            scoreTermMatch(zone.normalizedSpaced, term, 240),
            scoreTermMatch(zone.hexKey, term, 230),
            scoreTermMatch(zone.hashHexKey, term, 230),
            scoreTermMatch(zone.bareHexKey, term, 225),
          )
        : nameScore;
    if (!Number.isFinite(best)) {
      return Number.NEGATIVE_INFINITY;
    }
    score += best;
  }
  return score;
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
      _nameSearchCompact: compactSearchText(name),
    });
  }
  normalized.sort((left, right) => {
    if (left.order !== right.order) {
      return left.order - right.order;
    }
    if (left.confirmed !== right.confirmed) {
      return left.confirmed ? -1 : 1;
    }
    return String(left.name || "").localeCompare(String(right.name || ""));
  });
  return normalized;
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
    const score = scoreZoneMatch(zone, terms, parsedZoneRgb, query);
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

export async function loadZoneCatalog(
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

export function zoneCatalogEntryForRgb(zoneCatalog, zoneRgbInput) {
  const zoneRgb = Number.parseInt(zoneRgbInput, 10);
  if (!Number.isFinite(zoneRgb)) {
    return null;
  }
  return (Array.isArray(zoneCatalog) ? zoneCatalog : []).find((zone) => zone.zoneRgb === zoneRgb) || null;
}

export function zoneDisplayNameFromCatalog(zoneCatalog, zoneRgbInput) {
  const zone = zoneCatalogEntryForRgb(zoneCatalog, zoneRgbInput);
  if (!zone) {
    return "";
  }
  return String(zone.name || "").trim();
}
