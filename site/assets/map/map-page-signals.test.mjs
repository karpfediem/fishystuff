import test from "node:test";
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
});
