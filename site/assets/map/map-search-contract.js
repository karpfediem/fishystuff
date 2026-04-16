export const FISH_FILTER_TERM_ORDER = Object.freeze([
  "favourite",
  "missing",
  "red",
  "yellow",
  "blue",
  "green",
  "white",
]);

export const DEFAULT_SEARCH_EXPRESSION_OPERATOR = "or";
export const EMPTY_SEARCH_EXPRESSION = Object.freeze({
  type: "group",
  operator: DEFAULT_SEARCH_EXPRESSION_OPERATOR,
  children: Object.freeze([]),
});

export const MAP_SEARCH_LAYER_SUPPORT = Object.freeze({
  bookmarks: Object.freeze({
    termKinds: Object.freeze([]),
    attachmentClipModes: Object.freeze([]),
  }),
  fish_evidence: Object.freeze({
    termKinds: Object.freeze(["fish", "fish-filter", "zone"]),
    attachmentClipModes: Object.freeze(["zone-membership"]),
  }),
  minimap: Object.freeze({
    termKinds: Object.freeze([]),
    attachmentClipModes: Object.freeze(["mask-sample"]),
  }),
  node_waypoints: Object.freeze({
    termKinds: Object.freeze([]),
    attachmentClipModes: Object.freeze([]),
  }),
  region_groups: Object.freeze({
    termKinds: Object.freeze(["semantic"]),
    attachmentClipModes: Object.freeze(["mask-sample"]),
  }),
  regions: Object.freeze({
    termKinds: Object.freeze(["semantic"]),
    attachmentClipModes: Object.freeze(["mask-sample"]),
  }),
  zone_mask: Object.freeze({
    termKinds: Object.freeze(["fish", "fish-filter", "zone"]),
    attachmentClipModes: Object.freeze([]),
  }),
});

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

export function normalizeSearchTerm(raw) {
  if (!isPlainObject(raw)) {
    return null;
  }
  const kind = String(raw.kind ?? raw.type ?? "").trim().toLowerCase();
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

function normalizeSearchExpressionOperator(value) {
  return String(value ?? "").trim().toLowerCase() === "and"
    ? "and"
    : DEFAULT_SEARCH_EXPRESSION_OPERATOR;
}

function cloneSearchExpressionNode(node) {
  if (!isPlainObject(node)) {
    return null;
  }
  if (node.type === "term") {
    return {
      type: "term",
      term: normalizeSearchTerm(node.term),
    };
  }
  if (node.type === "group") {
    return {
      type: "group",
      operator: normalizeSearchExpressionOperator(node.operator),
      children: (Array.isArray(node.children) ? node.children : [])
        .map((child) => cloneSearchExpressionNode(child))
        .filter(Boolean),
    };
  }
  return null;
}

function normalizeSearchExpressionPath(path) {
  const normalized = String(path ?? "").trim();
  if (!normalized) {
    return null;
  }
  const parts = normalized.split(".").filter(Boolean);
  if (!parts.length || parts[0] !== "root") {
    return null;
  }
  const indices = [];
  for (const part of parts.slice(1)) {
    if (!/^\d+$/.test(part)) {
      return null;
    }
    indices.push(Number.parseInt(part, 10));
  }
  return {
    path: parts.length === 1 ? "root" : `root.${indices.join(".")}`,
    indices,
  };
}

function searchExpressionPathFromIndices(indices) {
  return Array.isArray(indices) && indices.length ? `root.${indices.join(".")}` : "root";
}

function compactSearchExpressionNode(node, { isRoot = false } = {}) {
  const clonedNode = cloneSearchExpressionNode(node);
  if (!clonedNode) {
    return null;
  }
  if (clonedNode.type === "term") {
    return clonedNode.term ? clonedNode : null;
  }
  const children = clonedNode.children
    .map((child) => compactSearchExpressionNode(child))
    .filter(Boolean);
  if (!isRoot && children.length === 0) {
    return null;
  }
  return {
    type: "group",
    operator: normalizeSearchExpressionOperator(clonedNode.operator),
    children,
  };
}

function visitSearchExpression(node, indices, visitor, currentIndices = []) {
  if (!isPlainObject(node)) {
    return node;
  }
  if (currentIndices.length === indices.length) {
    return visitor(node, currentIndices);
  }
  if (node.type !== "group") {
    return cloneSearchExpressionNode(node);
  }
  const childIndex = indices[currentIndices.length];
  if (!Number.isInteger(childIndex) || childIndex < 0 || childIndex >= node.children.length) {
    return cloneSearchExpressionNode(node);
  }
  const children = node.children.map((child, index) => {
    if (index !== childIndex) {
      return cloneSearchExpressionNode(child);
    }
    return visitSearchExpression(child, indices, visitor, currentIndices.concat(index));
  });
  return {
    type: "group",
    operator: normalizeSearchExpressionOperator(node.operator),
    children: children.filter(Boolean),
  };
}

function adjustPathIndicesAfterRemoval(targetIndices, sourceIndices) {
  const next = Array.isArray(targetIndices) ? targetIndices.slice() : [];
  const source = Array.isArray(sourceIndices) ? sourceIndices : [];
  const sharedLength = Math.min(next.length, source.length);
  for (let index = 0; index < sharedLength; index += 1) {
    const prefixesMatch = next.slice(0, index).every((value, prefixIndex) => value === source[prefixIndex]);
    if (!prefixesMatch) {
      break;
    }
    if (source[index] === next[index]) {
      continue;
    }
    if (source[index] < next[index]) {
      next[index] -= 1;
    }
    break;
  }
  return next;
}

function isSearchExpressionPathPrefix(prefixIndices, candidateIndices) {
  if (!Array.isArray(prefixIndices) || !Array.isArray(candidateIndices)) {
    return false;
  }
  if (prefixIndices.length > candidateIndices.length) {
    return false;
  }
  return prefixIndices.every((value, index) => candidateIndices[index] === value);
}

function searchExpressionIndicesEqual(leftIndices, rightIndices) {
  if (!Array.isArray(leftIndices) || !Array.isArray(rightIndices)) {
    return false;
  }
  if (leftIndices.length !== rightIndices.length) {
    return false;
  }
  return leftIndices.every((value, index) => rightIndices[index] === value);
}

function clampSearchExpressionChildIndex(value, fallback) {
  const normalized = Number.parseInt(value, 10);
  if (!Number.isInteger(normalized)) {
    return fallback;
  }
  return normalized < 0 ? 0 : normalized;
}

export function buildSearchExpressionFromSelectedTerms(selectedTerms, options = {}) {
  const operator = normalizeSearchExpressionOperator(options.operator);
  return {
    type: "group",
    operator,
    children: normalizeSelectedSearchTerms(selectedTerms).map((term) => ({
      type: "term",
      term,
    })),
  };
}

export function searchExpressionNodeKey(node) {
  if (!isPlainObject(node)) {
    return "";
  }
  if (node.type === "term") {
    return `term:${searchTermKey(node.term)}`;
  }
  if (node.type === "group") {
    return `group:${normalizeSearchExpressionOperator(node.operator)}:${(
      Array.isArray(node.children) ? node.children : []
    )
      .map((child) => searchExpressionNodeKey(child))
      .filter(Boolean)
      .join("|")}`;
  }
  return "";
}

function normalizeSearchExpressionNode(raw) {
  if (!isPlainObject(raw)) {
    const term = normalizeSearchTerm(raw);
    return term ? { type: "term", term } : null;
  }

  const type = String(raw.type ?? raw.nodeType ?? "").trim().toLowerCase();
  if (type === "term") {
    const term = normalizeSearchTerm(raw.term ?? raw.value ?? raw.searchTerm);
    return term ? { type: "term", term } : null;
  }

  const childValues =
    Array.isArray(raw.children) ? raw.children
      : Array.isArray(raw.items) ? raw.items
        : Array.isArray(raw.nodes) ? raw.nodes
          : null;
  if (type === "group" || childValues) {
    const children = [];
    const seen = new Set();
    for (const childValue of Array.isArray(childValues) ? childValues : []) {
      const child = normalizeSearchExpressionNode(childValue);
      if (!child) {
        continue;
      }
      const key = searchExpressionNodeKey(child);
      if (!key || seen.has(key)) {
        continue;
      }
      seen.add(key);
      children.push(child);
    }
    return {
      type: "group",
      operator: normalizeSearchExpressionOperator(raw.operator ?? raw.op),
      children,
    };
  }

  const term = normalizeSearchTerm(raw);
  return term ? { type: "term", term } : null;
}

export function normalizeSearchExpression(value) {
  if (value == null) {
    return null;
  }
  const node = normalizeSearchExpressionNode(value);
  if (!node) {
    return null;
  }
  if (node.type === "group") {
    return node;
  }
  return {
    type: "group",
    operator: DEFAULT_SEARCH_EXPRESSION_OPERATOR,
    children: [node],
  };
}

export function selectedSearchTermsFromExpression(expression) {
  const normalizedExpression = normalizeSearchExpression(expression);
  if (!normalizedExpression) {
    return [];
  }
  const terms = [];
  const seen = new Set();
  const visit = (node) => {
    if (!isPlainObject(node)) {
      return;
    }
    if (node.type === "term") {
      const term = normalizeSearchTerm(node.term);
      const key = searchTermKey(term);
      if (!term || !key || seen.has(key)) {
        return;
      }
      seen.add(key);
      terms.push(term);
      return;
    }
    for (const child of Array.isArray(node.children) ? node.children : []) {
      visit(child);
    }
  };
  visit(normalizedExpression);
  return terms;
}

export function resolveSearchExpressionNode(expression, path = "root") {
  const normalizedExpression = resolveSearchExpression(expression);
  const normalizedPath = normalizeSearchExpressionPath(path);
  if (!normalizedPath) {
    return null;
  }
  let current = normalizedExpression;
  for (const index of normalizedPath.indices) {
    if (current?.type !== "group" || !Array.isArray(current.children)) {
      return null;
    }
    current = current.children[index];
    if (!current) {
      return null;
    }
  }
  return cloneSearchExpressionNode(current);
}

export function findSearchExpressionTermPath(expression, target) {
  const normalizedExpression = resolveSearchExpression(expression);
  const normalizedTarget = normalizeSearchTerm(target);
  const targetKey = searchTermKey(normalizedTarget);
  if (!targetKey) {
    return "";
  }
  let foundPath = "";
  const visit = (node, indices = []) => {
    if (foundPath || !isPlainObject(node)) {
      return;
    }
    if (node.type === "term") {
      if (searchTermKey(node.term) === targetKey) {
        foundPath = searchExpressionPathFromIndices(indices);
      }
      return;
    }
    for (let index = 0; index < node.children.length; index += 1) {
      visit(node.children[index], indices.concat(index));
      if (foundPath) {
        return;
      }
    }
  };
  visit(normalizedExpression);
  return foundPath;
}

export function appendSearchExpressionTerm(expression, term, options = {}) {
  const normalizedExpression = resolveSearchExpression(expression);
  const normalizedTerm = normalizeSearchTerm(term);
  if (!normalizedTerm) {
    return normalizedExpression;
  }
  const termKey = searchTermKey(normalizedTerm);
  if (!termKey || findSearchExpressionTermPath(normalizedExpression, normalizedTerm)) {
    return normalizedExpression;
  }
  const targetPath = normalizeSearchExpressionPath(options.groupPath ?? "root");
  const targetNode = targetPath
    ? resolveSearchExpressionNode(normalizedExpression, targetPath.path)
    : null;
  const groupPath = targetNode?.type === "group" ? targetPath : { path: "root", indices: [] };
  return visitSearchExpression(normalizedExpression, groupPath.indices, (node) => {
    if (node?.type !== "group") {
      return cloneSearchExpressionNode(node);
    }
    return {
      type: "group",
      operator: normalizeSearchExpressionOperator(node.operator),
      children: node.children
        .map((child) => cloneSearchExpressionNode(child))
        .concat({ type: "term", term: normalizedTerm }),
    };
  });
}

export function setSearchExpressionGroupOperator(expression, path, operator) {
  const normalizedExpression = resolveSearchExpression(expression);
  const normalizedPath = normalizeSearchExpressionPath(path);
  if (!normalizedPath) {
    return normalizedExpression;
  }
  const normalizedOperator = normalizeSearchExpressionOperator(operator);
  const nextExpression = visitSearchExpression(
    normalizedExpression,
    normalizedPath.indices,
    (node) => {
      if (node?.type !== "group") {
        return cloneSearchExpressionNode(node);
      }
      return {
        type: "group",
        operator: normalizedOperator,
        children: node.children.map((child) => cloneSearchExpressionNode(child)),
      };
    },
  );
  return compactSearchExpressionNode(nextExpression, { isRoot: true }) || buildSearchExpressionFromSelectedTerms([]);
}

export function removeSearchExpressionNode(expression, path) {
  const normalizedExpression = resolveSearchExpression(expression);
  const normalizedPath = normalizeSearchExpressionPath(path);
  if (!normalizedPath || normalizedPath.indices.length === 0) {
    return normalizedExpression;
  }
  const nextExpression = visitSearchExpression(
    normalizedExpression,
    normalizedPath.indices.slice(0, -1),
    (node) => {
      if (node?.type !== "group") {
        return cloneSearchExpressionNode(node);
      }
      const targetIndex = normalizedPath.indices.at(-1);
      return {
        type: "group",
        operator: normalizeSearchExpressionOperator(node.operator),
        children: node.children
          .filter((_child, index) => index !== targetIndex)
          .map((child) => cloneSearchExpressionNode(child)),
      };
    },
  );
  return compactSearchExpressionNode(nextExpression, { isRoot: true }) || buildSearchExpressionFromSelectedTerms([]);
}

export function moveSearchExpressionTermToGroup(expression, sourcePath, groupPath) {
  const normalizedExpression = resolveSearchExpression(expression);
  const normalizedSourcePath = normalizeSearchExpressionPath(sourcePath);
  const sourceNode = resolveSearchExpressionNode(normalizedExpression, normalizedSourcePath?.path);
  if (sourceNode?.type !== "term") {
    return normalizedExpression;
  }
  return moveSearchExpressionNodeToGroup(normalizedExpression, sourcePath, groupPath);
}

export function moveSearchExpressionNodeToGroup(expression, sourcePath, groupPath) {
  const normalizedExpression = resolveSearchExpression(expression);
  const normalizedGroupPath = normalizeSearchExpressionPath(groupPath);
  const targetGroup = resolveSearchExpressionNode(normalizedExpression, normalizedGroupPath?.path);
  if (targetGroup?.type !== "group") {
    return normalizedExpression;
  }
  return moveSearchExpressionNodeToIndex(
    normalizedExpression,
    sourcePath,
    normalizedGroupPath.path,
    targetGroup.children.length,
  );
}

export function moveSearchExpressionNodeToIndex(expression, sourcePath, groupPath, childIndex) {
  const normalizedExpression = resolveSearchExpression(expression);
  const normalizedSourcePath = normalizeSearchExpressionPath(sourcePath);
  const normalizedGroupPath = normalizeSearchExpressionPath(groupPath);
  if (!normalizedSourcePath || !normalizedGroupPath || normalizedSourcePath.indices.length === 0) {
    return normalizedExpression;
  }
  const sourceNode = resolveSearchExpressionNode(normalizedExpression, normalizedSourcePath.path);
  const targetGroup = resolveSearchExpressionNode(normalizedExpression, normalizedGroupPath.path);
  if (!sourceNode || targetGroup?.type !== "group") {
    return normalizedExpression;
  }
  if (isSearchExpressionPathPrefix(normalizedSourcePath.indices, normalizedGroupPath.indices)) {
    return normalizedExpression;
  }
  const sourceParentIndices = normalizedSourcePath.indices.slice(0, -1);
  const sourceIndex = normalizedSourcePath.indices.at(-1);
  let nextChildIndex = clampSearchExpressionChildIndex(childIndex, targetGroup.children.length);
  if (
    searchExpressionIndicesEqual(sourceParentIndices, normalizedGroupPath.indices) &&
    Number.isInteger(sourceIndex) &&
    sourceIndex < nextChildIndex
  ) {
    nextChildIndex -= 1;
  }
  const withoutSource = removeSearchExpressionNode(normalizedExpression, normalizedSourcePath.path);
  const adjustedGroupIndices = adjustPathIndicesAfterRemoval(
    normalizedGroupPath.indices,
    normalizedSourcePath.indices,
  );
  const adjustedGroup = resolveSearchExpressionNode(
    withoutSource,
    searchExpressionPathFromIndices(adjustedGroupIndices),
  );
  const maxChildIndex =
    adjustedGroup?.type === "group" ? adjustedGroup.children.length : 0;
  nextChildIndex = Math.min(nextChildIndex, maxChildIndex);
  const nextExpression = visitSearchExpression(withoutSource, adjustedGroupIndices, (node) => {
    if (node?.type !== "group") {
      return cloneSearchExpressionNode(node);
    }
    const children = node.children.map((child) => cloneSearchExpressionNode(child));
    children.splice(nextChildIndex, 0, cloneSearchExpressionNode(sourceNode));
    return {
      type: "group",
      operator: normalizeSearchExpressionOperator(node.operator),
      children,
    };
  });
  return compactSearchExpressionNode(nextExpression, { isRoot: true }) || buildSearchExpressionFromSelectedTerms([]);
}

export function groupSearchExpressionTerms(expression, sourcePath, targetPath, options = {}) {
  const normalizedExpression = resolveSearchExpression(expression);
  const normalizedSourcePath = normalizeSearchExpressionPath(sourcePath);
  const normalizedTargetPath = normalizeSearchExpressionPath(targetPath);
  const sourceNode = resolveSearchExpressionNode(normalizedExpression, normalizedSourcePath?.path);
  const targetNode = resolveSearchExpressionNode(normalizedExpression, normalizedTargetPath?.path);
  if (sourceNode?.type !== "term" || targetNode?.type !== "term") {
    return normalizedExpression;
  }
  return groupSearchExpressionNodes(normalizedExpression, sourcePath, targetPath, options);
}

export function groupSearchExpressionNodes(expression, sourcePath, targetPath, options = {}) {
  const normalizedExpression = resolveSearchExpression(expression);
  const normalizedSourcePath = normalizeSearchExpressionPath(sourcePath);
  const normalizedTargetPath = normalizeSearchExpressionPath(targetPath);
  if (
    !normalizedSourcePath ||
    !normalizedTargetPath ||
    normalizedSourcePath.indices.length === 0 ||
    normalizedTargetPath.indices.length === 0 ||
    normalizedSourcePath.path === normalizedTargetPath.path ||
    isSearchExpressionPathPrefix(normalizedSourcePath.indices, normalizedTargetPath.indices) ||
    isSearchExpressionPathPrefix(normalizedTargetPath.indices, normalizedSourcePath.indices)
  ) {
    return normalizedExpression;
  }
  const sourceNode = resolveSearchExpressionNode(normalizedExpression, normalizedSourcePath.path);
  const targetNode = resolveSearchExpressionNode(normalizedExpression, normalizedTargetPath.path);
  if (!sourceNode || !targetNode) {
    return normalizedExpression;
  }
  const withoutSource = removeSearchExpressionNode(normalizedExpression, normalizedSourcePath.path);
  const adjustedTargetIndices = adjustPathIndicesAfterRemoval(
    normalizedTargetPath.indices,
    normalizedSourcePath.indices,
  );
  const groupOperator = normalizeSearchExpressionOperator(options.operator ?? "and");
  const nextExpression = visitSearchExpression(withoutSource, adjustedTargetIndices, (node) => {
    return {
      type: "group",
      operator: groupOperator,
      children: [
        cloneSearchExpressionNode(node),
        cloneSearchExpressionNode(sourceNode),
      ],
    };
  });
  return compactSearchExpressionNode(nextExpression, { isRoot: true }) || buildSearchExpressionFromSelectedTerms([]);
}

export function selectedSearchTermsFromLegacyFilters(filters) {
  const source = isPlainObject(filters) ? filters : {};
  const terms = [];
  for (const term of normalizeFishFilterTerms(source.fishFilterTerms)) {
    terms.push({ kind: "fish-filter", term });
  }
  for (const fishId of normalizeIntegerList(source.fishIds)) {
    terms.push({ kind: "fish", fishId });
  }
  for (const zoneRgb of normalizeIntegerList(source.zoneRgbs)) {
    terms.push({ kind: "zone", zoneRgb });
  }
  const byLayer = isPlainObject(source.semanticFieldIdsByLayer)
    ? source.semanticFieldIdsByLayer
    : {};
  for (const zoneRgb of normalizeIntegerList(byLayer.zone_mask)) {
    terms.push({ kind: "zone", zoneRgb });
  }
  for (const [layerIdRaw, fieldIdsRaw] of Object.entries(byLayer)) {
    const layerId = String(layerIdRaw ?? "").trim();
    if (!layerId || layerId === "zone_mask") {
      continue;
    }
    for (const fieldId of normalizeIntegerList(fieldIdsRaw)) {
      terms.push({ kind: "semantic", layerId, fieldId });
    }
  }
  return normalizeSelectedSearchTerms(terms);
}

export function resolveSearchExpression(expression, selectedTerms = undefined, legacyFilters = null) {
  if (expression !== undefined) {
    return normalizeSearchExpression(expression) || buildSearchExpressionFromSelectedTerms([]);
  }
  return buildSearchExpressionFromSelectedTerms(
    resolveSelectedSearchTerms(selectedTerms, legacyFilters),
  );
}

export function resolveSelectedSearchTerms(value, legacyFilters = null, expression = undefined) {
  if (expression !== undefined) {
    return selectedSearchTermsFromExpression(resolveSearchExpression(expression));
  }
  const selectedTerms = normalizeSelectedSearchTerms(value);
  if (selectedTerms.length || Array.isArray(value)) {
    return selectedTerms;
  }
  return selectedSearchTermsFromLegacyFilters(legacyFilters);
}

export function projectSelectedSearchTermsToBridgedFilters(terms) {
  const selectedTerms = normalizeSelectedSearchTerms(terms);
  const fishIds = [];
  const zoneRgbs = [];
  const fishFilterTerms = [];
  const semanticFieldIdsByLayer = {};

  for (const term of selectedTerms) {
    if (term.kind === "fish") {
      fishIds.push(term.fishId);
      continue;
    }
    if (term.kind === "zone") {
      zoneRgbs.push(term.zoneRgb);
      continue;
    }
    if (term.kind === "fish-filter") {
      fishFilterTerms.push(term.term);
      continue;
    }
    if (term.kind === "semantic") {
      const fieldIds = semanticFieldIdsByLayer[term.layerId] || [];
      fieldIds.push(term.fieldId);
      semanticFieldIdsByLayer[term.layerId] = fieldIds;
    }
  }

  if (zoneRgbs.length) {
    semanticFieldIdsByLayer.zone_mask = zoneRgbs.slice();
  }

  return {
    fishIds,
    zoneRgbs,
    semanticFieldIdsByLayer,
    fishFilterTerms,
  };
}

export function addSelectedSearchTerm(selectedTerms, term) {
  return normalizeSelectedSearchTerms(
    normalizeSelectedSearchTerms(selectedTerms).concat(term),
  );
}

export function removeSelectedSearchTerm(selectedTerms, target) {
  const normalizedTarget = normalizeSearchTerm(target);
  if (!normalizedTarget) {
    return normalizeSelectedSearchTerms(selectedTerms);
  }
  const targetKey = searchTermKey(normalizedTarget);
  return normalizeSelectedSearchTerms(selectedTerms).filter(
    (term) => searchTermKey(term) !== targetKey,
  );
}

export function buildSearchSelectionStatePatch(selectedTerms, searchPatch = null) {
  return buildSearchExpressionStatePatch(
    buildSearchExpressionFromSelectedTerms(selectedTerms),
    searchPatch,
  );
}

export function buildSearchExpressionStatePatch(expression, searchPatch = null) {
  const normalizedExpression = resolveSearchExpression(expression);
  const normalizedTerms = selectedSearchTermsFromExpression(normalizedExpression);
  const projection = projectSelectedSearchTermsToBridgedFilters(normalizedTerms);
  const patch = {
    _map_ui: {
      search: {
        expression: normalizedExpression,
        selectedTerms: normalizedTerms,
      },
    },
    _map_bridged: {
      filters: {
        fishIds: projection.fishIds,
        zoneRgbs: projection.zoneRgbs,
        semanticFieldIdsByLayer: projection.semanticFieldIdsByLayer,
        fishFilterTerms: projection.fishFilterTerms,
      },
    },
  };
  if (isPlainObject(searchPatch)) {
    patch._map_ui.search = {
      ...patch._map_ui.search,
      ...searchPatch,
    };
  }
  return patch;
}

export function layerSupportsSearchTerm(layerId, termKind) {
  const normalizedLayerId = String(layerId ?? "").trim();
  const normalizedTermKind = String(termKind ?? "").trim();
  const layerSupport = MAP_SEARCH_LAYER_SUPPORT[normalizedLayerId];
  return !!layerSupport?.termKinds?.includes(normalizedTermKind);
}

export function layerSupportsAttachmentClipMode(layerId, clipMode) {
  const normalizedLayerId = String(layerId ?? "").trim();
  const normalizedClipMode = String(clipMode ?? "").trim();
  const layerSupport = MAP_SEARCH_LAYER_SUPPORT[normalizedLayerId];
  return !!layerSupport?.attachmentClipModes?.includes(normalizedClipMode);
}
