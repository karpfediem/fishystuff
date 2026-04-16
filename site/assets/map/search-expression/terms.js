import { FISH_FILTER_TERM_ORDER } from "./constants.js";
import { isPlainObject } from "./shared.js";

export function normalizeFishFilterTerm(value) {
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
  if (normalized === "red" || normalized === "prize") {
    return "red";
  }
  if (normalized === "yellow" || normalized === "rare") {
    return "yellow";
  }
  if (
    normalized === "blue" ||
    normalized === "highquality" ||
    normalized === "high_quality" ||
    normalized === "high-quality"
  ) {
    return "blue";
  }
  if (normalized === "green" || normalized === "general") {
    return "green";
  }
  if (normalized === "white" || normalized === "trash") {
    return "white";
  }
  return "";
}

export function normalizeFishFilterTerms(values) {
  const selected = new Set();
  for (const value of Array.isArray(values) ? values : []) {
    const normalized = normalizeFishFilterTerm(value);
    if (normalized) {
      selected.add(normalized);
    }
  }
  return FISH_FILTER_TERM_ORDER.filter((term) => selected.has(term));
}

export function normalizePatchBound(value) {
  const normalized = String(value ?? "").trim().toLowerCase();
  if (normalized === "from" || normalized === "start" || normalized === "since") {
    return "from";
  }
  if (
    normalized === "to" ||
    normalized === "until" ||
    normalized === "end" ||
    normalized === "through"
  ) {
    return "to";
  }
  return "";
}

export function normalizePatchId(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

export function normalizeSearchTerm(raw) {
  if (!isPlainObject(raw)) {
    return null;
  }
  const kind = String(raw.kind ?? raw.type ?? "").trim().toLowerCase();
  if (kind === "patch-bound" || kind === "patch") {
    const bound = normalizePatchBound(raw.bound ?? raw.patchBound ?? raw.side);
    const patchId = normalizePatchId(raw.patchId ?? raw.value ?? raw.id);
    return bound && patchId ? { kind: "patch-bound", bound, patchId } : null;
  }
  if (kind === "fish-filter") {
    const term = normalizeFishFilterTerm(raw.term ?? raw.fishFilterTerm);
    return term ? { kind: "fish-filter", term } : null;
  }
  if (kind === "fish") {
    const fishId = Number.parseInt(raw.fishId ?? raw.itemId, 10);
    return Number.isInteger(fishId) && fishId > 0 ? { kind: "fish", fishId } : null;
  }
  if (kind === "zone") {
    const zoneRgb = Number.parseInt(raw.zoneRgb ?? raw.fieldId, 10);
    return Number.isInteger(zoneRgb) && zoneRgb > 0 ? { kind: "zone", zoneRgb } : null;
  }
  if (kind === "semantic") {
    const layerId = String(raw.layerId ?? "").trim();
    const fieldId = Number.parseInt(raw.fieldId, 10);
    if (layerId === "zone_mask") {
      return Number.isInteger(fieldId) && fieldId > 0 ? { kind: "zone", zoneRgb: fieldId } : null;
    }
    if (!layerId || !Number.isInteger(fieldId) || fieldId <= 0) {
      return null;
    }
    return { kind: "semantic", layerId, fieldId };
  }
  return null;
}

export function searchTermKey(term) {
  if (!term || typeof term !== "object") {
    return "";
  }
  if (term.kind === "patch-bound") {
    const bound = normalizePatchBound(term.bound);
    const patchId = normalizePatchId(term.patchId);
    return bound && patchId ? `patch-bound:${bound}:${patchId}` : "";
  }
  if (term.kind === "fish-filter") {
    return `fish-filter:${normalizeFishFilterTerm(term.term)}`;
  }
  if (term.kind === "fish") {
    const fishId = Number.parseInt(term.fishId, 10);
    return Number.isInteger(fishId) ? `fish:${fishId}` : "";
  }
  if (term.kind === "zone") {
    const zoneRgb = Number.parseInt(term.zoneRgb, 10);
    return Number.isInteger(zoneRgb) ? `zone:${zoneRgb}` : "";
  }
  if (term.kind === "semantic") {
    const layerId = String(term.layerId ?? "").trim();
    const fieldId = Number.parseInt(term.fieldId, 10);
    return layerId && Number.isInteger(fieldId) ? `semantic:${layerId}:${fieldId}` : "";
  }
  return "";
}

export function normalizeSelectedSearchTerms(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  const next = [];
  const seen = new Set();
  for (const value of values) {
    const normalized = normalizeSearchTerm(value);
    if (!normalized) {
      continue;
    }
    const key = searchTermKey(normalized);
    if (!key || seen.has(key)) {
      continue;
    }
    seen.add(key);
    next.push(normalized);
  }
  return next;
}
