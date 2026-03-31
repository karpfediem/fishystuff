import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const MAP_PAGE_STATE_SOURCE = fs.readFileSync(new URL("./map-page-state.js", import.meta.url), "utf8");

function loadHelper() {
  const context = {
    window: {},
    globalThis: null,
    JSON,
    Object,
    Array,
    String,
    Number,
    URL,
  };
  context.globalThis = context;
  vm.runInNewContext(MAP_PAGE_STATE_SOURCE, context, { filename: "map-page-state.js" });
  return context.window.__fishystuffMapPageState;
}

test("map-page-state restoreUiPatch falls back to default enabled layers", () => {
  const helper = loadHelper();
  const patch = helper.restoreUiPatch({
    bridgedFilters: {},
  });
  assert.deepEqual(JSON.parse(JSON.stringify(patch._map_bridged.filters.layerIdsVisible)), [
    "bookmarks",
    "fish_evidence",
    "zone_mask",
    "minimap",
  ]);
});

test("map-page-state strips query-owned restore fields", () => {
  const helper = loadHelper();
  const stripped = helper.stripQueryOwnedRestoreFields(
    {
      _map_ui: {
        search: { query: "migaloo" },
      },
      _map_bridged: {
        ui: {
          diagnosticsOpen: true,
        },
        filters: {
          fishIds: [153],
          layerIdsVisible: ["zone_mask"],
          fromPatchId: "100",
          toPatchId: "200",
        },
      },
    },
    "https://fishystuff.fish/map/?search=whale&fish=153&layers=zone_mask&fromPatch=100&toPatch=200&diagnostics=1",
  );
  assert.deepEqual(JSON.parse(JSON.stringify(stripped)), null);
});
