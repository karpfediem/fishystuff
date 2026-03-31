import test from "node:test";
import assert from "node:assert/strict";

import {
  buildDefaultFishFilterMatches,
  buildSearchMatchSignalPatch,
  buildSearchMatches,
  buildSearchPanelStateBundle,
  buildSearchSelectionRemovalSignalPatch,
  normalizeFishFilterTerms,
} from "./map-search-state.js";
import { normalizeZoneCatalog } from "./map-zone-catalog.js";

function baseSignals() {
  return {
    _map_ui: {
      search: {
        query: "",
        open: true,
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: [],
      },
    },
    _map_runtime: {
      ready: true,
      catalog: {
        fish: [
          { fishId: 912, itemId: 912, name: "Cron Dart", grade: "Rare", isPrize: false },
          { fishId: 77, itemId: 77, name: "Serendia Carp", grade: "General", isPrize: false },
        ],
        semanticTerms: [
          {
            layerId: "regions",
            fieldId: 22,
            label: "Southern Mountain (R22)",
            description: "Origin: Velia",
            layerName: "Regions",
            searchText: "southern mountain regions velia",
          },
        ],
      },
    },
    _shared_fish: {
      caughtIds: [77],
      favouriteIds: [912],
    },
  };
}

test("buildSearchPanelStateBundle keeps only search-relevant live signals", () => {
  assert.deepEqual(buildSearchPanelStateBundle(baseSignals()), {
    state: {
      ready: true,
      catalog: {
        fish: [
          { fishId: 912, itemId: 912, name: "Cron Dart", grade: "Rare", isPrize: false },
          { fishId: 77, itemId: 77, name: "Serendia Carp", grade: "General", isPrize: false },
        ],
        semanticTerms: [
          {
            layerId: "regions",
            fieldId: 22,
            label: "Southern Mountain (R22)",
            description: "Origin: Velia",
            layerName: "Regions",
            searchText: "southern mountain regions velia",
          },
        ],
      },
    },
    inputState: {
      filters: {
        searchText: "",
        fishIds: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: [],
      },
    },
    sharedFishState: {
      caughtIds: [77],
      favouriteIds: [912],
      caughtSet: new Set([77]),
      favouriteSet: new Set([912]),
    },
  });
});

test("buildSearchMatches returns fish filters, fish, and semantic matches from live catalog state", () => {
  const bundle = buildSearchPanelStateBundle({
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "southern",
        open: true,
      },
    },
  });

  const matches = buildSearchMatches(bundle, "southern");
  assert.equal(matches.some((match) => match.kind === "semantic" && match.fieldId === 22), true);

  const fishMatches = buildSearchMatches(bundle, "cron");
  assert.equal(fishMatches.some((match) => match.kind === "fish" && match.fishId === 912), true);

  const filterMatches = buildSearchMatches(bundle, "favorite");
  assert.equal(filterMatches.some((match) => match.kind === "fish-filter" && match.term === "favourite"), true);
});

test("buildSearchMatches returns matched zones instead of dropping zone-name queries", () => {
  const bundle = buildSearchPanelStateBundle({
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "Depth 4",
        open: true,
      },
    },
  });
  const zoneCatalog = normalizeZoneCatalog([
    {
      r: 112,
      g: 167,
      b: 193,
      name: "Zenato Sea - Depth 4",
      confirmed: true,
      order: 1,
    },
  ]);

  const matches = buildSearchMatches(bundle, "Depth 4", zoneCatalog);

  assert.equal(matches.some((match) => match.kind === "zone" && match.zoneRgb === 7382977), true);
  assert.equal(matches[0]?.kind, "zone");
});

test("buildDefaultFishFilterMatches omits already-selected filter terms", () => {
  const bundle = buildSearchPanelStateBundle({
    ...baseSignals(),
    _map_bridged: {
      filters: {
        fishFilterTerms: ["missing"],
      },
    },
  });
  assert.deepEqual(
    buildDefaultFishFilterMatches(bundle).map((match) => match.term),
    ["favourite"],
  );
});

test("buildSearchMatchSignalPatch updates bridged filters and closes the dropdown", () => {
  const signals = baseSignals();

  assert.deepEqual(buildSearchMatchSignalPatch(signals, { kind: "fish", fishId: 912 }), {
    _map_ui: {
      search: {
        query: "",
        open: false,
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [912],
      },
    },
  });

  assert.deepEqual(
    buildSearchMatchSignalPatch(signals, { kind: "fish-filter", term: "favorite" })._map_bridged.filters.fishFilterTerms,
    ["favourite"],
  );

  assert.deepEqual(
    buildSearchMatchSignalPatch(signals, { kind: "semantic", layerId: "regions", fieldId: 22 })._map_bridged.filters.semanticFieldIdsByLayer,
    { regions: [22] },
  );
});

test("buildSearchSelectionRemovalSignalPatch removes selected search filters cleanly", () => {
  const signals = {
    ...baseSignals(),
    _map_bridged: {
      filters: {
        fishIds: [912, 77],
        semanticFieldIdsByLayer: { regions: [22], zone_mask: [123] },
        fishFilterTerms: ["favourite", "missing"],
      },
    },
  };

  assert.deepEqual(
    buildSearchSelectionRemovalSignalPatch(signals, { fishFilterTerm: "favorite" })._map_bridged.filters.fishFilterTerms,
    ["missing"],
  );
  assert.deepEqual(
    buildSearchSelectionRemovalSignalPatch(signals, { fishId: 912 })._map_bridged.filters.fishIds,
    [77],
  );
  assert.deepEqual(
    buildSearchSelectionRemovalSignalPatch(signals, { semanticLayerId: "regions", semanticFieldId: 22 })._map_bridged.filters.semanticFieldIdsByLayer,
    { zone_mask: [123] },
  );
  assert.deepEqual(
    buildSearchSelectionRemovalSignalPatch(signals, { zoneRgb: 123 })._map_bridged.filters.semanticFieldIdsByLayer,
    { regions: [22] },
  );
});

test("normalizeFishFilterTerms normalizes aliases and order", () => {
  assert.deepEqual(normalizeFishFilterTerms(["favorite", "uncaught"]), [
    "favourite",
    "missing",
  ]);
});
