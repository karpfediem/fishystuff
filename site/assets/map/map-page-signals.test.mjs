import { test } from "bun:test";
import assert from "node:assert/strict";

import {
  applyMapPageSignalsPatch,
  patchMatchesMapPagePersistFilter,
} from "./map-page-signals.js";

test("map-page-signals applies exact replacement branches", () => {
  const signals = {
    _map_ui: {
      layers: {
        hoverFactsVisibleByLayer: {
          regions: { origin_region: true },
          region_groups: { resource_group: true },
        },
      },
    },
  };

  applyMapPageSignalsPatch(signals, {
    _map_ui: {
      layers: {
        hoverFactsVisibleByLayer: {
          zone_mask: { zone_name: true },
        },
      },
    },
  });

  assert.deepEqual(signals._map_ui.layers.hoverFactsVisibleByLayer, {
    zone_mask: { zone_name: true },
  });
});

test("map-page-signals replaces runtime effective filters atomically", () => {
  const signals = {
    _map_runtime: {
      effectiveFilters: {
        searchExpression: { type: "group", operator: "or", children: [] },
        sharedFishState: { caughtIds: [77], favouriteIds: [] },
        zoneMembershipByLayer: {
          fish_evidence: { active: true, zoneRgbs: [0x39e58d], revision: 4 },
        },
        semanticFieldFiltersByLayer: {},
      },
    },
  };

  applyMapPageSignalsPatch(signals, {
    _map_runtime: {
      effectiveFilters: {
        searchExpression: { type: "group", operator: "or", children: [] },
        sharedFishState: { caughtIds: [], favouriteIds: [912] },
        zoneMembershipByLayer: {},
        semanticFieldFiltersByLayer: {
          regions: { active: true, fieldIds: [12], revision: 7 },
        },
      },
    },
  });

  assert.deepEqual(signals._map_runtime.effectiveFilters, {
    searchExpression: { type: "group", operator: "or", children: [] },
    sharedFishState: { caughtIds: [], favouriteIds: [912] },
    zoneMembershipByLayer: {},
    semanticFieldFiltersByLayer: {
      regions: { active: true, fieldIds: [12], revision: 7 },
    },
  });
});

test("map-page-signals persists only durable map branches", () => {
  assert.equal(
    patchMatchesMapPagePersistFilter({
      _map_ui: {
        search: {
          query: "eel",
        },
      },
    }),
    true,
  );
  assert.equal(
    patchMatchesMapPagePersistFilter({
      _map_runtime: {
        ready: true,
      },
    }),
    false,
  );
  assert.equal(
    patchMatchesMapPagePersistFilter({
      _map_bridged: {
        filters: {
          fishIds: [77],
        },
      },
    }),
    false,
  );
  assert.equal(
    patchMatchesMapPagePersistFilter({
      _map_bridged: {
        filters: {
          layerFilterBindingIdsDisabledByLayer: {
            fish_evidence: ["zone_mask"],
          },
        },
      },
    }),
    true,
  );
});
