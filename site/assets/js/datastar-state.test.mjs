import test from "node:test";
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
