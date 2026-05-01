import { test } from "bun:test";
import assert from "node:assert/strict";

import { parseQuerySignalPatch } from "./map-query-state.js";

test("parseQuerySignalPatch maps page-owned and bridged query params into signals", () => {
  const patch = parseQuerySignalPatch(
    "https://fishystuff.fish/map/?fish=91&fishTerms=missing,rare,blue&search=velia&fromPatch=2026-02-26&toPatch=2026-03-12&layers=zones,terrain&diagnostics=true&view=3d",
  );

  assert.deepEqual(patch, {
    _map_ui: {
      search: {
        query: "velia",
        open: true,
        expression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish", fishId: 91 } },
            { type: "term", term: { kind: "fish-filter", term: "missing" } },
            { type: "term", term: { kind: "fish-filter", term: "yellow" } },
            { type: "term", term: { kind: "fish-filter", term: "blue" } },
            { type: "term", term: { kind: "patch-bound", bound: "from", patchId: "2026-02-26" } },
            { type: "term", term: { kind: "patch-bound", bound: "to", patchId: "2026-03-12" } },
          ],
        },
        selectedTerms: [
          { kind: "fish", fishId: 91 },
          { kind: "fish-filter", term: "missing" },
          { kind: "fish-filter", term: "yellow" },
          { kind: "fish-filter", term: "blue" },
          { kind: "patch-bound", bound: "from", patchId: "2026-02-26" },
          { kind: "patch-bound", bound: "to", patchId: "2026-03-12" },
        ],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [91],
        zoneRgbs: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: ["missing", "yellow", "blue"],
        searchExpression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish", fishId: 91 } },
            { type: "term", term: { kind: "fish-filter", term: "missing" } },
            { type: "term", term: { kind: "fish-filter", term: "yellow" } },
            { type: "term", term: { kind: "fish-filter", term: "blue" } },
            { type: "term", term: { kind: "patch-bound", bound: "from", patchId: "2026-02-26" } },
            { type: "term", term: { kind: "patch-bound", bound: "to", patchId: "2026-03-12" } },
          ],
        },
        patchId: null,
        fromPatchId: "2026-02-26",
        toPatchId: "2026-03-12",
        layerIdsVisible: ["zones", "terrain"],
      },
      ui: {
        diagnosticsOpen: true,
        viewMode: "3d",
      },
    },
  });
});

test("parseQuerySignalPatch prefers focusFish and patch when present", () => {
  const patch = parseQuerySignalPatch(
    "https://fishystuff.fish/map/?focusFish=820986&patch=2026-02-26",
  );

  assert.deepEqual(patch, {
    _map_ui: {
      search: {
        expression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish", fishId: 820986 } },
            { type: "term", term: { kind: "patch-bound", bound: "from", patchId: "2026-02-26" } },
            { type: "term", term: { kind: "patch-bound", bound: "to", patchId: "2026-02-26" } },
          ],
        },
        selectedTerms: [
          { kind: "fish", fishId: 820986 },
          { kind: "patch-bound", bound: "from", patchId: "2026-02-26" },
          { kind: "patch-bound", bound: "to", patchId: "2026-02-26" },
        ],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [820986],
        zoneRgbs: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: [],
        searchExpression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish", fishId: 820986 } },
            { type: "term", term: { kind: "patch-bound", bound: "from", patchId: "2026-02-26" } },
            { type: "term", term: { kind: "patch-bound", bound: "to", patchId: "2026-02-26" } },
          ],
        },
        patchId: "2026-02-26",
        fromPatchId: "2026-02-26",
        toPatchId: "2026-02-26",
      },
    },
  });
});

test("parseQuerySignalPatch supports multiple fish selectors and defers fish-name resolution", () => {
  const patch = parseQuerySignalPatch(
    "https://fishystuff.fish/map/?fish=91,Pink%20Dolphin&fish=opah&fishTerms=favourite",
  );

  assert.deepEqual(patch, {
    _map_ui: {
      search: {
        expression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish", fishId: 91 } },
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
          ],
        },
        selectedTerms: [
          { kind: "fish", fishId: 91 },
          { kind: "fish-filter", term: "favourite" },
        ],
        pendingQueryFishSelectors: ["Pink Dolphin", "opah"],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [91],
        zoneRgbs: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: ["favourite"],
        patchId: null,
        fromPatchId: null,
        toPatchId: null,
        searchExpression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish", fishId: 91 } },
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
          ],
        },
      },
    },
  });
});

test("parseQuerySignalPatch supports NPC selectors", () => {
  assert.deepEqual(
    parseQuerySignalPatch("https://fishystuff.fish/map/?npc=chunsu,nampo&npc=missing"),
    {
      _map_ui: {
        search: {
          pendingQueryNpcSelectors: ["chunsu", "nampo", "missing"],
        },
      },
    },
  );
});

test("parseQuerySignalPatch returns null when there are no relevant params", () => {
  assert.equal(parseQuerySignalPatch("https://fishystuff.fish/map/"), null);
});

test("parseQuerySignalPatch keeps empty search query but does not auto-open search", () => {
  const patch = parseQuerySignalPatch("https://fishystuff.fish/map/?search=");

  assert.deepEqual(patch, {
    _map_ui: {
      search: {
        query: "",
      },
    },
  });
});
