import test from "node:test";
import assert from "node:assert/strict";

import {
  readMapSearchPanelShellSignals,
  registerFishyMapSearchPanelElement,
} from "./map-search-panel-element.js";

test("readMapSearchPanelShellSignals prefers live shell signals over initial signals", () => {
  const initialSignals = { _map_ui: { search: { query: "initial" } } };
  const liveSignals = { _map_ui: { search: { query: "live" } } };
  const shell = {
    __fishymapInitialSignals: initialSignals,
    __fishymapLiveSignals: liveSignals,
  };

  assert.equal(readMapSearchPanelShellSignals(shell), liveSignals);
});

test("registerFishyMapSearchPanelElement defines the custom element once", () => {
  const registry = {
    definitions: new Map(),
    get(name) {
      return this.definitions.get(name) || null;
    },
    define(name, constructor) {
      this.definitions.set(name, constructor);
    },
  };

  assert.equal(registerFishyMapSearchPanelElement(registry), true);
  assert.equal(registerFishyMapSearchPanelElement(registry), true);
  assert.equal(registry.definitions.size, 1);
  assert.ok(registry.get("fishymap-search-panel"));
});
