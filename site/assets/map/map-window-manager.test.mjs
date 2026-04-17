import test from "node:test";
import assert from "node:assert/strict";

import {
  buildWindowUiEntryPatch,
  clampManagedWindowPosition,
  createMapWindowManager,
  patchTouchesWindowUi,
} from "./map-window-manager.js";

test("clampManagedWindowPosition keeps windows within the shell bounds", () => {
  assert.deepEqual(
    clampManagedWindowPosition(
      { width: 640, height: 480 },
      { width: 240, height: 320 },
      56,
      900,
      900,
    ),
    { x: 400, y: 424 },
  );

  assert.deepEqual(
    clampManagedWindowPosition(
      { width: 640, height: 480 },
      { width: 240, height: 320 },
      56,
      -50,
      -30,
    ),
    { x: 0, y: 0 },
  );
});

test("buildWindowUiEntryPatch normalizes search collapse and coordinates", () => {
  const patch = buildWindowUiEntryPatch(
    {
      search: { open: true, collapsed: false, x: null, y: null },
      settings: { open: true, collapsed: false, x: null, y: null, autoAdjustView: true },
      zoneInfo: { open: true, collapsed: false, x: null, y: null, tab: "" },
      layers: { open: true, collapsed: false, x: null, y: null },
      bookmarks: { open: false, collapsed: false, x: null, y: null },
    },
    "search",
    { collapsed: true, x: 12.8, y: "33" },
  );

  assert.deepEqual(patch, {
    _map_ui: {
      windowUi: {
        search: {
          open: true,
          collapsed: false,
          x: 13,
          y: 33,
        },
      },
    },
  });
});

test("patchTouchesWindowUi only reacts to window-ui patches", () => {
  assert.equal(
    patchTouchesWindowUi({
      _map_ui: {
        windowUi: {
          settings: { open: false },
        },
      },
    }),
    true,
  );
  assert.equal(
    patchTouchesWindowUi({
      _map_ui: {
        search: {
          open: true,
        },
      },
    }),
    false,
  );
});

class FakeStyle {
  removeProperty(name) {
    delete this[name];
  }
}

class FakeElement extends EventTarget {
  constructor({
    id = "",
    left = 0,
    top = 0,
    width = 240,
    height = 320,
  } = {}) {
    super();
    this.id = id;
    this.dataset = {};
    this.style = new FakeStyle();
    this._rect = { left, top, width, height };
    this._queryMap = new Map();
  }

  getAttribute(name) {
    if (name === "data-window-id") {
      return this.dataset.windowId ?? null;
    }
    return null;
  }

  setQuery(selector, element) {
    this._queryMap.set(selector, element);
  }

  querySelector(selector) {
    return this._queryMap.get(selector) || null;
  }

  querySelectorAll(selector) {
    if (selector === "[data-window-id]") {
      return Array.from(this._queryMap.values()).filter((candidate) => candidate?.dataset?.windowId);
    }
    return [];
  }

  closest() {
    return null;
  }

  getBoundingClientRect() {
    const left = Number.parseFloat(this.style.left ?? this._rect.left);
    const top = Number.parseFloat(this.style.top ?? this._rect.top);
    return {
      left,
      top,
      width: this._rect.width,
      height: this._rect.height,
    };
  }

  setPointerCapture() {}

  releasePointerCapture() {}

  hasPointerCapture() {
    return true;
  }
}

class FakePointerEvent extends Event {
  constructor(type, init = {}) {
    super(type, { bubbles: true });
    this.button = init.button ?? 0;
    this.pointerId = init.pointerId ?? 1;
    this.clientX = init.clientX ?? 0;
    this.clientY = init.clientY ?? 0;
  }
}

test("createMapWindowManager does not overwrite the actively dragged window from stale signals", () => {
  const shell = new FakeElement({ width: 1000, height: 700 });
  const root = new FakeElement({ id: "fishymap-settings-window", left: 10, top: 20 });
  root.dataset.windowId = "settings";
  const titlebar = new FakeElement({ width: 240, height: 56 });
  shell.setQuery("[data-window-titlebar=\"settings\"]", titlebar);
  shell.setQuery("[data-window-id]", root);

  const originalAddEventListener = globalThis.addEventListener;
  const originalRemoveEventListener = globalThis.removeEventListener;
  const listeners = new Map();
  globalThis.addEventListener = (type, handler) => {
    listeners.set(type, handler);
  };
  globalThis.removeEventListener = (type) => {
    listeners.delete(type);
  };

  try {
    const signals = {
      _map_ui: {
        windowUi: {
          search: { open: true, collapsed: false, x: null, y: null },
          settings: { open: true, collapsed: false, x: 10, y: 20, autoAdjustView: true },
          zoneInfo: { open: true, collapsed: false, x: null, y: null, tab: "" },
          layers: { open: true, collapsed: false, x: null, y: null },
          bookmarks: { open: true, collapsed: false, x: null, y: null },
        },
      },
    };

    const manager = createMapWindowManager({
      shell,
      getSignals: () => signals,
      listenToSignalPatches: false,
    });

    assert.equal(root.style.left, "10px");
    assert.equal(root.style.top, "20px");

    titlebar.dispatchEvent(new FakePointerEvent("pointerdown", {
      button: 0,
      pointerId: 9,
      clientX: 20,
      clientY: 30,
    }));
    listeners.get("pointermove")?.(new FakePointerEvent("pointermove", {
      pointerId: 9,
      clientX: 140,
      clientY: 80,
    }));

    assert.equal(root.style.left, "130px");
    assert.equal(root.style.top, "70px");

    manager.applyFromSignals();

    assert.equal(root.style.left, "130px");
    assert.equal(root.style.top, "70px");
  } finally {
    globalThis.addEventListener = originalAddEventListener;
    globalThis.removeEventListener = originalRemoveEventListener;
  }
});

test("createMapWindowManager keeps existing window position on non-position window-ui patches", () => {
  const shell = new FakeElement({ width: 1000, height: 700 });
  const layers = new FakeElement({ id: "fishymap-layers-window", left: 42, top: 55, width: 280, height: 48 });
  layers.dataset.windowId = "layers";
  shell.setQuery("[data-window-id]", layers);

  const signals = {
    _map_ui: {
      windowUi: {
        layers: { open: true, collapsed: false, x: 42, y: 55 },
        zoneInfo: { open: true, collapsed: false, x: 18, y: 22, tab: "zone" },
      },
    },
  };

  const manager = createMapWindowManager({
    shell,
    getSignals: () => signals,
    listenToSignalPatches: false,
  });

  assert.equal(layers.style.left, "42px");
  assert.equal(layers.style.top, "55px");

  manager.applyFromSignals({
    _map_ui: {
      windowUi: {
        zoneInfo: { tab: "territory" },
      },
    },
  });

  assert.equal(layers.style.left, "42px");
  assert.equal(layers.style.top, "55px");
});

test("createMapWindowManager keeps window position when window-ui patch carries null coordinates", () => {
  const shell = new FakeElement({ width: 1000, height: 700 });
  const layers = new FakeElement({ id: "fishymap-layers-window", left: 42, top: 55, width: 280, height: 48 });
  layers.dataset.windowId = "layers";
  shell.setQuery("[data-window-id]", layers);

  const manager = createMapWindowManager({
    shell,
    getSignals: () => ({
      _map_ui: {
        windowUi: {
          layers: { open: true, collapsed: false, x: 42, y: 55 },
        },
      },
    }),
    listenToSignalPatches: false,
  });

  assert.equal(layers.style.left, "42px");
  assert.equal(layers.style.top, "55px");

  manager.applyFromSignals({
    _map_ui: {
      windowUi: {
        layers: { x: null, y: null },
      },
    },
  });

  assert.equal(layers.style.left, "42px");
  assert.equal(layers.style.top, "55px");
});
