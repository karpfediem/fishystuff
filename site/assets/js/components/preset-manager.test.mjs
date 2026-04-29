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
  const syncArgs = [];
  manager.sync = (options = {}) => {
    syncArgs.push(options);
  };

  manager.handleSignalPatch({ detail: { _map_runtime: { selection: {} } } });
  manager.handleSignalPatch({ detail: { _map_bridged: { filters: {} } } });
  assert.equal(syncArgs.length, 0);

  manager.handleSignalPatch({ detail: { _user_presets: { version: 1 } } });
  assert.deepEqual(syncArgs, [{ refreshCurrent: false }]);
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

    manager.sync({ refreshCurrent: true });

    assert.deepEqual(actionStateOptions, { refresh: true, patchDatastar: false });
    assert.equal(elements.get("save").hidden, false);
    assert.equal(elements.get("save").disabled, false);
    assert.equal(elements.get("discard").hidden, false);
    assert.equal(elements.get("discard").disabled, false);
  } finally {
    globalThis.window = previousWindow;
    globalThis.HTMLInputElement = previousHTMLInputElement;
  }
});

test("preset manager trigger shows the active modified preset name", async () => {
  const { FishyPresetManager } = await loadModule();
  const previousWindow = globalThis.window;
  const previousHTMLInputElement = globalThis.HTMLInputElement;
  const elements = new Map(
    [
      "open-icon",
      "manager-icon",
      "open-label",
      "open-status",
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
        title: "",
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
      t(_key, vars = {}) {
        return vars.name ? `Modified: ${vars.name}` : "";
      },
    },
    __fishystuffUserPresets: {
      collectionAdapter() {
        return {
          titleFallback: "Calculator Presets",
          openLabelFallback: "Calculator Presets",
        };
      },
      currentActionState() {
        return {
          canSave: true,
          canDiscard: true,
          currentOrigin: { kind: "fixed", id: "default" },
          selectedSource: { kind: "fixed", id: "default" },
        };
      },
      current() {
        return {
          origin: { kind: "fixed", id: "default" },
        };
      },
    },
  };
  globalThis.HTMLInputElement = class HTMLInputElement {};

  try {
    const manager = new FishyPresetManager();
    manager.dataset = {
      presetCollection: "calculator-presets",
    };
    manager.element = (role) => elements.get(role) || null;
    manager.button = (role) => elements.get(role) || null;
    manager.selectedTitleInput = () => null;
    manager.cardItems = () => {
      const fixedItem = {
        key: "fixed:default",
        kind: "fixed",
        id: "default",
        name: "Default",
        payload: { level: 0 },
        source: { kind: "fixed", id: "default" },
      };
      const currentItem = {
        key: "work:work-default",
        kind: "working",
        id: "work-default",
        name: "Modified: Default",
        payload: { level: 42 },
        source: { kind: "fixed", id: "default" },
        sourceKey: fixedItem.key,
      };
      return {
        items: [fixedItem, currentItem],
        presetItems: [],
        currentPayload: { level: 42 },
        currentItem,
      };
    };
    manager.ensureSelectedCard = () => false;
    manager.selectedItem = (items) => items[1] || null;
    manager.selectedSavedPreset = () => null;
    manager.linkedSavedPresetForCurrent = () => null;
    manager.renderCards = () => {};

    manager.sync();

    assert.equal(elements.get("open-status").textContent, "Modified: Default");
    assert.equal(elements.get("open-status").hidden, false);
    assert.equal(elements.get("open-status").title, "Modified: Default");
  } finally {
    globalThis.window = previousWindow;
    globalThis.HTMLInputElement = previousHTMLInputElement;
  }
});

test("preset manager selected card follows the active source after save", async () => {
  const { FishyPresetManager } = await loadModule();
  const previousWindow = globalThis.window;
  globalThis.window = {
    __fishystuffUserPresets: {
      capturePayload() {
        return { row: 4 };
      },
    },
  };

  try {
    const manager = new FishyPresetManager();
    manager.dataset = {
      presetCollection: "calculator-layouts",
    };
    manager.adapter = () => ({
      normalizePayload(payload) {
        return { row: Number.parseInt(payload?.row ?? 0, 10) || 0 };
      },
    });
    manager.selectedCardKey = "fixed:default";
    const items = [
      {
        key: "fixed:default",
        kind: "fixed",
        id: "default",
        payload: { row: 0 },
      },
      {
        key: "preset:preset_1",
        kind: "preset",
        id: "preset_1",
        payload: { row: 4 },
      },
    ];

    const changed = manager.ensureSelectedCard(items, "preset_1", "");

    assert.equal(changed, true);
    assert.equal(manager.selectedCardKey, "preset:preset_1");
  } finally {
    globalThis.window = previousWindow;
  }
});

test("preset manager applies default selection even when current modifications exist", async () => {
  const { FishyPresetManager } = await loadModule();
  const previousWindow = globalThis.window;
  let activatedFixed = null;
  let closed = false;
  globalThis.window = {
    __fishystuffUserPresets: {
      current() {
        return {
          origin: { kind: "preset", id: "preset_1" },
          payload: { row: 4 },
        };
      },
      activateFixedPreset(collectionKey, fixedId) {
        activatedFixed = { collectionKey, fixedId };
      },
    },
  };

  try {
    const manager = new FishyPresetManager();
    manager.dataset = {
      presetCollection: "calculator-layouts",
    };
    manager.cardItems = () => ({
      items: [{
        key: "fixed:default",
        kind: "fixed",
        id: "default",
        payload: { row: 0 },
        source: { kind: "fixed", id: "default" },
      }],
    });
    manager.closeDialogBeforeApply = () => {
      closed = true;
    };
    manager.sync = () => {};

    manager.applyCardSelection("fixed:default");

    assert.equal(closed, true);
    assert.deepEqual(activatedFixed, {
      collectionKey: "calculator-layouts",
      fixedId: "default",
    });
  } finally {
    globalThis.window = previousWindow;
  }
});

test("preset manager card events resolve delegated child targets", async () => {
  const { FishyPresetManager } = await loadModule();
  const previousWindow = globalThis.window;
  const previousHTMLElement = globalThis.HTMLElement;
  let selectedCardKey = "";

  class FakeElement {
    constructor(dataset = {}) {
      this.dataset = {};
      Object.assign(this.dataset, dataset);
      this.parent = null;
    }

    closest(selector) {
      if (selector !== "[data-role='preset-card']") {
        return null;
      }
      let current = this;
      while (current) {
        if (current.dataset.role === "preset-card") {
          return current;
        }
        current = current.parent;
      }
      return null;
    }
  }

  try {
    globalThis.HTMLElement = FakeElement;
    globalThis.window = {};
    const manager = new FishyPresetManager();
    manager.commitSelectedTitleChange = () => {};
    manager.applyCardSelection = (cardKey) => {
      selectedCardKey = cardKey;
    };
    const card = new FakeElement({
      role: "preset-card",
      cardKey: "fixed:default",
    });
    const child = new FakeElement();
    child.parent = card;

    manager.handleCardClick(child);

    assert.equal(selectedCardKey, "fixed:default");
  } finally {
    globalThis.window = previousWindow;
    globalThis.HTMLElement = previousHTMLElement;
  }
});
