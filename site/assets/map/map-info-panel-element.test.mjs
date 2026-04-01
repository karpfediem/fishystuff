import test from "node:test";
import assert from "node:assert/strict";

import {
  FishyMapInfoPanelElement,
  readMapInfoPanelShellSignals,
  registerFishyMapInfoPanelElement,
} from "./map-info-panel-element.js";

test("readMapInfoPanelShellSignals prefers live shell signals over initial signals", () => {
  const initialSignals = { _map_runtime: { selection: { pointLabel: "Initial" } } };
  const liveSignals = { _map_runtime: { selection: { pointLabel: "Live" } } };
  const shell = {
    __fishymapInitialSignals: initialSignals,
    __fishymapLiveSignals: liveSignals,
  };

  assert.equal(readMapInfoPanelShellSignals(shell), liveSignals);
});

test("registerFishyMapInfoPanelElement defines the custom element once", () => {
  const registry = {
    definitions: new Map(),
    get(name) {
      return this.definitions.get(name) || null;
    },
    define(name, constructor) {
      this.definitions.set(name, constructor);
    },
  };

  assert.equal(registerFishyMapInfoPanelElement(registry), true);
  assert.equal(registerFishyMapInfoPanelElement(registry), true);
  assert.equal(registry.definitions.size, 1);
  assert.ok(registry.get("fishymap-info-panel"));
});

test("info panel element exposes refresh and signal patch handlers", () => {
  const element = new FishyMapInfoPanelElement();
  assert.equal(typeof element.handleSignalPatch, "function");
  assert.equal(typeof element.refreshZoneLootSummary, "function");
  assert.equal(typeof element.render, "function");
});
