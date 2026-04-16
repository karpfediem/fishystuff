import test from "node:test";
import assert from "node:assert/strict";

import {
  buildSearchProjectionSignalPatch,
  resolveSearchProjection,
} from "./map-search-projection.js";

test("resolveSearchProjection derives bridged filters from selected terms", () => {
  assert.deepEqual(
    resolveSearchProjection({
      _map_ui: {
        search: {
          selectedTerms: [
            { kind: "zone", zoneRgb: 123 },
            { kind: "semantic", layerId: "regions", fieldId: 430 },
            { kind: "fish-filter", term: "favorite" },
            { kind: "patch-bound", bound: "from", patchId: "2026-02-26" },
            { kind: "patch-bound", bound: "to", patchId: "2026-03-12" },
          ],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [912],
        },
      },
    }),
    {
      fishIds: [],
      zoneRgbs: [123],
      semanticFieldIdsByLayer: {
        regions: [430],
        zone_mask: [123],
      },
      fishFilterTerms: ["favourite"],
      patchId: null,
      fromPatchId: "2026-02-26",
      toPatchId: "2026-03-12",
      searchExpression: {
        type: "group",
        operator: "or",
        children: [
          { type: "term", term: { kind: "zone", zoneRgb: 123 } },
          { type: "term", term: { kind: "semantic", layerId: "regions", fieldId: 430 } },
          { type: "term", term: { kind: "fish-filter", term: "favourite" } },
          { type: "term", term: { kind: "patch-bound", bound: "from", patchId: "2026-02-26" } },
          { type: "term", term: { kind: "patch-bound", bound: "to", patchId: "2026-03-12" } },
        ],
      },
    },
  );
});

test("buildSearchProjectionSignalPatch clears stale bridged search filters", () => {
  assert.deepEqual(
    buildSearchProjectionSignalPatch({
      _map_ui: {
        search: {
          selectedTerms: [],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [912],
          zoneRgbs: [123],
          semanticFieldIdsByLayer: { regions: [430], zone_mask: [123] },
          fishFilterTerms: ["missing"],
        },
      },
    }),
    {
      _map_bridged: {
        filters: {
          fishIds: [],
          zoneRgbs: [],
          semanticFieldIdsByLayer: {},
          fishFilterTerms: [],
          patchId: null,
          fromPatchId: null,
          toPatchId: null,
          searchExpression: {
            type: "group",
            operator: "or",
            children: [],
          },
        },
      },
    },
  );
});

test("buildSearchProjectionSignalPatch fills in a missing bridged search expression", () => {
  assert.deepEqual(
    buildSearchProjectionSignalPatch({
      _map_ui: {
        search: {
          selectedTerms: [{ kind: "fish", fishId: 912 }],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [912],
          zoneRgbs: [],
          semanticFieldIdsByLayer: {},
          fishFilterTerms: [],
        },
      },
    }),
    {
      _map_bridged: {
        filters: {
          fishIds: [912],
          zoneRgbs: [],
          semanticFieldIdsByLayer: {},
          fishFilterTerms: [],
          patchId: null,
          fromPatchId: null,
          toPatchId: null,
          searchExpression: {
            type: "group",
            operator: "or",
            children: [{ type: "term", term: { kind: "fish", fishId: 912 } }],
          },
        },
      },
    },
  );
});
