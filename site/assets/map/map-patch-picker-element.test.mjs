import test from "node:test";
import assert from "node:assert/strict";
import { installMapTestI18n } from "./test-i18n.js";

import {
  readMapPatchPickerShellSignals,
  registerFishyMapPatchPickerElement,
} from "./map-patch-picker-element.js";

test("readMapPatchPickerShellSignals prefers live shell signals over initial signals", () => {
  const initialSignals = { _map_bridged: { filters: { fromPatchId: "initial" } } };
  const liveSignals = { _map_bridged: { filters: { fromPatchId: "live" } } };
  const shell = {
    __fishymapInitialSignals: initialSignals,
    __fishymapLiveSignals: liveSignals,
  };

  assert.equal(readMapPatchPickerShellSignals(shell), liveSignals);
});

test("registerFishyMapPatchPickerElement defines the custom element once", () => {
  const registry = {
    definitions: new Map(),
    get(name) {
      return this.definitions.get(name) || null;
    },
    define(name, constructor) {
      this.definitions.set(name, constructor);
    },
  };

  assert.equal(registerFishyMapPatchPickerElement(registry), true);
  assert.equal(registerFishyMapPatchPickerElement(registry), true);
  assert.equal(registry.definitions.size, 1);
  assert.ok(registry.get("fishymap-patch-picker"));
});
installMapTestI18n();
