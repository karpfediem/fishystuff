import {
  adjustPathIndicesAfterRemoval,
  buildSearchExpressionFromSelectedTerms,
  buildSearchExpressionSliceNode,
  clampSearchExpressionChildIndex,
  cloneSearchExpressionNode,
  coerceSearchExpression,
  compactSearchExpressionNode,
  findSearchExpressionTermPath,
  isSearchExpressionPathPrefix,
  mergeSearchExpressionNodeNegation,
  normalizeSearchExpressionNegated,
  normalizeSearchExpressionOperator,
  normalizeSearchExpressionPath,
  resolveSearchExpressionNode,
  searchExpressionIndicesEqual,
  searchExpressionPathFromIndices,
  visitSearchExpression,
} from "./core.js";
import {
  normalizeSearchTerm,
  searchTermKey,
} from "./terms.js";

function removeSearchExpressionNodeFromTree(expression, path) {
  const normalizedExpression = coerceSearchExpression(expression);
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
        ...(normalizeSearchExpressionNegated(node.negated) ? { negated: true } : {}),
      };
    },
  );
  return nextExpression;
}

export function appendSearchExpressionTerm(expression, term, options = {}) {
  const normalizedExpression = coerceSearchExpression(expression);
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
      ...(normalizeSearchExpressionNegated(node.negated) ? { negated: true } : {}),
    };
  });
}

export function setSearchExpressionGroupOperator(expression, path, operator) {
  const normalizedExpression = coerceSearchExpression(expression);
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
        ...(normalizeSearchExpressionNegated(node.negated) ? { negated: true } : {}),
      };
    },
  );
  return compactSearchExpressionNode(nextExpression, { isRoot: true }) || buildSearchExpressionFromSelectedTerms([]);
}

export function setSearchExpressionBoundaryOperator(expression, path, boundaryIndex, operator) {
  const normalizedExpression = coerceSearchExpression(expression);
  const normalizedPath = normalizeSearchExpressionPath(path);
  if (!normalizedPath) {
    return normalizedExpression;
  }
  const normalizedOperator = normalizeSearchExpressionOperator(operator);
  const targetNode = resolveSearchExpressionNode(normalizedExpression, normalizedPath.path);
  const splitIndex = Number.parseInt(boundaryIndex, 10);
  if (
    targetNode?.type !== "group" ||
    !Number.isInteger(splitIndex) ||
    splitIndex <= 0 ||
    splitIndex >= targetNode.children.length ||
    normalizeSearchExpressionOperator(targetNode.operator) === normalizedOperator
  ) {
    return normalizedExpression;
  }
  const nextExpression = visitSearchExpression(
    normalizedExpression,
    normalizedPath.indices,
    (node) => {
      if (node?.type !== "group") {
        return cloneSearchExpressionNode(node);
      }
      const leftNode = buildSearchExpressionSliceNode(node.operator, node.children.slice(0, splitIndex));
      const rightNode = buildSearchExpressionSliceNode(node.operator, node.children.slice(splitIndex));
      if (!leftNode || !rightNode) {
        return cloneSearchExpressionNode(node);
      }
      return {
        type: "group",
        operator: normalizedOperator,
        children: [leftNode, rightNode],
        ...(normalizeSearchExpressionNegated(node.negated) ? { negated: true } : {}),
      };
    },
  );
  return compactSearchExpressionNode(nextExpression, { isRoot: true }) || buildSearchExpressionFromSelectedTerms([]);
}

export function toggleSearchExpressionNodeNegated(expression, path) {
  const normalizedExpression = coerceSearchExpression(expression);
  const normalizedPath = normalizeSearchExpressionPath(path);
  if (!normalizedPath) {
    return normalizedExpression;
  }
  const nextExpression = visitSearchExpression(
    normalizedExpression,
    normalizedPath.indices,
    (node) => mergeSearchExpressionNodeNegation(node, true),
  );
  return compactSearchExpressionNode(nextExpression, { isRoot: true }) || buildSearchExpressionFromSelectedTerms([]);
}

export function replaceSearchExpressionTerm(expression, path, term) {
  const normalizedExpression = coerceSearchExpression(expression);
  const normalizedPath = normalizeSearchExpressionPath(path);
  const normalizedTerm = normalizeSearchTerm(term);
  if (!normalizedPath || !normalizedTerm) {
    return normalizedExpression;
  }
  const nextExpression = visitSearchExpression(
    normalizedExpression,
    normalizedPath.indices,
    (node) => {
      if (node?.type !== "term") {
        return cloneSearchExpressionNode(node);
      }
      return {
        type: "term",
        term: normalizedTerm,
        ...(normalizeSearchExpressionNegated(node.negated) ? { negated: true } : {}),
      };
    },
  );
  return compactSearchExpressionNode(nextExpression, { isRoot: true }) || buildSearchExpressionFromSelectedTerms([]);
}

export function removeSearchExpressionNode(expression, path) {
  const normalizedExpression = coerceSearchExpression(expression);
  const normalizedPath = normalizeSearchExpressionPath(path);
  if (!normalizedPath || normalizedPath.indices.length === 0) {
    return normalizedExpression;
  }
  const nextExpression = removeSearchExpressionNodeFromTree(
    normalizedExpression,
    normalizedPath.path,
  );
  return compactSearchExpressionNode(nextExpression, { isRoot: true }) || buildSearchExpressionFromSelectedTerms([]);
}

export function moveSearchExpressionTermToGroup(expression, sourcePath, groupPath) {
  const normalizedExpression = coerceSearchExpression(expression);
  const normalizedSourcePath = normalizeSearchExpressionPath(sourcePath);
  const sourceNode = resolveSearchExpressionNode(normalizedExpression, normalizedSourcePath?.path);
  if (sourceNode?.type !== "term") {
    return normalizedExpression;
  }
  return moveSearchExpressionNodeToGroup(normalizedExpression, sourcePath, groupPath);
}

export function moveSearchExpressionNodeToGroup(expression, sourcePath, groupPath) {
  const normalizedExpression = coerceSearchExpression(expression);
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
  const normalizedExpression = coerceSearchExpression(expression);
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
  const withoutSource = removeSearchExpressionNodeFromTree(
    normalizedExpression,
    normalizedSourcePath.path,
  );
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
      ...(normalizeSearchExpressionNegated(node.negated) ? { negated: true } : {}),
    };
  });
  return compactSearchExpressionNode(nextExpression, { isRoot: true }) || buildSearchExpressionFromSelectedTerms([]);
}

export function groupSearchExpressionTerms(expression, sourcePath, targetPath, options = {}) {
  const normalizedExpression = coerceSearchExpression(expression);
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
  const normalizedExpression = coerceSearchExpression(expression);
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
