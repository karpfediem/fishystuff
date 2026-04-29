import { test } from "bun:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const PRESET_PREVIEWS_SOURCE = fs.readFileSync(
  new URL("./preset-previews.js", import.meta.url),
  "utf8",
);

class FakeElement {
  constructor(tagName, ownerDocument) {
    this.tagName = tagName;
    this.ownerDocument = ownerDocument;
    this.childNodes = [];
    this.attributes = {};
    this.dataset = {};
    this.className = "";
    this.textContent = "";
  }

  append(...children) {
    this.childNodes.push(...children.filter(Boolean));
  }

  replaceChildren(...children) {
    this.childNodes = children.filter(Boolean);
  }

  setAttribute(name, value) {
    this.attributes[name] = String(value);
  }
}

function createDocument() {
  const document = {
    createElement(tagName) {
      return new FakeElement(tagName, document);
    },
    createElementNS(_namespace, tagName) {
      return new FakeElement(tagName, document);
    },
  };
  return document;
}

function createContext() {
  const listeners = new Map();
  const window = {
    addEventListener(type, listener) {
      if (!listeners.has(type)) {
        listeners.set(type, []);
      }
      listeners.get(type).push(listener);
    },
    dispatchEvent(event) {
      for (const listener of listeners.get(event.type) || []) {
        listener(event);
      }
      return true;
    },
  };
  const document = createDocument();
  const context = {
    window,
    document,
    console,
    JSON,
    Object,
    Array,
    String,
    Number,
    Boolean,
    RegExp,
    Error,
    Map,
    Set,
    CustomEvent: class CustomEvent {
      constructor(type, options = {}) {
        this.type = type;
        this.detail = options.detail;
      }
    },
    globalThis: null,
  };
  context.globalThis = context;
  vm.runInNewContext(PRESET_PREVIEWS_SOURCE, context, { filename: "preset-previews.js" });
  return { context, window, document };
}

function collectTextContent(element) {
  const values = [];
  function visit(node) {
    if (!node) {
      return;
    }
    if (node.textContent) {
      values.push(node.textContent);
    }
    for (const child of node.childNodes || []) {
      visit(child);
    }
  }
  visit(element);
  return values;
}

test("preset preview registry exposes built-in fixed preset payloads", () => {
  const { window } = createContext();
  const helper = window.__fishystuffPresetPreviews;

  assert.deepEqual(
    helper.fixedPresets("calculator-layouts").map((preset) => [preset.id, preset.payload.custom_layout.length]),
    [["default", 3]],
  );
  assert.deepEqual(
    helper.fixedPresets("map-presets").map((preset) => [preset.id, preset.payload.bridgedUi.viewMode]),
    [["default", "2d"]],
  );
  assert.deepEqual(
    helper.fixedPresets("fishydex-presets").map((preset) => [preset.id, preset.payload]),
    [["default", { caughtIds: [], favouriteIds: [] }]],
  );
});

test("preset preview registry resolves title icons without page adapters", () => {
  const { window } = createContext();
  const helper = window.__fishystuffPresetPreviews;

  assert.equal(
    helper.titleIconAlias("calculator-layouts", { payload: { custom_layout: [[["trade"]]] } }),
    "wheel-fill",
  );
  assert.equal(
    helper.titleIconAlias("calculator-presets", { payload: { fishingMode: "harpoon" } }),
    "wheel-fill",
  );
  assert.equal(
    helper.titleIconAlias("map-presets", { payload: { bridgedUi: { viewMode: "3d" } } }),
    "cube-view",
  );
  assert.equal(
    helper.titleIconAlias("fishydex-presets", { payload: { caughtIds: [10], favouriteIds: [] } }),
    "check-badge-solid",
  );
  assert.equal(
    helper.titleIconAlias("fishydex-presets", { payload: { caughtIds: [10], favouriteIds: [10] } }),
    "heart-fill",
  );
});

test("preset preview registry renders the shared layout preview shell", () => {
  const { window, document } = createContext();
  const helper = window.__fishystuffPresetPreviews;
  const shell = helper.createShell({ cardKey: "fixed:default" });
  const container = shell.preview;

  assert.equal(shell.shell.className, "fishy-preset-manager__preset-preview-shell");
  assert.equal(container.dataset.cardKey, "fixed:default");

  const rendered = helper.render(container, {
    collectionKey: "calculator-layouts",
    payload: { custom_layout: [[["overview"]]] },
  });

  assert.equal(rendered, true);
  assert.equal(container.childNodes.length, 1);
  assert.equal(container.childNodes[0].tagName, "svg");
  assert.equal(document.createElement("div").childNodes.length, 0);
});

test("preset preview registry renders Fishydex preset count chips", () => {
  const { window, document } = createContext();
  window.__fishystuffLanguage = {
    t(key, vars = {}) {
      const messages = {
        "fishydex.presets.preview.caught": "{$count} caught",
        "fishydex.presets.preview.favourite": "{$count} favourite",
        "fishydex.presets.preview.tracked": "{$count} tracked",
      };
      return String(messages[key] ?? key).replace(/\{\s*\$([A-Za-z0-9_]+)\s*\}/g, (_match, name) => (
        Object.prototype.hasOwnProperty.call(vars, name) ? String(vars[name]) : ""
      ));
    },
  };
  const helper = window.__fishystuffPresetPreviews;
  const container = document.createElement("div");

  const rendered = helper.render(container, {
    collectionKey: "fishydex-presets",
    payload: {
      caughtIds: [400, 100, 400],
      favouriteIds: [900, 100],
      search_query: "ignored",
    },
  });

  assert.equal(rendered, true);
  assert.deepEqual(collectTextContent(container), [
    "2 caught",
    "2 favourite",
    "3 tracked",
  ]);
});
