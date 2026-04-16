import test from "node:test";
import assert from "node:assert/strict";

import {
  buildDefaultFishFilterMatches,
  buildSearchExpressionDragSignalPatch,
  buildSearchExpressionNegationSignalPatch,
  buildSearchExpressionOperatorSignalPatch,
  buildSearchMatchSignalPatch,
  buildSearchPatchBoundToggleSignalPatch,
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
        patches: [],
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
        patches: [],
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
        expression: {
          type: "group",
          operator: "or",
          children: [],
        },
        selectedTerms: [],
      },
      filters: {
        searchText: "",
        fishIds: [],
        zoneRgbs: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: [],
        patchId: null,
        fromPatchId: null,
        toPatchId: null,
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

test("buildSearchMatches returns patch-bound matches and hides already-selected bounds", () => {
  const bundle = buildSearchPanelStateBundle({
    ...baseSignals(),
    _map_runtime: {
      ready: true,
      catalog: {
        fish: [],
        patches: [
          { patchId: "2026-03-12", patchName: "New Era", startTsUtc: 200 },
          { patchId: "2026-02-26", patchName: "Old Guard", startTsUtc: 100 },
        ],
        semanticTerms: [],
      },
    },
    _map_ui: {
      search: {
        query: "new",
        open: true,
        selectedTerms: [{ kind: "patch-bound", bound: "from", patchId: "2026-03-12" }],
      },
    },
  });

  assert.deepEqual(
    buildSearchMatches(bundle, "new").map((match) => [match.kind, match.bound, match.patchId]),
    [["patch-bound", "to", "2026-03-12"]],
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
        expression: {
          type: "group",
          operator: "or",
          children: [{ type: "term", term: { kind: "fish", fishId: 912 } }],
        },
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

test("buildSearchMatchSignalPatch replaces an existing patch bound", () => {
  const signals = {
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "new",
        open: true,
        selectedTerms: [{ kind: "patch-bound", bound: "from", patchId: "2026-02-26" }],
      },
    },
    _map_bridged: {
      filters: {
        fromPatchId: "2026-02-26",
      },
    },
  };

  assert.deepEqual(
    buildSearchMatchSignalPatch(signals, {
      kind: "patch-bound",
      bound: "from",
      patchId: "2026-03-12",
    }),
    {
      _map_ui: {
        search: {
          expression: {
            type: "group",
            operator: "or",
            children: [{ type: "term", term: { kind: "patch-bound", bound: "from", patchId: "2026-03-12" } }],
          },
          selectedTerms: [{ kind: "patch-bound", bound: "from", patchId: "2026-03-12" }],
          query: "",
          open: false,
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [],
          zoneRgbs: [],
          semanticFieldIdsByLayer: {},
          fishFilterTerms: [],
          patchId: null,
          fromPatchId: "2026-03-12",
          toPatchId: null,
          searchExpression: {
            type: "group",
            operator: "or",
            children: [{ type: "term", term: { kind: "patch-bound", bound: "from", patchId: "2026-03-12" } }],
          },
        },
      },
    },
  );
});

test("buildSearchPatchBoundToggleSignalPatch flips a date term and replaces the opposite bound", () => {
  const signals = {
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "",
        open: true,
        expression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "patch-bound", bound: "from", patchId: "2026-02-26" } },
            { type: "term", term: { kind: "patch-bound", bound: "to", patchId: "2026-03-12" } },
            { type: "term", term: { kind: "fish", fishId: 912 } },
          ],
        },
      },
    },
  };

  assert.deepEqual(
    buildSearchPatchBoundToggleSignalPatch(signals, { expressionPath: "root.0" }),
    {
      _map_ui: {
        search: {
          expression: {
            type: "group",
            operator: "or",
            children: [
              { type: "term", term: { kind: "patch-bound", bound: "to", patchId: "2026-02-26" } },
              { type: "term", term: { kind: "fish", fishId: 912 } },
            ],
          },
          selectedTerms: [
            { kind: "patch-bound", bound: "to", patchId: "2026-02-26" },
            { kind: "fish", fishId: 912 },
          ],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [912],
          zoneRgbs: [],
          semanticFieldIdsByLayer: {},
          fishFilterTerms: [],
          patchId: null,
          fromPatchId: null,
          toPatchId: "2026-02-26",
          searchExpression: {
            type: "group",
            operator: "or",
            children: [
              { type: "term", term: { kind: "patch-bound", bound: "to", patchId: "2026-02-26" } },
              { type: "term", term: { kind: "fish", fishId: 912 } },
            ],
          },
        },
      },
    },
  );
});

test("buildSearchMatchSignalPatch preserves nested expression groups when appending a new term", () => {
  const signals = {
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "",
        open: true,
        expression: {
          type: "group",
          operator: "or",
          children: [
            {
              type: "group",
              operator: "and",
              children: [{ type: "term", term: { kind: "fish-filter", term: "favourite" } }],
            },
          ],
        },
      },
    },
  };

  assert.deepEqual(buildSearchMatchSignalPatch(signals, { kind: "fish", fishId: 912 }), {
    _map_ui: {
      search: {
        expression: {
          type: "group",
          operator: "or",
          children: [
            {
              type: "group",
              operator: "and",
              children: [{ type: "term", term: { kind: "fish-filter", term: "favourite" } }],
            },
            {
              type: "term",
              term: { kind: "fish", fishId: 912 },
            },
          ],
        },
        selectedTerms: [
          { kind: "fish-filter", term: "favourite" },
          { kind: "fish", fishId: 912 },
        ],
        query: "",
        open: false,
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [912],
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
            {
              type: "group",
              operator: "and",
              children: [{ type: "term", term: { kind: "fish-filter", term: "favourite" } }],
            },
            { type: "term", term: { kind: "fish", fishId: 912 } },
          ],
        },
      },
    },
  });
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

test("buildSearchSelectionRemovalSignalPatch removes patch bounds cleanly", () => {
  const signals = {
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "",
        open: true,
        selectedTerms: [
          { kind: "patch-bound", bound: "from", patchId: "2026-02-26" },
          { kind: "patch-bound", bound: "to", patchId: "2026-03-12" },
        ],
      },
    },
    _map_bridged: {
      filters: {
        fromPatchId: "2026-02-26",
        toPatchId: "2026-03-12",
      },
    },
  };

  assert.deepEqual(
    buildSearchSelectionRemovalSignalPatch(signals, {
      patchBound: "from",
      patchId: "2026-02-26",
    })._map_bridged.filters,
    {
      fishIds: [],
      zoneRgbs: [],
      semanticFieldIdsByLayer: {},
      fishFilterTerms: [],
      patchId: null,
      fromPatchId: null,
      toPatchId: "2026-03-12",
      searchExpression: {
        type: "group",
        operator: "or",
        children: [{ type: "term", term: { kind: "patch-bound", bound: "to", patchId: "2026-03-12" } }],
      },
    },
  );
});

test("buildSearchExpressionNegationSignalPatch toggles node negation without changing selected terms", () => {
  const signals = {
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "",
        open: true,
        expression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            { type: "term", term: { kind: "fish", fishId: 912 } },
          ],
        },
      },
    },
  };

  assert.deepEqual(buildSearchExpressionNegationSignalPatch(signals, { expressionPath: "root.1" }), {
    _map_ui: {
      search: {
        expression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            {
              type: "term",
              term: { kind: "fish", fishId: 912 },
              negated: true,
            },
          ],
        },
        selectedTerms: [
          { kind: "fish-filter", term: "favourite" },
          { kind: "fish", fishId: 912 },
        ],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [912],
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
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            {
              type: "term",
              term: { kind: "fish", fishId: 912 },
              negated: true,
            },
          ],
        },
      },
    },
  });
});

test("buildSearchSelectionRemovalSignalPatch removes by expression path without flattening groups", () => {
  const signals = {
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "",
        open: true,
        expression: {
          type: "group",
          operator: "or",
          children: [
            {
              type: "group",
              operator: "and",
              children: [
                { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                { type: "term", term: { kind: "fish", fishId: 912 } },
              ],
            },
            { type: "term", term: { kind: "zone", zoneRgb: 123 } },
          ],
        },
      },
    },
  };

  assert.deepEqual(
    buildSearchSelectionRemovalSignalPatch(signals, { expressionPath: "root.0.1" }),
    {
      _map_ui: {
        search: {
          expression: {
            type: "group",
            operator: "or",
            children: [
              { type: "term", term: { kind: "fish-filter", term: "favourite" } },
              { type: "term", term: { kind: "zone", zoneRgb: 123 } },
            ],
          },
          selectedTerms: [
            { kind: "fish-filter", term: "favourite" },
            { kind: "zone", zoneRgb: 123 },
          ],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [],
          zoneRgbs: [123],
          semanticFieldIdsByLayer: { zone_mask: [123] },
          fishFilterTerms: ["favourite"],
          patchId: null,
          fromPatchId: null,
          toPatchId: null,
          searchExpression: {
            type: "group",
            operator: "or",
            children: [
              { type: "term", term: { kind: "fish-filter", term: "favourite" } },
              { type: "term", term: { kind: "zone", zoneRgb: 123 } },
            ],
          },
        },
      },
    },
  );
});

test("buildSearchExpressionOperatorSignalPatch preserves the subgroup when operators match", () => {
  const signals = {
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "",
        open: true,
        expression: {
          type: "group",
          operator: "or",
          children: [
            {
              type: "group",
              operator: "and",
              children: [
                { type: "term", term: { kind: "fish", fishId: 912 } },
                { type: "term", term: { kind: "zone", zoneRgb: 123 } },
              ],
            },
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
          ],
        },
      },
    },
  };

  assert.deepEqual(
    buildSearchExpressionOperatorSignalPatch(signals, {
      groupPath: "root.0",
      boundaryIndex: 1,
      nextOperator: "or",
    }),
    {
      _map_ui: {
        search: {
          expression: {
            type: "group",
            operator: "or",
            children: [
              {
                type: "group",
                operator: "or",
                children: [
                  { type: "term", term: { kind: "fish", fishId: 912 } },
                  { type: "term", term: { kind: "zone", zoneRgb: 123 } },
                ],
              },
              { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            ],
          },
          selectedTerms: [
            { kind: "fish", fishId: 912 },
            { kind: "zone", zoneRgb: 123 },
            { kind: "fish-filter", term: "favourite" },
          ],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [912],
          zoneRgbs: [123],
          semanticFieldIdsByLayer: { zone_mask: [123] },
          fishFilterTerms: ["favourite"],
          patchId: null,
          fromPatchId: null,
          toPatchId: null,
          searchExpression: {
            type: "group",
            operator: "or",
            children: [
              {
                type: "group",
                operator: "or",
                children: [
                  { type: "term", term: { kind: "fish", fishId: 912 } },
                  { type: "term", term: { kind: "zone", zoneRgb: 123 } },
                ],
              },
              { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            ],
          },
        },
      },
    },
  );
});

test("buildSearchExpressionOperatorSignalPatch rewrites only the clicked separator boundary", () => {
  const signals = {
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "",
        open: true,
        expression: {
          type: "group",
          operator: "and",
          children: [
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            { type: "term", term: { kind: "fish", fishId: 912 } },
            { type: "term", term: { kind: "zone", zoneRgb: 123 } },
          ],
        },
      },
    },
  };

  assert.deepEqual(
    buildSearchExpressionOperatorSignalPatch(signals, {
      groupPath: "root",
      boundaryIndex: 2,
      nextOperator: "or",
    }),
    {
      _map_ui: {
        search: {
          expression: {
            type: "group",
            operator: "or",
            children: [
              {
                type: "group",
                operator: "and",
                children: [
                  { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                  { type: "term", term: { kind: "fish", fishId: 912 } },
                ],
              },
              { type: "term", term: { kind: "zone", zoneRgb: 123 } },
            ],
          },
          selectedTerms: [
            { kind: "fish-filter", term: "favourite" },
            { kind: "fish", fishId: 912 },
            { kind: "zone", zoneRgb: 123 },
          ],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [912],
          zoneRgbs: [123],
          semanticFieldIdsByLayer: { zone_mask: [123] },
          fishFilterTerms: ["favourite"],
          patchId: null,
          fromPatchId: null,
          toPatchId: null,
          searchExpression: {
            type: "group",
            operator: "or",
            children: [
              {
                type: "group",
                operator: "and",
                children: [
                  { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                  { type: "term", term: { kind: "fish", fishId: 912 } },
                ],
              },
              { type: "term", term: { kind: "zone", zoneRgb: 123 } },
            ],
          },
        },
      },
    },
  );
});

test("buildSearchExpressionDragSignalPatch moves a term into a target group", () => {
  const signals = {
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "",
        open: true,
        expression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            {
              type: "group",
              operator: "and",
              children: [{ type: "term", term: { kind: "fish", fishId: 912 } }],
            },
          ],
        },
      },
    },
  };

  assert.deepEqual(
    buildSearchExpressionDragSignalPatch(signals, {
      sourcePath: "root.0",
      targetGroupPath: "root.1",
    }),
    {
      _map_ui: {
        search: {
          expression: {
            type: "group",
            operator: "or",
            children: [
              {
                type: "group",
                operator: "and",
                children: [
                  { type: "term", term: { kind: "fish", fishId: 912 } },
                  { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                ],
              },
            ],
          },
          selectedTerms: [
            { kind: "fish", fishId: 912 },
            { kind: "fish-filter", term: "favourite" },
          ],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [912],
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
              {
                type: "group",
                operator: "and",
                children: [
                  { type: "term", term: { kind: "fish", fishId: 912 } },
                  { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                ],
              },
            ],
          },
        },
      },
    },
  );
});

test("buildSearchExpressionDragSignalPatch groups a dragged term with a target term", () => {
  const signals = {
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "",
        open: true,
        expression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            { type: "term", term: { kind: "fish", fishId: 912 } },
            { type: "term", term: { kind: "zone", zoneRgb: 123 } },
          ],
        },
      },
    },
  };

  assert.deepEqual(
    buildSearchExpressionDragSignalPatch(signals, {
      sourcePath: "root.0",
      targetTermPath: "root.1",
      groupOperator: "and",
    }),
    {
      _map_ui: {
        search: {
          expression: {
            type: "group",
            operator: "or",
            children: [
              {
                type: "group",
                operator: "and",
                children: [
                  { type: "term", term: { kind: "fish", fishId: 912 } },
                  { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                ],
              },
              { type: "term", term: { kind: "zone", zoneRgb: 123 } },
            ],
          },
          selectedTerms: [
            { kind: "fish", fishId: 912 },
            { kind: "fish-filter", term: "favourite" },
            { kind: "zone", zoneRgb: 123 },
          ],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [912],
          zoneRgbs: [123],
          semanticFieldIdsByLayer: { zone_mask: [123] },
          fishFilterTerms: ["favourite"],
          patchId: null,
          fromPatchId: null,
          toPatchId: null,
          searchExpression: {
            type: "group",
            operator: "or",
            children: [
              {
                type: "group",
                operator: "and",
                children: [
                  { type: "term", term: { kind: "fish", fishId: 912 } },
                  { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                ],
              },
              { type: "term", term: { kind: "zone", zoneRgb: 123 } },
            ],
          },
        },
      },
    },
  );
});

test("buildSearchExpressionDragSignalPatch moves a dragged subgroup into another group", () => {
  const signals = {
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "",
        open: true,
        expression: {
          type: "group",
          operator: "or",
          children: [
            {
              type: "group",
              operator: "and",
              children: [
                { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                { type: "term", term: { kind: "fish", fishId: 912 } },
              ],
            },
            {
              type: "group",
              operator: "or",
              children: [{ type: "term", term: { kind: "zone", zoneRgb: 123 } }],
            },
          ],
        },
      },
    },
  };

  assert.deepEqual(
    buildSearchExpressionDragSignalPatch(signals, {
      sourcePath: "root.0",
      targetGroupPath: "root.1",
    }),
    {
      _map_ui: {
        search: {
          expression: {
            type: "group",
            operator: "or",
            children: [
              {
                type: "group",
                operator: "or",
                children: [
                  { type: "term", term: { kind: "zone", zoneRgb: 123 } },
                  {
                    type: "group",
                    operator: "and",
                    children: [
                      { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                      { type: "term", term: { kind: "fish", fishId: 912 } },
                    ],
                  },
                ],
              },
            ],
          },
          selectedTerms: [
            { kind: "zone", zoneRgb: 123 },
            { kind: "fish-filter", term: "favourite" },
            { kind: "fish", fishId: 912 },
          ],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [912],
          zoneRgbs: [123],
          semanticFieldIdsByLayer: { zone_mask: [123] },
          fishFilterTerms: ["favourite"],
          patchId: null,
          fromPatchId: null,
          toPatchId: null,
          searchExpression: {
            type: "group",
            operator: "or",
            children: [
              {
                type: "group",
                operator: "or",
                children: [
                  { type: "term", term: { kind: "zone", zoneRgb: 123 } },
                  {
                    type: "group",
                    operator: "and",
                    children: [
                      { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                      { type: "term", term: { kind: "fish", fishId: 912 } },
                    ],
                  },
                ],
              },
            ],
          },
        },
      },
    },
  );
});

test("buildSearchExpressionDragSignalPatch groups a dragged subgroup with a target term", () => {
  const signals = {
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "",
        open: true,
        expression: {
          type: "group",
          operator: "or",
          children: [
            {
              type: "group",
              operator: "and",
              children: [
                { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                { type: "term", term: { kind: "fish", fishId: 912 } },
              ],
            },
            { type: "term", term: { kind: "zone", zoneRgb: 123 } },
          ],
        },
      },
    },
  };

  assert.deepEqual(
    buildSearchExpressionDragSignalPatch(signals, {
      sourcePath: "root.0",
      targetNodePath: "root.1",
      groupOperator: "or",
    }),
    {
      _map_ui: {
        search: {
          expression: {
            type: "group",
            operator: "or",
            children: [
              {
                type: "group",
                operator: "or",
                children: [
                  { type: "term", term: { kind: "zone", zoneRgb: 123 } },
                  {
                    type: "group",
                    operator: "and",
                    children: [
                      { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                      { type: "term", term: { kind: "fish", fishId: 912 } },
                    ],
                  },
                ],
              },
            ],
          },
          selectedTerms: [
            { kind: "zone", zoneRgb: 123 },
            { kind: "fish-filter", term: "favourite" },
            { kind: "fish", fishId: 912 },
          ],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [912],
          zoneRgbs: [123],
          semanticFieldIdsByLayer: { zone_mask: [123] },
          fishFilterTerms: ["favourite"],
          patchId: null,
          fromPatchId: null,
          toPatchId: null,
          searchExpression: {
            type: "group",
            operator: "or",
            children: [
              {
                type: "group",
                operator: "or",
                children: [
                  { type: "term", term: { kind: "zone", zoneRgb: 123 } },
                  {
                    type: "group",
                    operator: "and",
                    children: [
                      { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                      { type: "term", term: { kind: "fish", fishId: 912 } },
                    ],
                  },
                ],
              },
            ],
          },
        },
      },
    },
  );
});

test("buildSearchExpressionDragSignalPatch reorders a dragged node by group slot index", () => {
  const signals = {
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "",
        open: true,
        expression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            { type: "term", term: { kind: "fish", fishId: 912 } },
            { type: "term", term: { kind: "zone", zoneRgb: 123 } },
          ],
        },
      },
    },
  };

  assert.deepEqual(
    buildSearchExpressionDragSignalPatch(signals, {
      sourcePath: "root.0",
      targetGroupPath: "root",
      targetGroupIndex: 2,
    }),
    {
      _map_ui: {
        search: {
          expression: {
            type: "group",
            operator: "or",
            children: [
              { type: "term", term: { kind: "fish", fishId: 912 } },
              { type: "term", term: { kind: "fish-filter", term: "favourite" } },
              { type: "term", term: { kind: "zone", zoneRgb: 123 } },
            ],
          },
          selectedTerms: [
            { kind: "fish", fishId: 912 },
            { kind: "fish-filter", term: "favourite" },
            { kind: "zone", zoneRgb: 123 },
          ],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [912],
          zoneRgbs: [123],
          semanticFieldIdsByLayer: { zone_mask: [123] },
          fishFilterTerms: ["favourite"],
          patchId: null,
          fromPatchId: null,
          toPatchId: null,
          searchExpression: {
            type: "group",
            operator: "or",
            children: [
              { type: "term", term: { kind: "fish", fishId: 912 } },
              { type: "term", term: { kind: "fish-filter", term: "favourite" } },
              { type: "term", term: { kind: "zone", zoneRgb: 123 } },
            ],
          },
        },
      },
    },
  );
});

test("buildSearchExpressionDragSignalPatch groups a dragged subgroup with a target subgroup handle", () => {
  const signals = {
    ...baseSignals(),
    _map_ui: {
      search: {
        query: "",
        open: true,
        expression: {
          type: "group",
          operator: "or",
          children: [
            {
              type: "group",
              operator: "and",
              children: [{ type: "term", term: { kind: "fish-filter", term: "favourite" } }],
            },
            {
              type: "group",
              operator: "or",
              children: [{ type: "term", term: { kind: "zone", zoneRgb: 123 } }],
            },
          ],
        },
      },
    },
  };

  assert.deepEqual(
    buildSearchExpressionDragSignalPatch(signals, {
      sourcePath: "root.0",
      targetNodePath: "root.1",
      groupOperator: "and",
    }),
    {
      _map_ui: {
        search: {
          expression: {
            type: "group",
            operator: "or",
            children: [
              {
                type: "group",
                operator: "and",
                children: [
                  { type: "term", term: { kind: "zone", zoneRgb: 123 } },
                  { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                ],
              },
            ],
          },
          selectedTerms: [
            { kind: "zone", zoneRgb: 123 },
            { kind: "fish-filter", term: "favourite" },
          ],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [],
          zoneRgbs: [123],
          semanticFieldIdsByLayer: { zone_mask: [123] },
          fishFilterTerms: ["favourite"],
          patchId: null,
          fromPatchId: null,
          toPatchId: null,
          searchExpression: {
            type: "group",
            operator: "or",
            children: [
              {
                type: "group",
                operator: "and",
                children: [
                  { type: "term", term: { kind: "zone", zoneRgb: 123 } },
                  { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                ],
              },
            ],
          },
        },
      },
    },
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
