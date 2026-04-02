import test from "node:test";
import assert from "node:assert/strict";

const originalHTMLElement = globalThis.HTMLElement;

class FakeElement extends EventTarget {}

async function loadModule() {
  globalThis.HTMLElement = FakeElement;
  return import(`./map-window-manager-element.js?test=${Date.now()}-${Math.random()}`);
}

test("registerFishyMapWindowManagerElement defines the custom element once", async () => {
  const { registerFishyMapWindowManagerElement } = await loadModule();
  const registry = {
    definitions: new Map(),
    get(name) {
      return this.definitions.get(name) || null;
    },
    define(name, constructor) {
      this.definitions.set(name, constructor);
    },
  };

  assert.equal(registerFishyMapWindowManagerElement(registry), true);
  assert.equal(registerFishyMapWindowManagerElement(registry), true);
  assert.equal(registry.definitions.size, 1);
  assert.ok(registry.get("fishymap-window-manager"));
});

test.after(() => {
  globalThis.HTMLElement = originalHTMLElement;
});
