import { test } from "bun:test";
import assert from "node:assert/strict";

import {
  clearInitialMapShellSignals,
  consumeInitialMapShellSignals,
  FISHYMAP_LIVE_INIT_EVENT,
  readMapShellSignals,
  resolveMapPageShell,
  writeMapShellLiveSignals,
} from "./map-shell-signals.js";

function createShell() {
  return {
    dispatchEvent() {
      return true;
    },
  };
}

test("map-shell-signals exports the live init event name", () => {
  assert.equal(FISHYMAP_LIVE_INIT_EVENT, "fishymap-live-init");
});

test("resolveMapPageShell returns the live shell element", () => {
  const shell = createShell();
  const globalRef = {
    document: {
      getElementById(id) {
        return id === "map-page-shell" ? shell : null;
      },
    },
  };

  assert.equal(resolveMapPageShell(globalRef), shell);
});

test("readMapShellSignals prefers live shell signals over sticky initial signals", () => {
  const shell = createShell();
  shell.__fishymapInitialSignals = { phase: "initial" };
  shell.__fishymapLiveSignals = { phase: "live" };

  assert.deepEqual(readMapShellSignals(shell), { phase: "live" });
});

test("consumeInitialMapShellSignals returns and clears sticky initial signals", () => {
  const shell = createShell();
  const initialSignals = { _map_ui: { search: { query: "eel" } } };
  shell.__fishymapInitialSignals = initialSignals;

  assert.equal(consumeInitialMapShellSignals(shell), initialSignals);
  assert.equal("__fishymapInitialSignals" in shell, false);
});

test("clearInitialMapShellSignals clears invalid sticky initial signals too", () => {
  const shell = createShell();
  shell.__fishymapInitialSignals = "bad";

  assert.equal(clearInitialMapShellSignals(shell), true);
  assert.equal(readMapShellSignals(shell), null);
});

test("writeMapShellLiveSignals stores the current live signal object", () => {
  const shell = createShell();
  const signals = { _map_runtime: { ready: true } };

  assert.equal(writeMapShellLiveSignals(shell, signals), signals);
  assert.equal(readMapShellSignals(shell), signals);
});
