import test from "node:test";
import assert from "node:assert/strict";

import {
  appendSearchExpressionTerm,
  addSelectedSearchTerm,
  buildSearchExpressionFromSelectedTerms,
  buildSearchSelectionStatePatch,
  findSearchExpressionTermPath,
  groupSearchExpressionNodes,
  groupSearchExpressionTerms,
  layerSupportsAttachmentClipMode,
  layerSupportsSearchTerm,
  moveSearchExpressionNodeToIndex,
  moveSearchExpressionNodeToGroup,
  moveSearchExpressionTermToGroup,
  normalizeFishFilterTerms,
  normalizeSearchExpression,
  normalizeSelectedSearchTerms,
  projectSelectedSearchTermsToBridgedFilters,
  removeSearchExpressionNode,
  resolveSearchExpressionNode,
  selectedSearchTermsFromExpression,
  setSearchExpressionGroupOperator,
  resolveSelectedSearchTerms,
} from "./map-search-contract.js";

test("normalizeSelectedSearchTerms canonicalizes aliases and deduplicates term kinds", () => {
  assert.deepEqual(
    normalizeSelectedSearchTerms([
      { kind: "fish-filter", term: "favorite" },
      { kind: "fish-filter", term: "rare" },
      { kind: "fish-filter", term: "favourites" },
      { kind: "fish", fishId: "912" },
      { kind: "semantic", layerId: "zone_mask", fieldId: 123 },
      { kind: "zone", zoneRgb: 123 },
      { kind: "semantic", layerId: "regions", fieldId: 22 },
    ]),
    [
      { kind: "fish-filter", term: "favourite" },
      { kind: "fish-filter", term: "yellow" },
      { kind: "fish", fishId: 912 },
      { kind: "zone", zoneRgb: 123 },
      { kind: "semantic", layerId: "regions", fieldId: 22 },
    ],
  );
});

test("resolveSelectedSearchTerms falls back to legacy bridged filters", () => {
  assert.deepEqual(
    resolveSelectedSearchTerms(undefined, {
      fishIds: [912],
      zoneRgbs: [123],
      fishFilterTerms: ["missing"],
      semanticFieldIdsByLayer: {
        regions: [22],
      },
    }),
    [
      { kind: "fish-filter", term: "missing" },
      { kind: "fish", fishId: 912 },
      { kind: "zone", zoneRgb: 123 },
      { kind: "semantic", layerId: "regions", fieldId: 22 },
    ],
  );
});

test("projectSelectedSearchTermsToBridgedFilters derives explicit runtime filters", () => {
  assert.deepEqual(
    projectSelectedSearchTermsToBridgedFilters([
      { kind: "fish-filter", term: "missing" },
      { kind: "fish", fishId: 912 },
      { kind: "zone", zoneRgb: 123 },
      { kind: "semantic", layerId: "regions", fieldId: 22 },
    ]),
    {
      fishIds: [912],
      zoneRgbs: [123],
      semanticFieldIdsByLayer: {
        regions: [22],
        zone_mask: [123],
      },
      fishFilterTerms: ["missing"],
    },
  );
});

test("buildSearchSelectionStatePatch keeps selected terms page-owned and projects bridged filters", () => {
  assert.deepEqual(
    buildSearchSelectionStatePatch(
      [
        { kind: "fish", fishId: 912 },
        { kind: "zone", zoneRgb: 123 },
      ],
      { query: "", open: false },
    ),
    {
      _map_ui: {
        search: {
          expression: {
            type: "group",
            operator: "or",
            children: [
              {
                type: "term",
                term: { kind: "fish", fishId: 912 },
              },
              {
                type: "term",
                term: { kind: "zone", zoneRgb: 123 },
              },
            ],
          },
          selectedTerms: [
            { kind: "fish", fishId: 912 },
            { kind: "zone", zoneRgb: 123 },
          ],
          query: "",
          open: false,
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [912],
          zoneRgbs: [123],
          semanticFieldIdsByLayer: { zone_mask: [123] },
          fishFilterTerms: [],
        },
      },
    },
  );
});

test("normalizeSearchExpression canonicalizes grouped terms and deduplicates leaves", () => {
  assert.deepEqual(
    normalizeSearchExpression({
      type: "group",
      operator: "and",
      children: [
        { kind: "fish-filter", term: "favorite" },
        { type: "term", term: { kind: "fish", fishId: "912" } },
        { kind: "fish-filter", term: "favourites" },
      ],
    }),
    {
      type: "group",
      operator: "and",
      children: [
        {
          type: "term",
          term: { kind: "fish-filter", term: "favourite" },
        },
        {
          type: "term",
          term: { kind: "fish", fishId: 912 },
        },
      ],
    },
  );
});

test("selectedSearchTermsFromExpression preserves first-seen term order across nested groups", () => {
  assert.deepEqual(
    selectedSearchTermsFromExpression({
      type: "group",
      operator: "or",
      children: [
        {
          type: "group",
          operator: "and",
          children: [
            { kind: "fish-filter", term: "missing" },
            { kind: "fish", fishId: 912 },
          ],
        },
        { kind: "fish-filter", term: "uncaught" },
        { kind: "zone", zoneRgb: 123 },
      ],
    }),
    [
      { kind: "fish-filter", term: "missing" },
      { kind: "fish", fishId: 912 },
      { kind: "zone", zoneRgb: 123 },
    ],
  );
});

test("buildSearchExpressionFromSelectedTerms lifts flat selections into an or-group", () => {
  assert.deepEqual(
    buildSearchExpressionFromSelectedTerms([
      { kind: "fish", fishId: 912 },
      { kind: "fish-filter", term: "missing" },
    ]),
    {
      type: "group",
      operator: "or",
      children: [
        {
          type: "term",
          term: { kind: "fish", fishId: 912 },
        },
        {
          type: "term",
          term: { kind: "fish-filter", term: "missing" },
        },
      ],
    },
  );
});

test("appendSearchExpressionTerm preserves existing groups and avoids duplicate terms", () => {
  const expression = {
    type: "group",
    operator: "or",
    children: [
      {
        type: "group",
        operator: "and",
        children: [{ type: "term", term: { kind: "fish-filter", term: "favourite" } }],
      },
    ],
  };

  assert.deepEqual(
    appendSearchExpressionTerm(expression, { kind: "fish", fishId: 912 }),
    {
      type: "group",
      operator: "or",
      children: [
        {
          type: "group",
          operator: "and",
          children: [{ type: "term", term: { kind: "fish-filter", term: "favourite" } }],
        },
        {
          type: "term",
          term: { kind: "fish", fishId: 912 },
        },
      ],
    },
  );
  assert.deepEqual(
    appendSearchExpressionTerm(expression, { kind: "fish-filter", term: "favorite" }),
    expression,
  );
});

test("removeSearchExpressionNode removes leaf paths and dissolves one-child groups", () => {
  assert.deepEqual(
    removeSearchExpressionNode(
      {
        type: "group",
        operator: "or",
        children: [
          {
            type: "group",
            operator: "and",
            children: [
              { type: "term", term: { kind: "fish-filter", term: "favourite" } },
              { type: "term", term: { kind: "fish", fishId: 912 } },
            ],
          },
          { type: "term", term: { kind: "zone", zoneRgb: 123 } },
        ],
      },
      "root.0.1",
    ),
    {
      type: "group",
      operator: "or",
      children: [
        { type: "term", term: { kind: "fish-filter", term: "favourite" } },
        { type: "term", term: { kind: "zone", zoneRgb: 123 } },
      ],
    },
  );
});

test("setSearchExpressionGroupOperator merges a group into its parent when operators match", () => {
  const expression = {
    type: "group",
    operator: "or",
    children: [
      {
        type: "group",
        operator: "and",
        children: [
          { type: "term", term: { kind: "fish", fishId: 912 } },
          { type: "term", term: { kind: "zone", zoneRgb: 123 } },
        ],
      },
      { type: "term", term: { kind: "fish-filter", term: "favourite" } },
    ],
  };

  assert.deepEqual(setSearchExpressionGroupOperator(expression, "root.0", "or"), {
    type: "group",
    operator: "or",
    children: [
      { type: "term", term: { kind: "fish", fishId: 912 } },
      { type: "term", term: { kind: "zone", zoneRgb: 123 } },
      { type: "term", term: { kind: "fish-filter", term: "favourite" } },
    ],
  });
});

test("moveSearchExpressionTermToGroup moves a leaf into another group without flattening the tree", () => {
  assert.deepEqual(
    moveSearchExpressionTermToGroup(
      {
        type: "group",
        operator: "or",
        children: [
          { type: "term", term: { kind: "fish-filter", term: "favourite" } },
          {
            type: "group",
            operator: "and",
            children: [{ type: "term", term: { kind: "fish", fishId: 912 } }],
          },
        ],
      },
      "root.0",
      "root.1",
    ),
    {
      type: "group",
      operator: "or",
      children: [
        {
          type: "group",
          operator: "and",
          children: [
            { type: "term", term: { kind: "fish", fishId: 912 } },
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
          ],
        },
      ],
    },
  );
});

test("groupSearchExpressionTerms wraps source and target terms into a new subgroup", () => {
  assert.deepEqual(
    groupSearchExpressionTerms(
      {
        type: "group",
        operator: "or",
        children: [
          { type: "term", term: { kind: "fish-filter", term: "favourite" } },
          { type: "term", term: { kind: "fish", fishId: 912 } },
          { type: "term", term: { kind: "zone", zoneRgb: 123 } },
        ],
      },
      "root.0",
      "root.1",
    ),
    {
      type: "group",
      operator: "or",
      children: [
        {
          type: "group",
          operator: "and",
          children: [
            { type: "term", term: { kind: "fish", fishId: 912 } },
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
          ],
        },
        { type: "term", term: { kind: "zone", zoneRgb: 123 } },
      ],
    },
  );
});

test("moveSearchExpressionNodeToGroup moves a nested subgroup into another group", () => {
  assert.deepEqual(
    moveSearchExpressionNodeToGroup(
      {
        type: "group",
        operator: "or",
        children: [
          {
            type: "group",
            operator: "and",
            children: [
              { type: "term", term: { kind: "fish-filter", term: "favourite" } },
              { type: "term", term: { kind: "fish", fishId: 912 } },
            ],
          },
          {
            type: "group",
            operator: "or",
            children: [{ type: "term", term: { kind: "zone", zoneRgb: 123 } }],
          },
        ],
      },
      "root.0",
      "root.1",
    ),
    {
      type: "group",
      operator: "or",
      children: [
        { type: "term", term: { kind: "zone", zoneRgb: 123 } },
        {
          type: "group",
          operator: "and",
          children: [
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            { type: "term", term: { kind: "fish", fishId: 912 } },
          ],
        },
      ],
    },
  );
});

test("moveSearchExpressionNodeToIndex reorders a node within the same parent group", () => {
  assert.deepEqual(
    moveSearchExpressionNodeToIndex(
      {
        type: "group",
        operator: "or",
        children: [
          { type: "term", term: { kind: "fish-filter", term: "favourite" } },
          { type: "term", term: { kind: "fish", fishId: 912 } },
          { type: "term", term: { kind: "zone", zoneRgb: 123 } },
        ],
      },
      "root.0",
      "root",
      2,
    ),
    {
      type: "group",
      operator: "or",
      children: [
        { type: "term", term: { kind: "fish", fishId: 912 } },
        { type: "term", term: { kind: "fish-filter", term: "favourite" } },
        { type: "term", term: { kind: "zone", zoneRgb: 123 } },
      ],
    },
  );
});

test("moveSearchExpressionNodeToIndex inserts a subgroup at the requested child index", () => {
  assert.deepEqual(
    moveSearchExpressionNodeToIndex(
      {
        type: "group",
        operator: "or",
        children: [
          {
            type: "group",
            operator: "and",
            children: [
              { type: "term", term: { kind: "fish-filter", term: "favourite" } },
              { type: "term", term: { kind: "fish", fishId: 912 } },
            ],
          },
          {
            type: "group",
            operator: "or",
            children: [
              { type: "term", term: { kind: "zone", zoneRgb: 123 } },
              { type: "term", term: { kind: "semantic", layerId: "regions", fieldId: 22 } },
            ],
          },
        ],
      },
      "root.0",
      "root.1",
      1,
    ),
    {
      type: "group",
      operator: "or",
      children: [
        { type: "term", term: { kind: "zone", zoneRgb: 123 } },
        {
          type: "group",
          operator: "and",
          children: [
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            { type: "term", term: { kind: "fish", fishId: 912 } },
          ],
        },
        { type: "term", term: { kind: "semantic", layerId: "regions", fieldId: 22 } },
      ],
    },
  );
});

test("groupSearchExpressionNodes wraps a subgroup with another term into a new subgroup", () => {
  assert.deepEqual(
    groupSearchExpressionNodes(
      {
        type: "group",
        operator: "or",
        children: [
          {
            type: "group",
            operator: "and",
            children: [
              { type: "term", term: { kind: "fish-filter", term: "favourite" } },
              { type: "term", term: { kind: "fish", fishId: 912 } },
            ],
          },
          { type: "term", term: { kind: "zone", zoneRgb: 123 } },
          { type: "term", term: { kind: "semantic", layerId: "regions", fieldId: 22 } },
        ],
      },
      "root.0",
      "root.1",
      { operator: "or" },
    ),
    {
      type: "group",
      operator: "or",
      children: [
        { type: "term", term: { kind: "zone", zoneRgb: 123 } },
        {
          type: "group",
          operator: "and",
          children: [
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            { type: "term", term: { kind: "fish", fishId: 912 } },
          ],
        },
        { type: "term", term: { kind: "semantic", layerId: "regions", fieldId: 22 } },
      ],
    },
  );
});

test("groupSearchExpressionNodes rejects grouping a node with its own descendant", () => {
  const expression = {
    type: "group",
    operator: "or",
    children: [
      {
        type: "group",
        operator: "and",
        children: [
          { type: "term", term: { kind: "fish-filter", term: "favourite" } },
          { type: "term", term: { kind: "fish", fishId: 912 } },
        ],
      },
      { type: "term", term: { kind: "zone", zoneRgb: 123 } },
    ],
  };

  assert.deepEqual(groupSearchExpressionNodes(expression, "root.0", "root.0.1"), expression);
});

test("groupSearchExpressionNodes wraps a subgroup with a target subgroup handle", () => {
  assert.deepEqual(
    groupSearchExpressionNodes(
      {
        type: "group",
        operator: "or",
        children: [
          {
            type: "group",
            operator: "and",
            children: [{ type: "term", term: { kind: "fish-filter", term: "favourite" } }],
          },
          {
            type: "group",
            operator: "or",
            children: [{ type: "term", term: { kind: "zone", zoneRgb: 123 } }],
          },
        ],
      },
      "root.0",
      "root.1",
      { operator: "and" },
    ),
    {
      type: "group",
      operator: "or",
      children: [
        {
          type: "group",
          operator: "and",
          children: [
            { type: "term", term: { kind: "zone", zoneRgb: 123 } },
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
          ],
        },
      ],
    },
  );
});

test("resolveSearchExpressionNode and findSearchExpressionTermPath address nodes by root paths", () => {
  const expression = {
    type: "group",
    operator: "or",
    children: [
      {
        type: "group",
        operator: "and",
        children: [{ type: "term", term: { kind: "fish-filter", term: "favourite" } }],
      },
      { type: "term", term: { kind: "zone", zoneRgb: 123 } },
    ],
  };

  assert.deepEqual(resolveSearchExpressionNode(expression, "root.0"), {
    type: "group",
    operator: "and",
    children: [{ type: "term", term: { kind: "fish-filter", term: "favourite" } }],
  });
  assert.equal(
    findSearchExpressionTermPath(expression, { kind: "zone", zoneRgb: 123 }),
    "root.1",
  );
});

test("addSelectedSearchTerm keeps insertion order with deduped canonical keys", () => {
  assert.deepEqual(
    addSelectedSearchTerm([{ kind: "fish-filter", term: "missing" }], {
      kind: "fish-filter",
      term: "uncaught",
    }),
    [{ kind: "fish-filter", term: "missing" }],
  );
});

test("search layer support documents direct term and clip capabilities", () => {
  assert.equal(layerSupportsSearchTerm("zone_mask", "fish"), true);
  assert.equal(layerSupportsSearchTerm("regions", "fish"), false);
  assert.equal(layerSupportsAttachmentClipMode("regions", "mask-sample"), true);
  assert.equal(layerSupportsAttachmentClipMode("bookmarks", "mask-sample"), false);
});

test("normalizeFishFilterTerms preserves canonical term order", () => {
  assert.deepEqual(normalizeFishFilterTerms(["trash", "favorite", "rare", "blue"]), [
    "favourite",
    "yellow",
    "blue",
    "white",
  ]);
});
