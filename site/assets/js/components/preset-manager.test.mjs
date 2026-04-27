import { test } from "bun:test";
import assert from "node:assert/strict";

async function loadModule() {
  const originalHTMLElement = globalThis.HTMLElement;
  const originalCustomElements = globalThis.customElements;
  globalThis.HTMLElement = globalThis.HTMLElement ?? class {};
  globalThis.customElements = globalThis.customElements ?? {
    define() {},
    get() {
      return null;
    },
  };
  try {
    return await import(`./preset-manager.js?test=${Date.now()}-${Math.random()}`);
  } finally {
    globalThis.HTMLElement = originalHTMLElement;
    globalThis.customElements = originalCustomElements;
  }
}

test("preset manager ignores unrelated Datastar signal patches", async () => {
  const { FishyPresetManager, patchTouchesPresetManager } = await loadModule();

  assert.equal(patchTouchesPresetManager({ _map_runtime: { selection: {} } }), false);
  assert.equal(patchTouchesPresetManager({ _map_bridged: { filters: {} } }), false);
  assert.equal(patchTouchesPresetManager({ _user_presets: { version: 1 } }), true);
  assert.equal(patchTouchesPresetManager({ _preset_manager_ui: { "map-presets": { open: true } } }), true);

  const manager = new FishyPresetManager();
  let syncCount = 0;
  manager.sync = () => {
    syncCount += 1;
  };

  manager.handleSignalPatch({ detail: { _map_runtime: { selection: {} } } });
  manager.handleSignalPatch({ detail: { _map_bridged: { filters: {} } } });
  assert.equal(syncCount, 0);

  manager.handleSignalPatch({ detail: { _user_presets: { version: 1 } } });
  assert.equal(syncCount, 1);
});
