import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const MAP_PAGE_SIGNALS_SOURCE = fs.readFileSync(
  new URL("./map-page-signals.js", import.meta.url),
  "utf8",
);

function loadHelper() {
  const context = {
    window: {},
    globalThis: null,
    JSON,
    Object,
    Array,
    String,
    RegExp,
  };
  context.globalThis = context;
  vm.runInNewContext(MAP_PAGE_SIGNALS_SOURCE, context, { filename: "map-page-signals.js" });
  return context.window.__fishystuffMapPageSignals;
}

test("map-page-signals applyPatchToSignals replaces exact patch paths", () => {
  const helper = loadHelper();
  const signals = {
    _map_runtime: {
      catalog: {
        layers: [{ layerId: "zone_mask" }, { layerId: "regions" }],
      },
    },
  };
  helper.applyPatchToSignals(signals, {
    _map_runtime: {
      catalog: {
        layers: [{ layerId: "regions" }],
      },
    },
  });
  assert.deepEqual(JSON.parse(JSON.stringify(signals._map_runtime.catalog.layers)), [
    { layerId: "regions" },
  ]);
});

test("map-page-signals patchMatchesPersistFilter ignores runtime-only patches", () => {
  const helper = loadHelper();
  assert.equal(
    helper.patchMatchesPersistFilter({
      _map_runtime: {
        ready: true,
      },
    }),
    false,
  );
  assert.equal(
    helper.patchMatchesPersistFilter({
      _map_bridged: {
        filters: {
          layerIdsVisible: ["zone_mask"],
        },
      },
    }),
    true,
  );
});
