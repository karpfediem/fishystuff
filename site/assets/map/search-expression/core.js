import {
  DEFAULT_SEARCH_EXPRESSION_OPERATOR,
} from "./constants.js";
import { isPlainObject } from "./shared.js";
import {
  normalizeSearchTerm,
  normalizeSelectedSearchTerms,
  searchTermKey,
} from "./terms.js";

export function normalizeSearchExpressionOperator(value) {
  return String(value ?? "").trim().toLowerCase() === "and"
    ? "and"
    : DEFAULT_SEARCH_EXPRESSION_OPERATOR;
}

export function normalizeSearchExpressionNegated(value) {
  return value === true;
}

export function cloneSearchExpressionNode(node) {
  if (!isPlainObject(node)) {
    return null;
  }
  if (node.type === "term") {
    const cloned = {
      type: "term",
      term: normalizeSearchTerm(node.term),
    };
    if (normalizeSearchExpressionNegated(node.negated)) {
      cloned.negated = true;
    }
    return cloned;
  }
  if (node.type === "group") {
    const cloned = {
      type: "group",
      operator: normalizeSearchExpressionOperator(node.operator),
      children: (Array.isArray(node.children) ? node.children : [])
        .map((child) => cloneSearchExpressionNode(child))
        .filter(Boolean),
    };
    if (normalizeSearchExpressionNegated(node.negated)) {
      cloned.negated = true;
    }
    return cloned;
  }
  return null;
}

export function normalizeSearchExpressionPath(path) {
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

export function searchExpressionPathFromIndices(indices) {
  return Array.isArray(indices) && indices.length ? `root.${indices.join(".")}` : "root";
}

export function mergeSearchExpressionNodeNegation(node, negated) {
  const clonedNode = cloneSearchExpressionNode(node);
  if (!clonedNode) {
    return null;
  }
  const nextNegated = normalizeSearchExpressionNegated(clonedNode.negated)
    !== normalizeSearchExpressionNegated(negated);
  if (nextNegated) {
    clonedNode.negated = true;
  } else {
    delete clonedNode.negated;
  }
  return clonedNode;
}

export function compactSearchExpressionNode(node, { isRoot = false } = {}) {
  const clonedNode = cloneSearchExpressionNode(node);
  if (!clonedNode) {
    return null;
  }
  if (clonedNode.type === "term") {
    return clonedNode.term ? clonedNode : null;
  }
  const operator = normalizeSearchExpressionOperator(clonedNode.operator);
  const children = clonedNode.children
    .map((child) => compactSearchExpressionNode(child))
    .filter(Boolean);
  if (!isRoot && children.length === 0) {
    return null;
  }
  if (!isRoot && children.length === 1) {
    return mergeSearchExpressionNodeNegation(children[0], clonedNode.negated);
  }
  const compacted = {
    type: "group",
    operator,
    children,
  };
  if (normalizeSearchExpressionNegated(clonedNode.negated)) {
    compacted.negated = true;
  }
  return compacted;
}

export function visitSearchExpression(node, indices, visitor, currentIndices = []) {
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
  const nextNode = {
    type: "group",
    operator: normalizeSearchExpressionOperator(node.operator),
    children: children.filter(Boolean),
  };
  if (normalizeSearchExpressionNegated(node.negated)) {
    nextNode.negated = true;
  }
  return nextNode;
}

export function adjustPathIndicesAfterRemoval(targetIndices, sourceIndices) {
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

export function isSearchExpressionPathPrefix(prefixIndices, candidateIndices) {
  if (!Array.isArray(prefixIndices) || !Array.isArray(candidateIndices)) {
    return false;
  }
  if (prefixIndices.length > candidateIndices.length) {
    return false;
  }
  return prefixIndices.every((value, index) => candidateIndices[index] === value);
}

export function searchExpressionIndicesEqual(leftIndices, rightIndices) {
  if (!Array.isArray(leftIndices) || !Array.isArray(rightIndices)) {
    return false;
  }
  if (leftIndices.length !== rightIndices.length) {
    return false;
  }
  return leftIndices.every((value, index) => rightIndices[index] === value);
}

export function clampSearchExpressionChildIndex(value, fallback) {
  const normalized = Number.parseInt(value, 10);
  if (!Number.isInteger(normalized)) {
    return fallback;
  }
  return normalized < 0 ? 0 : normalized;
}

export function buildSearchExpressionSliceNode(operator, children) {
  const normalizedChildren = (Array.isArray(children) ? children : [])
    .map((child) => cloneSearchExpressionNode(child))
    .filter(Boolean);
  if (normalizedChildren.length === 0) {
    return null;
  }
  if (normalizedChildren.length === 1) {
    return normalizedChildren[0];
  }
  return {
    type: "group",
    operator: normalizeSearchExpressionOperator(operator),
    children: normalizedChildren,
  };
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
  const negationPrefix = normalizeSearchExpressionNegated(node.negated) ? "not:" : "";
  if (node.type === "term") {
    return `term:${negationPrefix}${searchTermKey(node.term)}`;
  }
  if (node.type === "group") {
    return `group:${negationPrefix}${normalizeSearchExpressionOperator(node.operator)}:${(
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
    if (!term) {
      return null;
    }
    const normalized = { type: "term", term };
    if (normalizeSearchExpressionNegated(raw.negated ?? raw.not ?? raw.inverted)) {
      normalized.negated = true;
    }
    return normalized;
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
    const normalized = {
      type: "group",
      operator: normalizeSearchExpressionOperator(raw.operator ?? raw.op),
      children,
    };
    if (normalizeSearchExpressionNegated(raw.negated ?? raw.not ?? raw.inverted)) {
      normalized.negated = true;
    }
    return normalized;
  }

  const term = normalizeSearchTerm(raw);
  if (!term) {
    return null;
  }
  const normalized = { type: "term", term };
  if (normalizeSearchExpressionNegated(raw.negated ?? raw.not ?? raw.inverted)) {
    normalized.negated = true;
  }
  return normalized;
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

export function coerceSearchExpression(expression) {
  return normalizeSearchExpression(expression) || buildSearchExpressionFromSelectedTerms([]);
}

export function selectedSearchTermsFromExpression(expression) {
  const normalizedExpression = coerceSearchExpression(expression);
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
  const normalizedExpression = coerceSearchExpression(expression);
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
  const normalizedExpression = coerceSearchExpression(expression);
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
