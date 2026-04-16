import { findZoneMatches } from "./map-zone-catalog.js";
import {
  FISH_FILTER_TERM_ORDER,
  appendSearchExpressionTerm,
  buildSearchExpressionStatePatch,
  findSearchExpressionTermPath,
  groupSearchExpressionNodes,
  moveSearchExpressionNodeToIndex,
  moveSearchExpressionNodeToGroup,
  normalizeFishFilterTerm,
  normalizeFishFilterTerms,
  removeSearchExpressionNode,
  resolveSearchExpression,
  setSearchExpressionBoundaryOperator,
  resolveSelectedSearchTerms,
  setSearchExpressionGroupOperator,
} from "./map-search-contract.js";
import { resolveSearchProjection } from "./map-search-projection.js";

export { normalizeFishFilterTerm, normalizeFishFilterTerms } from "./map-search-contract.js";
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
  red: Object.freeze({
    label: "Red",
    description: "Prize or red-grade fish and loot.",
    searchText: "red prize red-grade fish loot item grade",
    icon: "nav-dex",
    iconClass: "text-error",
  }),
  yellow: Object.freeze({
    label: "Yellow",
    description: "Rare or yellow-grade fish and loot.",
    searchText: "yellow rare yellow-grade fish loot item grade",
    icon: "nav-dex",
    iconClass: "text-warning",
  }),
  blue: Object.freeze({
    label: "Blue",
    description: "High-quality or blue-grade fish and loot.",
    searchText: "blue highquality high-quality high quality blue-grade fish loot item grade",
    icon: "nav-dex",
    iconClass: "text-info",
  }),
  green: Object.freeze({
    label: "Green",
    description: "General or green-grade fish and loot.",
    searchText: "green general green-grade fish loot item grade",
    icon: "nav-dex",
    iconClass: "text-success",
  }),
  white: Object.freeze({
    label: "White",
    description: "Trash or white-grade fish and loot.",
    searchText: "white trash white-grade fish loot item grade",
    icon: "nav-dex",
    iconClass: "text-base-content/70",
  }),
});
const FISH_GRADE_FILTER_TERMS = new Set(["red", "yellow", "blue", "green", "white"]);

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
  const search = isPlainObject(signals?._map_ui?.search) ? signals._map_ui.search : {};
  const expression = resolveSearchExpression(
    search.expression,
    search.selectedTerms,
    signals?._map_bridged?.filters,
  );
  const selectedTerms = resolveSelectedSearchTerms(
    search.selectedTerms,
    signals?._map_bridged?.filters,
    expression,
  );
  const projectedFilters = resolveSearchProjection(signals);
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
      search: {
        searchText: String(search.query ?? ""),
        expression,
        selectedTerms,
      },
      filters: {
        searchText: String(search.query ?? ""),
        fishIds: normalizeIntegerList(projectedFilters.fishIds),
        zoneRgbs: normalizeIntegerList(projectedFilters.zoneRgbs),
        semanticFieldIdsByLayer: normalizeSemanticFieldIdsByLayer(projectedFilters.semanticFieldIdsByLayer),
        fishFilterTerms: normalizeFishFilterTerms(projectedFilters.fishFilterTerms),
      },
    },
    sharedFishState: normalizeSharedFishState(signals?._shared_fish),
  };
}

export function resolveSelectedSearchTermsFromBundle(stateBundle) {
  const expression = resolveSearchExpression(stateBundle?.inputState?.search?.expression);
  const selectedTerms = resolveSelectedSearchTerms(
    stateBundle?.inputState?.search?.selectedTerms,
    null,
    expression,
  );
  if (
    selectedTerms.length ||
    Array.isArray(stateBundle?.inputState?.search?.selectedTerms) ||
    stateBundle?.inputState?.search?.expression !== undefined
  ) {
    return selectedTerms;
  }
  return resolveSelectedSearchTerms(undefined, stateBundle?.inputState?.filters);
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
    {
      term: "red",
      patterns: [/\bred\b/g, /\bprize\b/g],
    },
    {
      term: "yellow",
      patterns: [/\byellow\b/g, /\brare\b/g],
    },
    {
      term: "blue",
      patterns: [/\bblue\b/g, /\bhigh[\s_-]*quality\b/g],
    },
    {
      term: "green",
      patterns: [/\bgreen\b/g, /\bgeneral\b/g],
    },
    {
      term: "white",
      patterns: [/\bwhite\b/g, /\btrash\b/g],
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

function resolveFishGradeFilterTerm(fish) {
  const grade = String(fish?.grade ?? "").trim().toLowerCase();
  if (fish?.isPrize === true || fish?.is_prize === true || grade === "prize" || grade === "red") {
    return "red";
  }
  if (grade === "rare" || grade === "yellow") {
    return "yellow";
  }
  if (
    grade === "highquality" ||
    grade === "high_quality" ||
    grade === "high-quality" ||
    grade === "blue"
  ) {
    return "blue";
  }
  if (grade === "general" || grade === "green") {
    return "green";
  }
  if (grade === "trash" || grade === "white") {
    return "white";
  }
  return "";
}

function resolveFishIdentityIds(fish) {
  const ids = [];
  const seen = new Set();
  for (const value of [fish?.fishId, fish?.itemId]) {
    const id = Number.parseInt(value, 10);
    if (!Number.isInteger(id) || id <= 0 || seen.has(id)) {
      continue;
    }
    seen.add(id);
    ids.push(id);
  }
  return ids;
}

function fishMatchesFilterTerms(fish, filterTerms, sharedFishState) {
  if (!filterTerms.length) {
    return true;
  }
  const fishIdentityIds = resolveFishIdentityIds(fish);
  if (!fishIdentityIds.length) {
    return false;
  }
  const selectedGradeTerms = filterTerms.filter((term) => FISH_GRADE_FILTER_TERMS.has(term));
  if (selectedGradeTerms.length) {
    const gradeTerm = resolveFishGradeFilterTerm(fish);
    if (!selectedGradeTerms.includes(gradeTerm)) {
      return false;
    }
  }
  for (const term of filterTerms) {
    if (FISH_GRADE_FILTER_TERMS.has(term)) {
      continue;
    }
    if (
      term === "favourite" &&
      !fishIdentityIds.some((fishId) => sharedFishState?.favouriteSet?.has(fishId))
    ) {
      return false;
    }
    if (
      term === "missing" &&
      fishIdentityIds.some((fishId) => sharedFishState?.caughtSet?.has(fishId))
    ) {
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
  const effectiveFishFilterTerms = normalizeFishFilterTerms(filterDirectives.directTerms);
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
  const expression = resolveSearchExpression(
    stateBundle?.inputState?.search?.expression,
    stateBundle?.inputState?.search?.selectedTerms,
    stateBundle?.inputState?.filters,
  );
  return buildSearchExpressionStatePatch(appendSearchExpressionTerm(expression, match), {
    query: "",
    open: false,
  });
}

export function buildSearchSelectionRemovalSignalPatch(signals, target) {
  const stateBundle = buildSearchPanelStateBundle(signals);
  const expression = resolveSearchExpression(
    stateBundle?.inputState?.search?.expression,
    stateBundle?.inputState?.search?.selectedTerms,
    stateBundle?.inputState?.filters,
  );
  const expressionPath = String(target?.expressionPath || "").trim();
  let removalPath = expressionPath;
  if (!removalPath) {
    const fishFilterTerm = normalizeFishFilterTerm(target?.fishFilterTerm);
    if (fishFilterTerm) {
      removalPath = findSearchExpressionTermPath(expression, {
        kind: "fish-filter",
        term: fishFilterTerm,
      });
    }
  }
  if (!removalPath) {
    const zoneRgb = Number.parseInt(target?.zoneRgb, 10);
    if (Number.isFinite(zoneRgb)) {
      removalPath = findSearchExpressionTermPath(expression, { kind: "zone", zoneRgb });
    }
  }
  if (!removalPath) {
    const semanticLayerId = String(target?.semanticLayerId || "").trim();
    const semanticFieldId = Number.parseInt(target?.semanticFieldId, 10);
    if (semanticLayerId && Number.isFinite(semanticFieldId)) {
      removalPath = findSearchExpressionTermPath(expression, {
        kind: "semantic",
        layerId: semanticLayerId,
        fieldId: semanticFieldId,
      });
    }
  }
  if (!removalPath) {
    const fishId = Number.parseInt(target?.fishId, 10);
    if (Number.isFinite(fishId)) {
      removalPath = findSearchExpressionTermPath(expression, { kind: "fish", fishId });
    }
  }
  return buildSearchExpressionStatePatch(removeSearchExpressionNode(expression, removalPath));
}

export function buildSearchExpressionOperatorSignalPatch(signals, target) {
  const stateBundle = buildSearchPanelStateBundle(signals);
  const expression = resolveSearchExpression(
    stateBundle?.inputState?.search?.expression,
    stateBundle?.inputState?.search?.selectedTerms,
    stateBundle?.inputState?.filters,
  );
  const boundaryIndex = Number.parseInt(
    target?.boundaryIndex ?? target?.expressionBoundaryIndex,
    10,
  );
  return buildSearchExpressionStatePatch(
    Number.isInteger(boundaryIndex)
      ? setSearchExpressionBoundaryOperator(
        expression,
        target?.expressionPath ?? target?.groupPath,
        boundaryIndex,
        target?.operator ?? target?.nextOperator,
      )
      : setSearchExpressionGroupOperator(
        expression,
        target?.expressionPath ?? target?.groupPath,
        target?.operator ?? target?.nextOperator,
      ),
  );
}

export function buildSearchExpressionDragSignalPatch(signals, target) {
  const stateBundle = buildSearchPanelStateBundle(signals);
  const expression = resolveSearchExpression(
    stateBundle?.inputState?.search?.expression,
    stateBundle?.inputState?.search?.selectedTerms,
    stateBundle?.inputState?.filters,
  );
  const sourcePath = String(target?.sourcePath ?? target?.dragPath ?? "").trim();
  const targetNodePath = String(
    target?.targetNodePath ??
      target?.nodePath ??
      target?.targetTermPath ??
      target?.termPath ??
      "",
  ).trim();
  const targetGroupPath = String(target?.targetGroupPath ?? target?.groupPath ?? "").trim();
  const targetGroupIndex = Number.parseInt(
    target?.targetGroupIndex ?? target?.groupIndex ?? target?.slotIndex,
    10,
  );
  const nextExpression = Number.isInteger(targetGroupIndex) && targetGroupPath
    ? moveSearchExpressionNodeToIndex(expression, sourcePath, targetGroupPath, targetGroupIndex)
    : targetNodePath
      ? groupSearchExpressionNodes(expression, sourcePath, targetNodePath, {
        operator: target?.groupOperator ?? target?.operator ?? "and",
      })
      : moveSearchExpressionNodeToGroup(expression, sourcePath, targetGroupPath);
  if (JSON.stringify(nextExpression) === JSON.stringify(expression)) {
    return null;
  }
  return buildSearchExpressionStatePatch(nextExpression);
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
