import { findZoneMatches } from "./map-zone-catalog.js";

const FISH_FILTER_TERM_ORDER = Object.freeze(["favourite", "missing"]);
const FISH_FILTER_TERM_METADATA = Object.freeze({
  favourite: Object.freeze({
    label: "Favourite",
    description: "Fish marked with a heart in Fishydex.",
    searchText: "favourite favourites favorite favorites heart liked",
    icon: "heart-fill",
    iconClass: "text-error",
  }),
  missing: Object.freeze({
    label: "Missing",
    description: "Fish not marked caught in Fishydex.",
    searchText: "missing uncaught not caught not yet caught",
    icon: "check-circle-dash-fill",
    iconClass: "text-warning",
  }),
});

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
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

function normalizeSemanticFieldIdsByLayer(values) {
  if (!isPlainObject(values)) {
    return {};
  }
  const out = {};
  for (const [layerIdRaw, fieldIdsRaw] of Object.entries(values)) {
    const layerId = String(layerIdRaw ?? "").trim();
    if (!layerId || !Array.isArray(fieldIdsRaw)) {
      continue;
    }
    const fieldIds = normalizeIntegerList(fieldIdsRaw);
    if (fieldIds.length) {
      out[layerId] = fieldIds;
    }
  }
  return out;
}

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

function semanticTermLookupKey(layerId, fieldId) {
  return `${String(layerId || "").trim()}:${Number.parseInt(fieldId, 10)}`;
}

export function buildSemanticTermLookup(stateBundle) {
  return new Map(
    (stateBundle?.state?.catalog?.semanticTerms || []).map((term) => [
      semanticTermLookupKey(term.layerId, term.fieldId),
      term,
    ]),
  );
}

function normalizeSharedFishState(value) {
  const caughtIds = normalizeIntegerList(value?.caughtIds);
  const favouriteIds = normalizeIntegerList(value?.favouriteIds);
  return {
    caughtIds,
    favouriteIds,
    caughtSet: new Set(caughtIds),
    favouriteSet: new Set(favouriteIds),
  };
}

export function buildSearchPanelStateBundle(signals) {
  const runtime = isPlainObject(signals?._map_runtime) ? signals._map_runtime : {};
  const bridgedFilters = isPlainObject(signals?._map_bridged?.filters) ? signals._map_bridged.filters : {};
  const search = isPlainObject(signals?._map_ui?.search) ? signals._map_ui.search : {};
  return {
    state: {
      ready: runtime.ready === true,
      catalog: {
        fish: Array.isArray(runtime.catalog?.fish) ? cloneJson(runtime.catalog.fish) : [],
        semanticTerms: Array.isArray(runtime.catalog?.semanticTerms)
          ? cloneJson(runtime.catalog.semanticTerms)
          : [],
      },
    },
    inputState: {
      filters: {
        searchText: String(search.query ?? ""),
        fishIds: normalizeIntegerList(bridgedFilters.fishIds),
        semanticFieldIdsByLayer: normalizeSemanticFieldIdsByLayer(
          bridgedFilters.semanticFieldIdsByLayer,
        ),
        fishFilterTerms: normalizeFishFilterTerms(bridgedFilters.fishFilterTerms),
      },
    },
    sharedFishState: normalizeSharedFishState(signals?._shared_fish),
  };
}

export function resolveSelectedFishIds(stateBundle) {
  return normalizeIntegerList(stateBundle?.inputState?.filters?.fishIds);
}

export function resolveSelectedSemanticFieldIdsByLayer(stateBundle) {
  return normalizeSemanticFieldIdsByLayer(stateBundle?.inputState?.filters?.semanticFieldIdsByLayer);
}

export function resolveSelectedZoneRgbs(stateBundle) {
  const selectedByLayer = resolveSelectedSemanticFieldIdsByLayer(stateBundle);
  return normalizeIntegerList(selectedByLayer.zone_mask);
}

export function resolveSelectedFishFilterTerms(stateBundle) {
  return normalizeFishFilterTerms(stateBundle?.inputState?.filters?.fishFilterTerms);
}

function addSelectedFishId(selectedFishIds, fishId) {
  return selectedFishIds.includes(fishId) ? selectedFishIds : selectedFishIds.concat(fishId);
}

function removeSelectedFishId(selectedFishIds, fishId) {
  return selectedFishIds.filter((value) => value !== fishId);
}

function addSelectedZoneRgb(selectedZoneRgbs, zoneRgb) {
  return selectedZoneRgbs.includes(zoneRgb) ? selectedZoneRgbs : selectedZoneRgbs.concat(zoneRgb);
}

function removeSelectedZoneRgb(selectedZoneRgbs, zoneRgb) {
  return selectedZoneRgbs.filter((value) => value !== zoneRgb);
}

function addSelectedSemanticFieldId(selectedFieldIds, fieldId) {
  return selectedFieldIds.includes(fieldId) ? selectedFieldIds : selectedFieldIds.concat(fieldId);
}

function removeSelectedSemanticFieldId(selectedFieldIds, fieldId) {
  return selectedFieldIds.filter((value) => value !== fieldId);
}

function updateSelectedSemanticFieldIdsByLayer(selectedByLayer, layerId, nextFieldIds) {
  const next = {
    ...normalizeSemanticFieldIdsByLayer(selectedByLayer),
  };
  const normalizedLayerId = String(layerId || "").trim();
  if (!normalizedLayerId) {
    return next;
  }
  if (!Array.isArray(nextFieldIds) || nextFieldIds.length === 0) {
    delete next[normalizedLayerId];
    return next;
  }
  next[normalizedLayerId] = normalizeIntegerList(nextFieldIds);
  return normalizeSemanticFieldIdsByLayer(next);
}

function addSelectedFishFilterTerm(selectedTerms, term) {
  return normalizeFishFilterTerms((selectedTerms || []).concat(term));
}

function removeSelectedFishFilterTerm(selectedTerms, term) {
  const normalizedTerm = normalizeFishFilterTerm(term);
  return normalizeFishFilterTerms((selectedTerms || []).filter((entry) => entry !== normalizedTerm));
}

export function parseFishFilterDirectives(searchText) {
  const rawQuery = String(searchText || "").trim().toLowerCase().replace(/\s+/g, " ");
  if (!rawQuery) {
    return {
      rawQuery: "",
      remainingQuery: "",
      directTerms: [],
    };
  }

  let remainingQuery = rawQuery;
  const directTerms = new Set();
  const replacements = [
    {
      term: "missing",
      patterns: [/\bnot\s+yet\s+caught\b/g, /\bnot\s+caught\b/g, /\buncaught\b/g, /\bmissing\b/g],
    },
    {
      term: "favourite",
      patterns: [/\bfavou?rites?\b/g],
    },
  ];
  for (const replacement of replacements) {
    for (const pattern of replacement.patterns) {
      remainingQuery = remainingQuery.replace(pattern, () => {
        directTerms.add(replacement.term);
        return " ";
      });
    }
  }
  remainingQuery = remainingQuery.replace(/\s+/g, " ").trim();
  return {
    rawQuery,
    remainingQuery,
    directTerms: FISH_FILTER_TERM_ORDER.filter((term) => directTerms.has(term)),
  };
}

function scoreTermMatch(haystack, term, baseScore) {
  const index = String(haystack || "").indexOf(term);
  if (index < 0) {
    return Number.NEGATIVE_INFINITY;
  }
  return baseScore - Math.min(index, baseScore - 1);
}

function scoreFishMatch(fish, queryTerms) {
  if (!queryTerms.length) {
    return 0;
  }
  const name = String(fish.name || "").toLowerCase();
  const id = String(fish.fishId || "");
  let score = 0;
  for (const term of queryTerms) {
    if (id === term) {
      score += 200;
      continue;
    }
    const idIndex = id.indexOf(term);
    if (idIndex >= 0) {
      score += 120 - idIndex;
      continue;
    }
    const nameIndex = name.indexOf(term);
    if (nameIndex >= 0) {
      score += 90 - Math.min(nameIndex, 60);
      continue;
    }
    return Number.NEGATIVE_INFINITY;
  }
  return score;
}

function fishMatchesFilterTerms(fish, filterTerms, sharedFishState) {
  if (!filterTerms.length) {
    return true;
  }
  const fishId = Number.parseInt(fish?.fishId ?? fish?.itemId, 10);
  if (!Number.isInteger(fishId) || fishId <= 0) {
    return false;
  }
  for (const term of filterTerms) {
    if (term === "favourite" && !sharedFishState?.favouriteSet?.has(fishId)) {
      return false;
    }
    if (term === "missing" && sharedFishState?.caughtSet?.has(fishId)) {
      return false;
    }
  }
  return true;
}

function findFishMatches(catalogFish, searchText, options = {}) {
  const query = String(searchText || "").trim().toLowerCase();
  const terms = query ? query.split(/\s+/g).filter(Boolean) : [];
  const includeAllWhenEmpty = options.includeAllWhenEmpty === true;
  const filterTerms = normalizeFishFilterTerms(options.filterTerms);
  const sharedFishState = normalizeSharedFishState(options.sharedFishState);
  if (!terms.length && !includeAllWhenEmpty) {
    return [];
  }
  const filtered = [];
  for (const fish of catalogFish || []) {
    if (!fishMatchesFilterTerms(fish, filterTerms, sharedFishState)) {
      continue;
    }
    const score = terms.length ? scoreFishMatch(fish, terms) : 0;
    if (terms.length && !Number.isFinite(score)) {
      continue;
    }
    filtered.push({
      kind: "fish",
      ...fish,
      _score: Number.isFinite(score) ? score : 0,
    });
  }
  filtered.sort((left, right) => {
    if (terms.length && right._score !== left._score) {
      return right._score - left._score;
    }
    return String(left.name || "").localeCompare(String(right.name || ""));
  });
  return filtered;
}

function findFishFilterMatches(searchText, selectedTerms) {
  const query = String(searchText || "").trim().toLowerCase();
  const terms = query ? query.split(/\s+/g).filter(Boolean) : [];
  if (!terms.length) {
    return [];
  }
  const selected = new Set(normalizeFishFilterTerms(selectedTerms));
  const matches = [];
  for (const term of FISH_FILTER_TERM_ORDER) {
    if (selected.has(term)) {
      continue;
    }
    const metadata = FISH_FILTER_TERM_METADATA[term];
    let score = 0;
    for (const queryTerm of terms) {
      const best = Math.max(
        term === queryTerm ? 240 : Number.NEGATIVE_INFINITY,
        scoreTermMatch(String(metadata?.label || "").toLowerCase(), queryTerm, 200),
        scoreTermMatch(String(metadata?.description || "").toLowerCase(), queryTerm, 140),
        scoreTermMatch(String(metadata?.searchText || "").toLowerCase(), queryTerm, 160),
      );
      if (!Number.isFinite(best)) {
        score = Number.NEGATIVE_INFINITY;
        break;
      }
      score += best;
    }
    if (!Number.isFinite(score)) {
      continue;
    }
    matches.push({
      kind: "fish-filter",
      term,
      label: metadata?.label || term,
      description: metadata?.description || "",
      _score: score,
    });
  }
  matches.sort((left, right) => {
    if (right._score !== left._score) {
      return right._score - left._score;
    }
    return String(left.label || "").localeCompare(String(right.label || ""));
  });
  return matches;
}

export function buildDefaultFishFilterMatches(stateBundle) {
  const selected = new Set(resolveSelectedFishFilterTerms(stateBundle));
  return FISH_FILTER_TERM_ORDER.filter((term) => !selected.has(term)).map((term) => ({
    kind: "fish-filter",
    term,
    label: FISH_FILTER_TERM_METADATA[term]?.label || term,
    description: FISH_FILTER_TERM_METADATA[term]?.description || "",
    _score: 0,
  }));
}

function scoreSemanticMatch(term, queryTerms) {
  if (!queryTerms.length) {
    return 0;
  }
  const fieldId = String(term.fieldId || "");
  const label = String(term.label || "").toLowerCase();
  const description = String(term.description || "").toLowerCase();
  const layerName = String(term.layerName || "").toLowerCase();
  const searchText = String(term.searchText || "").toLowerCase();
  let score = 0;
  for (const queryTerm of queryTerms) {
    const best = Math.max(
      fieldId === queryTerm ? 220 : Number.NEGATIVE_INFINITY,
      scoreTermMatch(label, queryTerm, 170),
      scoreTermMatch(description, queryTerm, 130),
      scoreTermMatch(layerName, queryTerm, 90),
      scoreTermMatch(searchText, queryTerm, 80),
    );
    if (!Number.isFinite(best)) {
      return Number.NEGATIVE_INFINITY;
    }
    score += best;
  }
  return score;
}

function findSemanticMatches(semanticTerms, searchText) {
  const query = String(searchText || "").trim().toLowerCase();
  const terms = query ? query.split(/\s+/g).filter(Boolean) : [];
  if (!query) {
    return [];
  }
  const filteredByKey = new Map();
  for (const term of semanticTerms || []) {
    if (!term || String(term.layerId || "").trim() === "zone_mask") {
      continue;
    }
    const score = scoreSemanticMatch(term, terms);
    if (!Number.isFinite(score)) {
      continue;
    }
    const candidate = {
      kind: "semantic",
      ...term,
      _score: score,
    };
    const key = semanticTermLookupKey(term.layerId, term.fieldId);
    const existing = filteredByKey.get(key);
    if (
      !existing ||
      candidate._score > existing._score ||
      (candidate._score === existing._score &&
        String(candidate.label || "").length < String(existing.label || "").length)
    ) {
      filteredByKey.set(key, candidate);
    }
  }
  const filtered = Array.from(filteredByKey.values());
  filtered.sort((left, right) => {
    if (right._score !== left._score) {
      return right._score - left._score;
    }
    if (left.layerName !== right.layerName) {
      return String(left.layerName || "").localeCompare(String(right.layerName || ""));
    }
    return String(left.label || "").localeCompare(String(right.label || ""));
  });
  return filtered;
}

function searchMatchPriority(match) {
  if (match?.kind === "fish-filter") {
    return -1;
  }
  if (match?.kind === "fish") {
    return 0;
  }
  if (match?.kind === "zone") {
    return 1;
  }
  if (match?.kind === "semantic") {
    return 2;
  }
  return 9;
}

export function buildSearchMatches(stateBundle, searchText, zoneCatalog = []) {
  const catalogFish = stateBundle?.state?.catalog?.fish || [];
  const semanticTerms = stateBundle?.state?.catalog?.semanticTerms || [];
  const selectedFishIds = new Set(resolveSelectedFishIds(stateBundle));
  const selectedFishFilterTerms = resolveSelectedFishFilterTerms(stateBundle);
  const selectedSemanticFieldIdsByLayer = resolveSelectedSemanticFieldIdsByLayer(stateBundle);
  const selectedZoneRgbs = new Set(resolveSelectedZoneRgbs(stateBundle));
  const sharedFishState = normalizeSharedFishState(stateBundle?.sharedFishState);
  const filterDirectives = parseFishFilterDirectives(searchText);
  const effectiveFishFilterTerms = normalizeFishFilterTerms(
    selectedFishFilterTerms.concat(filterDirectives.directTerms),
  );
  const fishFilterMatches = findFishFilterMatches(searchText, selectedFishFilterTerms);
  const fishMatches = findFishMatches(catalogFish, filterDirectives.remainingQuery, {
    includeAllWhenEmpty: effectiveFishFilterTerms.length > 0,
    filterTerms: effectiveFishFilterTerms,
    sharedFishState,
  }).filter((fish) => !selectedFishIds.has(fish.fishId));
  const zoneMatches = findZoneMatches(zoneCatalog, filterDirectives.remainingQuery).filter(
    (zone) => !selectedZoneRgbs.has(zone.zoneRgb),
  );
  const semanticMatches = findSemanticMatches(semanticTerms, filterDirectives.remainingQuery).filter(
    (term) => !(selectedSemanticFieldIdsByLayer[term.layerId] || []).includes(term.fieldId),
  );
  return fishFilterMatches.concat(fishMatches, semanticMatches, zoneMatches).sort((left, right) => {
    const leftPriority = searchMatchPriority(left);
    const rightPriority = searchMatchPriority(right);
    if (leftPriority !== rightPriority) {
      return leftPriority - rightPriority;
    }
    if (right._score !== left._score) {
      return right._score - left._score;
    }
    return String(left.name || left.label || "").localeCompare(
      String(right.name || right.label || ""),
    );
  });
}

export function buildSearchMatchSignalPatch(signals, match) {
  const stateBundle = buildSearchPanelStateBundle(signals);
  const selectedFishIds = resolveSelectedFishIds(stateBundle);
  const selectedFishFilterTerms = resolveSelectedFishFilterTerms(stateBundle);
  const selectedSemanticFieldIdsByLayer = resolveSelectedSemanticFieldIdsByLayer(stateBundle);
  const patch = {
    _map_ui: {
      search: {
        query: "",
        open: false,
      },
    },
    _map_bridged: {
      filters: {},
    },
  };
  if (match?.kind === "fish-filter") {
    patch._map_bridged.filters.fishFilterTerms = addSelectedFishFilterTerm(
      selectedFishFilterTerms,
      match.term,
    );
  } else if (match?.kind === "fish") {
    patch._map_bridged.filters.fishIds = addSelectedFishId(selectedFishIds, match.fishId);
  } else if (match?.kind === "zone") {
    patch._map_bridged.filters.semanticFieldIdsByLayer = updateSelectedSemanticFieldIdsByLayer(
      selectedSemanticFieldIdsByLayer,
      "zone_mask",
      addSelectedZoneRgb(resolveSelectedZoneRgbs(stateBundle), match.zoneRgb),
    );
  } else if (match?.kind === "semantic") {
    patch._map_bridged.filters.semanticFieldIdsByLayer = updateSelectedSemanticFieldIdsByLayer(
      selectedSemanticFieldIdsByLayer,
      match.layerId,
      addSelectedSemanticFieldId(
        selectedSemanticFieldIdsByLayer[match.layerId] || [],
        match.fieldId,
      ),
    );
  }
  return patch;
}

export function buildSearchSelectionRemovalSignalPatch(signals, target) {
  const stateBundle = buildSearchPanelStateBundle(signals);
  const selectedSemanticFieldIdsByLayer = resolveSelectedSemanticFieldIdsByLayer(stateBundle);
  const patch = {
    _map_bridged: {
      filters: {},
    },
  };
  const fishFilterTerm = normalizeFishFilterTerm(target?.fishFilterTerm);
  if (fishFilterTerm) {
    patch._map_bridged.filters.fishFilterTerms = removeSelectedFishFilterTerm(
      resolveSelectedFishFilterTerms(stateBundle),
      fishFilterTerm,
    );
    return patch;
  }
  const zoneRgb = Number.parseInt(target?.zoneRgb, 10);
  if (Number.isFinite(zoneRgb)) {
    patch._map_bridged.filters.semanticFieldIdsByLayer = updateSelectedSemanticFieldIdsByLayer(
      selectedSemanticFieldIdsByLayer,
      "zone_mask",
      removeSelectedZoneRgb(resolveSelectedZoneRgbs(stateBundle), zoneRgb),
    );
    return patch;
  }
  const semanticLayerId = String(target?.semanticLayerId || "").trim();
  const semanticFieldId = Number.parseInt(target?.semanticFieldId, 10);
  if (semanticLayerId && Number.isFinite(semanticFieldId)) {
    patch._map_bridged.filters.semanticFieldIdsByLayer = updateSelectedSemanticFieldIdsByLayer(
      selectedSemanticFieldIdsByLayer,
      semanticLayerId,
      removeSelectedSemanticFieldId(
        selectedSemanticFieldIdsByLayer[semanticLayerId] || [],
        semanticFieldId,
      ),
    );
    return patch;
  }
  const fishId = Number.parseInt(target?.fishId, 10);
  patch._map_bridged.filters.fishIds = removeSelectedFishId(resolveSelectedFishIds(stateBundle), fishId);
  return patch;
}

export function patchTouchesSearchPanelSignals(patch) {
  if (!isPlainObject(patch)) {
    return false;
  }
  if (patch._map_ui?.search != null) {
    return true;
  }
  if (patch._map_bridged?.filters != null) {
    return true;
  }
  if (patch._map_runtime?.ready != null) {
    return true;
  }
  if (patch._map_runtime?.catalog != null) {
    return true;
  }
  if (patch._shared_fish != null) {
    return true;
  }
  return false;
}

export function fishFilterTermMetadata(term) {
  return FISH_FILTER_TERM_METADATA[normalizeFishFilterTerm(term)] || null;
}
