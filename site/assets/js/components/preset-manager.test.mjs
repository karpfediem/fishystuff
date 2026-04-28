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

test("preset manager uses shared preset action state for save and discard buttons", async () => {
  const { FishyPresetManager } = await loadModule();
  const previousWindow = globalThis.window;
  const previousHTMLInputElement = globalThis.HTMLInputElement;
  let actionStateOptions = null;
  const elements = new Map(
    [
      "open-icon",
      "manager-icon",
      "open-label",
      "manager-title",
      "grid-title",
      "grid-count",
      "grid-empty",
      "status",
      "selected-section-title",
      "selected-title-label",
      "save",
      "discard",
      "copy",
      "export",
      "import",
      "delete",
    ].map((role) => [
      role,
      {
        role,
        hidden: false,
        disabled: false,
        textContent: "",
        innerHTML: "",
        className: "",
        querySelector() {
          return null;
        },
        setAttribute() {},
      },
    ]),
  );

  globalThis.window = {
    __fishystuffCalculator: {
      iconSpriteUrl: "/img/icons.svg",
    },
    __fishystuffLanguage: {
      t(key) {
        return key;
      },
    },
    __fishystuffUserPresets: {
      collectionAdapter() {
        return {
          titleFallback: "Workspace Presets",
          openLabelFallback: "Workspace Presets",
        };
      },
      currentActionState(_collectionKey, options = {}) {
        actionStateOptions = options;
        return {
          canSave: true,
          canDiscard: true,
        };
      },
    },
  };
  globalThis.HTMLInputElement = class HTMLInputElement {};

  try {
    const manager = new FishyPresetManager();
    manager.dataset = {
      presetCollection: "calculator-layouts",
    };
    manager.element = (role) => elements.get(role) || null;
    manager.button = (role) => elements.get(role) || null;
    manager.selectedTitleInput = () => null;
    manager.cardItems = () => ({
      items: [],
      presetItems: [],
      currentPayload: null,
      currentItem: null,
    });
    manager.ensureSelectedCard = () => false;
    manager.selectedItem = () => null;
    manager.selectedSavedPreset = () => null;
    manager.linkedSavedPresetForCurrent = () => null;
    manager.renderCards = () => {};

    manager.sync();

    assert.deepEqual(actionStateOptions, { refresh: true });
    assert.equal(elements.get("save").hidden, false);
    assert.equal(elements.get("save").disabled, false);
    assert.equal(elements.get("discard").hidden, false);
    assert.equal(elements.get("discard").disabled, false);
  } finally {
    globalThis.window = previousWindow;
    globalThis.HTMLInputElement = previousHTMLInputElement;
  }
});
