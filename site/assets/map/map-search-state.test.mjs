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
        selectedTerms: [],
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
      search: {
        searchText: "",
        selectedTerms: [],
      },
      filters: {
        searchText: "",
        fishIds: [],
        zoneRgbs: [],
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
        selectedTerms: [],
      },
    },
  });

  const matches = buildSearchMatches(bundle, "southern");
  assert.equal(matches.some((match) => match.kind === "semantic" && match.fieldId === 22), true);

  const fishMatches = buildSearchMatches(bundle, "cron");
  assert.equal(fishMatches.some((match) => match.kind === "fish" && match.fishId === 912), true);

  const filterMatches = buildSearchMatches(bundle, "favorite");
  assert.equal(filterMatches.some((match) => match.kind === "fish-filter" && match.term === "favourite"), true);

  const gradeMatches = buildSearchMatches(bundle, "rare");
  assert.equal(gradeMatches.some((match) => match.kind === "fish-filter" && match.term === "yellow"), true);
  assert.equal(gradeMatches.some((match) => match.kind === "fish" && match.fishId === 912), true);
});

test("buildSearchMatches treats multiple selected grade filters as an OR group", () => {
  const bundle = buildSearchPanelStateBundle(baseSignals());

  const matches = buildSearchMatches(bundle, "rare general");

  assert.deepEqual(
    matches.filter((match) => match.kind === "fish").map((match) => match.fishId),
    [912, 77],
  );
});

test("buildSearchMatches does not constrain fish matches from selected favourite filters", () => {
  const bundle = buildSearchPanelStateBundle({
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "serendia",
        open: true,
        selectedTerms: [{ kind: "fish-filter", term: "favourite" }],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: ["favourite"],
      },
    },
    _map_runtime: {
      ready: true,
      catalog: {
        fish: [
          { fishId: 235, itemId: 820986, name: "Pink Dolphin", grade: "Prize", isPrize: true },
          { fishId: 77, itemId: 77, name: "Serendia Carp", grade: "General", isPrize: false },
        ],
        semanticTerms: [],
      },
    },
    _shared_fish: {
      caughtIds: [],
      favouriteIds: [820986],
    },
  });

  assert.deepEqual(
    buildSearchMatches(bundle, "serendia")
      .filter((match) => match.kind === "fish")
      .map((match) => match.fishId),
    [77],
  );
});

test("buildSearchMatches does not constrain fish matches from selected missing filters", () => {
  const bundle = buildSearchPanelStateBundle({
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "pink",
        open: true,
        selectedTerms: [{ kind: "fish-filter", term: "missing" }],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: ["missing"],
      },
    },
    _map_runtime: {
      ready: true,
      catalog: {
        fish: [
          { fishId: 235, itemId: 820986, name: "Pink Dolphin", grade: "Prize", isPrize: true },
          { fishId: 77, itemId: 77, name: "Serendia Carp", grade: "General", isPrize: false },
        ],
        semanticTerms: [],
      },
    },
    _shared_fish: {
      caughtIds: [820986],
      favouriteIds: [],
    },
  });

  assert.deepEqual(
    buildSearchMatches(bundle, "pink")
      .filter((match) => match.kind === "fish")
      .map((match) => match.fishId),
    [235],
  );
});

test("buildSearchMatches returns matched zones instead of dropping zone-name queries", () => {
  const bundle = buildSearchPanelStateBundle({
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "Depth 4",
        open: true,
        selectedTerms: [],
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
    _map_ui: {
      search: {
        query: "",
        open: true,
        selectedTerms: [{ kind: "fish-filter", term: "missing" }],
      },
    },
    _map_bridged: {
      filters: {
        fishFilterTerms: ["missing"],
      },
    },
  });
  assert.deepEqual(
    buildDefaultFishFilterMatches(bundle).map((match) => match.term),
    ["favourite", "red", "yellow", "blue", "green", "white"],
  );
});

test("buildSearchMatchSignalPatch updates bridged filters and closes the dropdown", () => {
  const signals = baseSignals();

  assert.deepEqual(buildSearchMatchSignalPatch(signals, { kind: "fish", fishId: 912 }), {
    _map_ui: {
      search: {
        selectedTerms: [{ kind: "fish", fishId: 912 }],
        query: "",
        open: false,
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
    _map_ui: {
      search: {
        query: "",
        open: true,
        selectedTerms: [
          { kind: "fish-filter", term: "favourite" },
          { kind: "fish-filter", term: "missing" },
          { kind: "fish", fishId: 912 },
          { kind: "fish", fishId: 77 },
          { kind: "zone", zoneRgb: 123 },
          { kind: "semantic", layerId: "regions", fieldId: 22 },
        ],
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
  assert.deepEqual(normalizeFishFilterTerms(["favorite", "uncaught", "rare", "trash"]), [
    "favourite",
    "missing",
    "yellow",
    "white",
  ]);
});
