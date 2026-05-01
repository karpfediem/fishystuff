import { test } from "bun:test";
import assert from "node:assert/strict";

import {
  buildQueryFishSelectionSignalPatch,
  buildQueryNpcFocusSignalPatch,
  buildSearchProjectionPatchForSignalPatch,
  createMapPageDerivedController,
} from "./map-page-derived.js";

function createEventTarget() {
  const listeners = new Map();
  return {
    addEventListener(type, listener) {
      if (!listeners.has(type)) {
        listeners.set(type, []);
      }
      listeners.get(type).push(listener);
    },
    removeEventListener(type, listener) {
      if (!listeners.has(type)) {
        return;
      }
      listeners.set(
        type,
        listeners.get(type).filter((candidate) => candidate !== listener),
      );
    },
    dispatchEvent(event) {
      for (const listener of listeners.get(event.type) || []) {
        listener(event);
      }
      return true;
    },
  };
}

test("buildSearchProjectionPatchForSignalPatch projects selected search terms against the patched signal state", () => {
  const patch = buildSearchProjectionPatchForSignalPatch(
    {
      _map_ui: {
        search: {
          selectedTerms: [],
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [],
          zoneRgbs: [],
          semanticFieldIdsByLayer: {},
          fishFilterTerms: [],
        },
      },
    },
    {
      _map_ui: {
        search: {
          selectedTerms: [{ kind: "zone", zoneRgb: 123456 }],
        },
      },
    },
  );

  assert.deepEqual(patch, {
    _map_bridged: {
      filters: {
        fishIds: [],
        zoneRgbs: [123456],
        semanticFieldIdsByLayer: {
          zone_mask: [123456],
        },
        fishFilterTerms: [],
        patchId: null,
        fromPatchId: null,
        toPatchId: null,
        searchExpression: {
          type: "group",
          operator: "or",
          children: [{ type: "term", term: { kind: "zone", zoneRgb: 123456 } }],
        },
      },
    },
  });
});

test("buildQueryFishSelectionSignalPatch resolves pending fish-name selectors from the runtime catalog", () => {
  const patch = buildQueryFishSelectionSignalPatch({
    _map_ui: {
      search: {
        selectedTerms: [{ kind: "fish-filter", term: "favourite" }],
        pendingQueryFishSelectors: ["Pink Dolphin", "opah", "missing fish"],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [],
        zoneRgbs: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: ["favourite"],
      },
    },
    _map_runtime: {
      catalog: {
        fish: [
          { fishId: 235, itemId: 820986, name: "Pink Dolphin" },
          { fishId: 179, itemId: 821292, name: "Opah" },
        ],
      },
    },
  });

  assert.deepEqual(patch, {
    _map_ui: {
      search: {
        expression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            { type: "term", term: { kind: "fish", fishId: 235 } },
            { type: "term", term: { kind: "fish", fishId: 179 } },
          ],
        },
        selectedTerms: [
          { kind: "fish-filter", term: "favourite" },
          { kind: "fish", fishId: 235 },
          { kind: "fish", fishId: 179 },
        ],
        pendingQueryFishSelectors: [],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [235, 179],
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
            { type: "term", term: { kind: "fish", fishId: 235 } },
            { type: "term", term: { kind: "fish", fishId: 179 } },
          ],
        },
      },
    },
  });
});

test("buildQueryNpcFocusSignalPatch resolves pending NPC selectors into focus patches", async () => {
  const patch = await buildQueryNpcFocusSignalPatch(
    {
      _map_ui: {
        search: {
          pendingQueryNpcSelectors: ["chunsu"],
        },
      },
      _map_actions: {
        focusWorldPointToken: 4,
      },
      _map_session: {
        view: {
          viewMode: "2d",
          camera: {
            zoom: 700,
          },
        },
      },
    },
    {
      loadTradeNpcMapCatalogImpl: async () => ({
        features: [
          {
            npcKey: 1,
            npcName: "Chunsu",
            spawn: { worldX: 10, worldZ: 20 },
            sellOrigin: { worldX: 30, worldZ: 40 },
          },
        ],
      }),
    },
  );

  assert.deepEqual(patch, {
    _map_actions: {
      focusWorldPointToken: 5,
      focusWorldPoint: {
        worldX: 10,
        worldZ: 20,
        pointKind: "waypoint",
        pointLabel: "Chunsu",
      },
    },
    _map_session: {
      view: {
        viewMode: "2d",
        camera: {
          zoom: 700,
          centerWorldX: 10,
          centerWorldZ: 20,
        },
      },
    },
    _map_ui: {
      search: {
        pendingQueryNpcSelectors: [],
      },
    },
  });
});

test("map-page-derived controller applies initial query and search projection patches", () => {
  const patches = [];
  const signals = {
    _map_ui: {
      search: {
        query: "",
        selectedTerms: [{ kind: "fish", fishId: 42 }],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [],
        zoneRgbs: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: [],
      },
    },
  };
  const controller = createMapPageDerivedController({
    globalRef: {
      location: {
        href: "https://fishystuff.fish/map/?search=eel",
      },
    },
    readSignals() {
      return signals;
    },
    dispatchPatch(patch) {
      patches.push(patch);
    },
  });

  const applied = controller.applyInitialPatches();

  assert.equal(patches.length, 2);
  assert.equal(applied.queryPatch._map_ui.search.query, "eel");
  assert.deepEqual(applied.projectionPatch, {
    _map_bridged: {
      filters: {
        fishIds: [42],
        zoneRgbs: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: [],
        patchId: null,
        fromPatchId: null,
        toPatchId: null,
        searchExpression: {
          type: "group",
          operator: "or",
          children: [{ type: "term", term: { kind: "fish", fishId: 42 } }],
        },
      },
    },
  });
});

test("map-page-derived controller reacts to shell-local signal patch events", () => {
  const shell = createEventTarget();
  const dispatched = [];
  const controller = createMapPageDerivedController({
    shell,
    readSignals() {
      return {
        _map_ui: {
          search: {
            selectedTerms: [],
          },
        },
        _map_bridged: {
          filters: {
            fishIds: [],
            zoneRgbs: [],
            semanticFieldIdsByLayer: {},
            fishFilterTerms: [],
          },
        },
      };
    },
    dispatchPatch(patch) {
      dispatched.push(patch);
    },
  });

  controller.start(shell);
  shell.dispatchEvent({
    type: "fishymap:signal-patched",
    detail: {
      _map_ui: {
        search: {
          selectedTerms: [{ kind: "zone", zoneRgb: 654321 }],
        },
      },
    },
  });

  assert.equal(dispatched.length, 1);
  assert.deepEqual(dispatched[0], {
    _map_bridged: {
      filters: {
        fishIds: [],
        zoneRgbs: [654321],
        semanticFieldIdsByLayer: {
          zone_mask: [654321],
        },
        fishFilterTerms: [],
        patchId: null,
        fromPatchId: null,
        toPatchId: null,
        searchExpression: {
          type: "group",
          operator: "or",
          children: [{ type: "term", term: { kind: "zone", zoneRgb: 654321 } }],
        },
      },
    },
  });
});

test("map-page-derived controller resolves query fish selectors when the runtime catalog patch lands", () => {
  const shell = createEventTarget();
  const dispatched = [];
  const signals = {
    _map_ui: {
      search: {
        selectedTerms: [],
        pendingQueryFishSelectors: ["Pink Dolphin", "opah"],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [],
        zoneRgbs: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: [],
      },
    },
    _map_runtime: {
      catalog: {
        fish: [],
      },
    },
  };
  const controller = createMapPageDerivedController({
    shell,
    readSignals() {
      return signals;
    },
    dispatchPatch(patch) {
      dispatched.push(patch);
      if (patch?._map_ui?.search) {
        signals._map_ui.search = {
          ...signals._map_ui.search,
          ...patch._map_ui.search,
        };
      }
      if (patch?._map_bridged?.filters) {
        signals._map_bridged.filters = {
          ...signals._map_bridged.filters,
          ...patch._map_bridged.filters,
        };
      }
      if (patch?._map_runtime?.catalog) {
        signals._map_runtime.catalog = patch._map_runtime.catalog;
      }
    },
  });

  controller.start(shell);
  signals._map_runtime.catalog = {
    fish: [
      { fishId: 235, itemId: 820986, name: "Pink Dolphin" },
      { fishId: 179, itemId: 821292, name: "Opah" },
    ],
  };
  shell.dispatchEvent({
    type: "fishymap:signal-patched",
    detail: {
      _map_runtime: {
        catalog: signals._map_runtime.catalog,
      },
    },
  });

  assert.equal(dispatched.length, 1);
  assert.deepEqual(dispatched[0], {
    _map_ui: {
      search: {
        expression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish", fishId: 235 } },
            { type: "term", term: { kind: "fish", fishId: 179 } },
          ],
        },
        selectedTerms: [
          { kind: "fish", fishId: 235 },
          { kind: "fish", fishId: 179 },
        ],
        pendingQueryFishSelectors: [],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [235, 179],
        zoneRgbs: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: [],
        patchId: null,
        fromPatchId: null,
        toPatchId: null,
        searchExpression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish", fishId: 235 } },
            { type: "term", term: { kind: "fish", fishId: 179 } },
          ],
        },
      },
    },
  });
});
