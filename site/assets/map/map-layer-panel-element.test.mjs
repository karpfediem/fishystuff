import test from "node:test";
import assert from "node:assert/strict";
import { installMapTestI18n } from "./test-i18n.js";

import {
  FishyMapLayerPanelElement,
  readMapLayerPanelShellSignals,
  registerFishyMapLayerPanelElement,
} from "./map-layer-panel-element.js";

test("readMapLayerPanelShellSignals prefers live shell signals over initial signals", () => {
  const initialSignals = { _map_runtime: { catalog: { layers: [{ layerId: "initial" }] } } };
  const liveSignals = { _map_runtime: { catalog: { layers: [{ layerId: "live" }] } } };
  const shell = {
    __fishymapInitialSignals: initialSignals,
    __fishymapLiveSignals: liveSignals,
  };

  assert.equal(readMapLayerPanelShellSignals(shell), liveSignals);
});

test("registerFishyMapLayerPanelElement defines the custom element once", () => {
  const registry = {
    definitions: new Map(),
    get(name) {
      return this.definitions.get(name) || null;
    },
    define(name, constructor) {
      this.definitions.set(name, constructor);
    },
  };

  assert.equal(registerFishyMapLayerPanelElement(registry), true);
  assert.equal(registerFishyMapLayerPanelElement(registry), true);
  assert.equal(registry.definitions.size, 1);
  assert.ok(registry.get("fishymap-layer-panel"));
});

test("layer panel element exposes render and scheduling hooks", () => {
  const element = new FishyMapLayerPanelElement();
  assert.equal(typeof element.render, "function");
  assert.equal(typeof element.scheduleRender, "function");
  assert.equal(typeof element.writeBridgedFilters, "function");
});
installMapTestI18n();
