import test from "node:test";
import assert from "node:assert/strict";

import { parseQuerySignalPatch } from "./map-query-state.js";

test("parseQuerySignalPatch maps page-owned and bridged query params into signals", () => {
  const patch = parseQuerySignalPatch(
    "https://fishystuff.fish/map/?fish=91&fishTerms=missing,favourite&search=velia&fromPatch=2026-02-26&toPatch=2026-03-12&layers=zones,terrain&diagnostics=true&view=3d",
  );

  assert.deepEqual(patch, {
    _map_ui: {
      search: {
        query: "velia",
        open: true,
        selectedTerms: [
          { kind: "fish", fishId: 91 },
          { kind: "fish-filter", term: "missing" },
          { kind: "fish-filter", term: "favourite" },
        ],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [91],
        zoneRgbs: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: ["missing", "favourite"],
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
        selectedTerms: [{ kind: "fish", fishId: 820986 }],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [820986],
        zoneRgbs: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: [],
        patchId: "2026-02-26",
      },
    },
  });
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
