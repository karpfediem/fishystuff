import { test } from "bun:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const DATASTAR_STATE_SOURCE = fs.readFileSync(new URL("./datastar-state.js", import.meta.url), "utf8");

function createContext() {
  const context = {
    window: {},
    Object,
    Array,
    String,
    JSON,
    console,
    globalThis: null,
  };
  context.globalThis = context;
  vm.runInNewContext(DATASTAR_STATE_SOURCE, context, { filename: "datastar-state.js" });
  return context.window.__fishystuffDatastarState;
}

test("datastar state helper toggles a nested boolean path in place", () => {
  const helper = createContext();
  const signals = {
    _map_ui: {
      windowUi: {
        search: { open: true },
      },
    },
  };

  const returned = helper.toggleBooleanPath(signals, "_map_ui.windowUi.search.open");

  assert.equal(returned, signals);
  assert.equal(signals._map_ui.windowUi.search.open, false);
});

test("datastar state helper creates missing intermediate objects", () => {
  const helper = createContext();
  const signals = {};

  helper.setObjectPath(signals, "_map_ui.windowUi.layers.open", true);

  assert.deepEqual(JSON.parse(JSON.stringify(signals)), {
    _map_ui: {
      windowUi: {
        layers: { open: true },
      },
    },
  });
});

test("datastar state helper creates a reusable signal store", () => {
  const helper = createContext();
  const store = helper.createSignalStore();
  const signals = {
    _map_input: {
      filters: {
        searchText: "Velia",
      },
    },
  };

  store.connect(signals);
  store.patchSignals({
    _map_ui: {
      search: {
        open: true,
      },
    },
  });

  assert.equal(store.signalObject(), signals);
  assert.equal(store.readSignal("_map_input.filters.searchText"), "Velia");
  assert.equal(signals._map_ui.search.open, true);
});

test("datastar state helper can replace an entire signal branch", () => {
  const helper = createContext();
  const store = helper.createSignalStore();
  const signals = {
    _map_controls: {
      filters: {
        layerWaypointConnectionsVisible: { region_nodes: false },
      },
    },
  };

  store.connect(signals);
  store.writeSignal("_map_controls", {
    filters: {
      layerWaypointConnectionsVisible: {},
    },
  });

  assert.deepEqual(JSON.parse(JSON.stringify(signals)), {
    _map_controls: {
      filters: {
        layerWaypointConnectionsVisible: {},
      },
    },
  });
});

test("datastar state helper creates a page signal store", () => {
  const helper = createContext();
  const store = helper.createPageSignalStore();
  const signals = {
    _calculator_ui: {
      distribution_tab: "groups",
    },
  };

  store.connect(signals);
  store.patchSignals({
    _calculator_actions: {
      clearToken: 1,
    },
  });

  assert.equal(store.signalObject(), signals);
  assert.equal(store.readSignal("_calculator_ui.distribution_tab"), "groups");
  assert.equal(signals._calculator_actions.clearToken, 1);
});

test("datastar state helper merges nested signal patches without replacing siblings", () => {
  const helper = createContext();
  const store = helper.createSignalStore();
  const signals = {
    _map_ui: {
      windowUi: {
        search: { open: true },
      },
      search: {
        open: false,
      },
    },
  };

  store.connect(signals);
  store.patchSignals({
    _map_ui: {
      search: {
        open: true,
      },
    },
  });

  assert.deepEqual(JSON.parse(JSON.stringify(signals)), {
    _map_ui: {
      windowUi: {
        search: { open: true },
      },
      search: {
        open: true,
      },
    },
  });
});

test("datastar state helper toggles ordered selection values deterministically", () => {
  const helper = createContext();

  assert.deepEqual(
    helper.toggleOrderedValue(["yellow", "blue"], "red", ["red", "yellow", "blue", "green"]),
    ["red", "yellow", "blue"],
  );
  assert.deepEqual(
    helper.toggleOrderedValue(["red", "yellow", "blue"], "yellow", ["red", "yellow", "blue", "green"]),
    ["red", "blue"],
  );
});

test("datastar state helper normalizes and consumes incremented action tokens", () => {
  const helper = createContext();
  const previous = {
    copyToken: 1,
    clearToken: 2,
  };
  const next = helper.normalizeCounterTokenState(
    {
      copyToken: "3",
      clearToken: 2,
      ignoredToken: 99,
    },
    {
      copyToken: 0,
      clearToken: 0,
    },
  );
  const fired = [];

  const result = helper.consumeIncrementedCounterTokens(previous, next, {
    copyToken(nextValue, previousValue) {
      fired.push(["copyToken", previousValue, nextValue]);
    },
    clearToken() {
      fired.push(["clearToken"]);
      return true;
    },
  });

  assert.deepEqual(JSON.parse(JSON.stringify(next)), {
    copyToken: 3,
    clearToken: 2,
  });
  assert.deepEqual(JSON.parse(JSON.stringify(result.handledState)), {
    copyToken: 3,
    clearToken: 2,
  });
  assert.deepEqual(fired, [["copyToken", 1, 3]]);
  assert.equal(result.mutated, false);
});

test("datastar state helper creates a reusable counter token controller", () => {
  const helper = createContext();
  const controller = helper.createCounterTokenController({
    copyToken: 0,
    clearToken: 0,
  });
  const fired = [];

  const first = controller.consume(
    {
      copyToken: "2",
      clearToken: 0,
    },
    {
      copyToken(nextValue, previousValue) {
        fired.push(["copyToken", previousValue, nextValue]);
      },
    },
  );

  const second = controller.consume(
    {
      copyToken: 2,
      clearToken: 1,
    },
    {
      clearToken(nextValue, previousValue) {
        fired.push(["clearToken", previousValue, nextValue]);
        return true;
      },
    },
  );

  assert.equal(first.mutated, false);
  assert.equal(second.mutated, true);
  assert.deepEqual(fired, [
    ["copyToken", 0, 2],
    ["clearToken", 0, 1],
  ]);
  assert.deepEqual(JSON.parse(JSON.stringify(controller.handledState())), {
    copyToken: 2,
    clearToken: 1,
  });
});
