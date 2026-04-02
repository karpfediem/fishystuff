import test from "node:test";
import assert from "node:assert/strict";

import {
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
      },
    },
  });
});
