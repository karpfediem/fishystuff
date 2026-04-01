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
        },
      },
    },
  );
});

test("buildSearchProjectionSignalPatch returns null when bridged filters already match", () => {
  assert.equal(
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
    null,
  );
});
