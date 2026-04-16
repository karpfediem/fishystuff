import test from "node:test";
import assert from "node:assert/strict";

import {
  readMapSearchPanelShellSignals,
  resolveSearchPanelMatches,
  registerFishyMapSearchPanelElement,
} from "./map-search-panel-element.js";

const originalHTMLElement = globalThis.HTMLElement;
const originalDocument = globalThis.document;
const originalRequestAnimationFrame = globalThis.requestAnimationFrame;
const originalCancelAnimationFrame = globalThis.cancelAnimationFrame;

class FakeElement extends EventTarget {
  constructor() {
    super();
    this.hidden = false;
    this.attributes = new Map();
    this.dataset = {};
    this.style = {};
    this.textContent = "";
    this.id = "";
    this._innerHTML = "";
    this._queryMap = new Map();
    this._closestMap = new Map();
    this._children = [];
    this._parent = null;
  }

  get innerHTML() {
    return this._innerHTML;
  }

  set innerHTML(value) {
    this._innerHTML = String(value ?? "");
    this._queryMap.clear();
    const idPattern = /id="([^"]+)"/g;
    for (const [, id] of this._innerHTML.matchAll(idPattern)) {
      const element = new FakeElement();
      element.id = id;
      this._queryMap.set(`#${id}`, element);
    }
  }

  setQuery(selector, element) {
    this._queryMap.set(selector, element);
  }

  setAttribute(name, value) {
    const normalizedName = String(name);
    const normalizedValue = String(value ?? "");
    const previousValue = this.attributes.has(normalizedName)
      ? this.attributes.get(normalizedName)
      : null;
    this.attributes.set(normalizedName, normalizedValue);
    this.attributeChangedCallback?.(normalizedName, previousValue, normalizedValue);
  }

  getAttribute(name) {
    return this.attributes.has(name) ? this.attributes.get(name) : null;
  }

  removeAttribute(name) {
    this.attributes.delete(String(name));
  }

  setClosest(selector, element) {
    this._closestMap.set(selector, element);
  }

  querySelector(selector) {
    return this._queryMap.get(selector) || null;
  }

  closest(selector) {
    return this._closestMap.get(selector) || null;
  }

  contains(target) {
    return Array.from(this._queryMap.values()).includes(target);
  }

  appendChild(child) {
    if (!child) {
      return child;
    }
    child._parent = this;
    this._children.push(child);
    return child;
  }

  remove() {
    if (!this._parent) {
      return;
    }
    this._parent._children = this._parent._children.filter((child) => child !== this);
    this._parent = null;
  }

  cloneNode() {
    const clone = new FakeElement();
    clone.hidden = this.hidden;
    clone.textContent = this.textContent;
    clone.id = this.id;
    clone._innerHTML = this._innerHTML;
    clone.dataset = { ...this.dataset };
    clone.style = { ...this.style };
    clone.attributes = new Map(this.attributes);
    return clone;
  }

  getBoundingClientRect() {
    return {
      width: 96,
      height: 28,
    };
  }
}

function createDocumentStub() {
  const document = new EventTarget();
  document.activeElement = null;
  document.body = new FakeElement();
  document.documentElement = new FakeElement();
  document.createElement = () => new FakeElement();
  return document;
}

function createSignals() {
  return {
    _map_ui: {
      search: {
        open: false,
        query: "",
        selectedTerms: [],
      },
    },
    _map_bridged: {
      filters: {},
    },
    _map_runtime: {
      ready: false,
      catalog: {
        fish: [],
        semanticTerms: [],
      },
    },
    _shared_fish: {
      caughtIds: [],
      favouriteIds: [],
    },
  };
}

async function loadModule() {
  globalThis.HTMLElement = FakeElement;
  globalThis.document = createDocumentStub();
  globalThis.requestAnimationFrame = undefined;
  globalThis.cancelAnimationFrame = undefined;
  return import(`./map-search-panel-element.js?test=${Date.now()}-${Math.random()}`);
}

function createShellAndPanel(FishyMapSearchPanelElement) {
  const shell = new FakeElement();
  const searchWindow = new FakeElement();
  const searchCount = new FakeElement();
  const panel = new FishyMapSearchPanelElement();
  panel.id = "fishymap-search-panel";
  panel.setClosest("#map-page-shell", shell);
  shell.setQuery("#fishymap-search-window", searchWindow);
  shell.setQuery("#fishymap-search-count", searchCount);
  return { shell, panel, searchWindow, searchCount };
}

test("readMapSearchPanelShellSignals prefers live shell signals over initial signals", () => {
  const initialSignals = { _map_ui: { search: { query: "initial" } } };
  const liveSignals = { _map_ui: { search: { query: "live" } } };
  const shell = {
    __fishymapInitialSignals: initialSignals,
    __fishymapLiveSignals: liveSignals,
  };

  assert.equal(readMapSearchPanelShellSignals(shell), liveSignals);
});

test("registerFishyMapSearchPanelElement defines the custom element once", () => {
  const registry = {
    definitions: new Map(),
    get(name) {
      return this.definitions.get(name) || null;
    },
    define(name, constructor) {
      this.definitions.set(name, constructor);
    },
  };

  assert.equal(registerFishyMapSearchPanelElement(registry), true);
  assert.equal(registerFishyMapSearchPanelElement(registry), true);
  assert.equal(registry.definitions.size, 1);
  assert.ok(registry.get("fishymap-search-panel"));
});

test("resolveSearchPanelMatches keeps frontend filter matches available while the runtime is still loading", () => {
  const matches = resolveSearchPanelMatches(
    {
      state: {
        ready: false,
        catalog: {
          fish: [],
          semanticTerms: [],
        },
      },
      inputState: {
        filters: {
          searchText: "favorite",
          fishIds: [],
          zoneRgbs: [],
          semanticFieldIdsByLayer: {},
          fishFilterTerms: [],
        },
      },
      sharedFishState: {
        caughtIds: [],
        favouriteIds: [],
        caughtSet: new Set(),
        favouriteSet: new Set(),
      },
    },
    {
      open: true,
      query: "favorite",
      selectedTerms: [],
    },
  );

  assert.deepEqual(
    matches.map((match) => match.kind === "fish-filter" ? match.term : match.kind),
    ["favourite"],
  );
});

test("FishyMapSearchPanelElement rerenders search results from Datastar-driven attribute updates", async () => {
  const { FishyMapSearchPanelElement } = await loadModule();
  const { shell, panel, searchCount } = createShellAndPanel(FishyMapSearchPanelElement);
  const signals = createSignals();
  shell.__fishymapLiveSignals = signals;

  panel.connectedCallback();
  assert.equal(panel.querySelector("#fishymap-search-results")?.innerHTML, "");
  assert.equal(searchCount.hidden, true);

  signals._map_ui.search.open = true;
  signals._map_ui.search.query = "favorite";
  panel.setAttribute("data-search-state", JSON.stringify(signals._map_ui.search));

  assert.match(panel.querySelector("#fishymap-search-results")?.innerHTML || "", /Favourite/);
  assert.equal(searchCount.hidden, false);
  assert.equal(searchCount.textContent, "1 match");
});

test("FishyMapSearchPanelElement dispatches operator-toggle patches that preserve same-operator groups", async () => {
  const { FishyMapSearchPanelElement } = await loadModule();
  const { shell, panel } = createShellAndPanel(FishyMapSearchPanelElement);
  const signals = createSignals();
  signals._map_ui.search.expression = {
    type: "group",
    operator: "or",
    children: [
      {
        type: "group",
        operator: "and",
        children: [
          { type: "term", term: { kind: "fish", fishId: 912 } },
          { type: "term", term: { kind: "zone", zoneRgb: 123 } },
        ],
      },
    ],
  };
  shell.__fishymapLiveSignals = signals;
  panel.connectedCallback();

  let dispatchedPatch = null;
  panel.dispatchPatch = (patch) => {
    dispatchedPatch = patch;
  };

  const button = new FakeElement();
  button.setAttribute("data-expression-group-path", "root.0");
  button.setAttribute("data-expression-boundary-index", "1");
  button.setAttribute("data-expression-next-operator", "or");
  button.setClosest(
    "button.fishy-applied-expression-operator-toggle[data-expression-group-path][data-expression-boundary-index][data-expression-next-operator]",
    button,
  );

  panel._handleClick({
    target: button,
  });

  assert.deepEqual(dispatchedPatch, {
    _map_ui: {
      search: {
        expression: {
          type: "group",
          operator: "or",
          children: [
            {
              type: "group",
              operator: "or",
              children: [
                { type: "term", term: { kind: "fish", fishId: 912 } },
                { type: "term", term: { kind: "zone", zoneRgb: 123 } },
              ],
            },
          ],
        },
        selectedTerms: [
          { kind: "fish", fishId: 912 },
          { kind: "zone", zoneRgb: 123 },
        ],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [912],
        zoneRgbs: [123],
        semanticFieldIdsByLayer: { zone_mask: [123] },
        fishFilterTerms: [],
        patchId: null,
        fromPatchId: null,
        toPatchId: null,
        searchExpression: {
          type: "group",
          operator: "or",
          children: [
            {
              type: "group",
              operator: "or",
              children: [
                { type: "term", term: { kind: "fish", fishId: 912 } },
                { type: "term", term: { kind: "zone", zoneRgb: 123 } },
              ],
            },
          ],
        },
      },
    },
  });
});

test("FishyMapSearchPanelElement dispatches negation-toggle patches from the applied expression view", async () => {
  const { FishyMapSearchPanelElement } = await loadModule();
  const { shell, panel } = createShellAndPanel(FishyMapSearchPanelElement);
  const signals = createSignals();
  signals._map_ui.search.expression = {
    type: "group",
    operator: "or",
    children: [
      { type: "term", term: { kind: "fish-filter", term: "favourite" } },
      { type: "term", term: { kind: "fish", fishId: 912 } },
    ],
  };
  shell.__fishymapLiveSignals = signals;
  panel.connectedCallback();

  let dispatchedPatch = null;
  panel.dispatchPatch = (patch) => {
    dispatchedPatch = patch;
  };

  const button = new FakeElement();
  button.setAttribute("data-expression-negate-path", "root.1");
  button.setClosest(
    "button.fishy-applied-expression-negate-toggle[data-expression-negate-path]",
    button,
  );

  panel._handleClick({
    target: button,
  });

  assert.deepEqual(dispatchedPatch, {
    _map_ui: {
      search: {
        expression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            {
              type: "term",
              term: { kind: "fish", fishId: 912 },
              negated: true,
            },
          ],
        },
        selectedTerms: [
          { kind: "fish-filter", term: "favourite" },
          { kind: "fish", fishId: 912 },
        ],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [912],
        zoneRgbs: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: ["favourite"],
        patchId: null,
        fromPatchId: null,
        toPatchId: null,
        searchExpression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            {
              type: "term",
              term: { kind: "fish", fishId: 912 },
              negated: true,
            },
          ],
        },
      },
    },
  });
});

test("FishyMapSearchPanelElement dispatches drag grouping patches from the applied expression view", async () => {
  const { FishyMapSearchPanelElement } = await loadModule();
  const { shell, panel } = createShellAndPanel(FishyMapSearchPanelElement);
  const signals = createSignals();
  signals._map_ui.search.expression = {
    type: "group",
    operator: "or",
    children: [
      { type: "term", term: { kind: "fish-filter", term: "favourite" } },
      { type: "term", term: { kind: "fish", fishId: 912 } },
    ],
  };
  shell.__fishymapLiveSignals = signals;
  panel.connectedCallback();

  let dispatchedPatch = null;
  panel.dispatchPatch = (patch) => {
    dispatchedPatch = patch;
  };

  const source = new FakeElement();
  source.setAttribute("data-expression-drag-path", "root.0");
  source.setAttribute("draggable", "true");
  source.setClosest("[data-expression-drag-path][draggable='true']", source);

  let dragImageCall = null;

  panel._handleDragStart({
    target: source,
    dataTransfer: {
      effectAllowed: "",
      setData() {},
      setDragImage(image, x, y) {
        dragImageCall = { image, x, y };
      },
    },
  });
  assert.equal(panel.querySelector("#fishymap-search-selection")?.dataset.expressionDragging, "true");
  assert.equal(dragImageCall?.x, 8);
  assert.equal(dragImageCall?.y, 8);
  assert.ok(dragImageCall?.image);
  assert.notEqual(dragImageCall?.image, source);
  assert.equal(dragImageCall?.image?.style?.opacity, undefined);
  assert.equal(dragImageCall?.image?.style?.transform, "scale(0.78)");

  const targetTerm = new FakeElement();
  targetTerm.setAttribute("data-expression-drop-node-path", "root.1");
  targetTerm.setClosest("[data-expression-drop-node-path]", targetTerm);

  let prevented = false;
  panel._handleDragOver({
    target: targetTerm,
    preventDefault() {
      prevented = true;
    },
    dataTransfer: {
      dropEffect: "",
    },
  });

  assert.equal(prevented, true);
  assert.equal(targetTerm.dataset.expressionDropMode, "group");

  panel._handleDrop({
    preventDefault() {},
  });

  assert.deepEqual(dispatchedPatch, {
    _map_ui: {
      search: {
        expression: {
          type: "group",
          operator: "or",
          children: [
            {
              type: "group",
              operator: "and",
              children: [
                { type: "term", term: { kind: "fish", fishId: 912 } },
                { type: "term", term: { kind: "fish-filter", term: "favourite" } },
              ],
            },
          ],
        },
        selectedTerms: [
          { kind: "fish", fishId: 912 },
          { kind: "fish-filter", term: "favourite" },
        ],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [912],
        zoneRgbs: [],
        semanticFieldIdsByLayer: {},
        fishFilterTerms: ["favourite"],
        patchId: null,
        fromPatchId: null,
        toPatchId: null,
        searchExpression: {
          type: "group",
          operator: "or",
          children: [
            {
              type: "group",
              operator: "and",
              children: [
                { type: "term", term: { kind: "fish", fishId: 912 } },
                { type: "term", term: { kind: "fish-filter", term: "favourite" } },
              ],
            },
          ],
        },
      },
    },
  });
  assert.equal(source.dataset.dragging, undefined);
  assert.equal(targetTerm.dataset.expressionDropMode, undefined);
  assert.equal(panel.querySelector("#fishymap-search-selection")?.dataset.expressionDragging, undefined);
});

test("FishyMapSearchPanelElement dispatches subgroup move patches from the applied expression view", async () => {
  const { FishyMapSearchPanelElement } = await loadModule();
  const { shell, panel } = createShellAndPanel(FishyMapSearchPanelElement);
  const signals = createSignals();
  signals._map_ui.search.expression = {
    type: "group",
    operator: "or",
    children: [
      {
        type: "group",
        operator: "and",
        children: [
          { type: "term", term: { kind: "fish-filter", term: "favourite" } },
          { type: "term", term: { kind: "fish", fishId: 912 } },
        ],
      },
      {
        type: "group",
        operator: "or",
        children: [{ type: "term", term: { kind: "zone", zoneRgb: 123 } }],
      },
    ],
  };
  shell.__fishymapLiveSignals = signals;
  panel.connectedCallback();

  let dispatchedPatch = null;
  panel.dispatchPatch = (patch) => {
    dispatchedPatch = patch;
  };

  const sourceGroup = new FakeElement();
  sourceGroup.setAttribute("data-expression-drag-path", "root.0");
  sourceGroup.setAttribute("draggable", "true");
  sourceGroup.setClosest("[data-expression-drag-path][draggable='true']", sourceGroup);

  panel._handleDragStart({
    target: sourceGroup,
    dataTransfer: {
      effectAllowed: "",
      setData() {},
    },
  });

  const targetGroup = new FakeElement();
  targetGroup.setAttribute("data-expression-drop-group-path", "root.1");
  targetGroup.setClosest("[data-expression-drop-group-path]", targetGroup);

  let prevented = false;
  panel._handleDragOver({
    target: targetGroup,
    preventDefault() {
      prevented = true;
    },
    dataTransfer: {
      dropEffect: "",
    },
  });

  assert.equal(prevented, true);
  assert.equal(targetGroup.dataset.expressionDropMode, "move");

  panel._handleDrop({
    preventDefault() {},
  });

  assert.deepEqual(dispatchedPatch, {
    _map_ui: {
      search: {
        expression: {
          type: "group",
          operator: "or",
          children: [
            {
              type: "group",
              operator: "or",
              children: [
                { type: "term", term: { kind: "zone", zoneRgb: 123 } },
                {
                  type: "group",
                  operator: "and",
                  children: [
                    { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                    { type: "term", term: { kind: "fish", fishId: 912 } },
                  ],
                },
              ],
            },
          ],
        },
        selectedTerms: [
          { kind: "zone", zoneRgb: 123 },
          { kind: "fish-filter", term: "favourite" },
          { kind: "fish", fishId: 912 },
        ],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [912],
        zoneRgbs: [123],
        semanticFieldIdsByLayer: { zone_mask: [123] },
        fishFilterTerms: ["favourite"],
        patchId: null,
        fromPatchId: null,
        toPatchId: null,
        searchExpression: {
          type: "group",
          operator: "or",
          children: [
            {
              type: "group",
              operator: "or",
              children: [
                { type: "term", term: { kind: "zone", zoneRgb: 123 } },
                {
                  type: "group",
                  operator: "and",
                  children: [
                    { type: "term", term: { kind: "fish-filter", term: "favourite" } },
                    { type: "term", term: { kind: "fish", fishId: 912 } },
                  ],
                },
              ],
            },
          ],
        },
      },
    },
  });
  assert.equal(sourceGroup.dataset.dragging, undefined);
  assert.equal(targetGroup.dataset.expressionDropMode, undefined);
});

test("FishyMapSearchPanelElement dispatches slot insertion patches from the applied expression view", async () => {
  const { FishyMapSearchPanelElement } = await loadModule();
  const { shell, panel } = createShellAndPanel(FishyMapSearchPanelElement);
  const signals = createSignals();
  signals._map_ui.search.expression = {
    type: "group",
    operator: "or",
    children: [
      { type: "term", term: { kind: "fish-filter", term: "favourite" } },
      { type: "term", term: { kind: "fish", fishId: 912 } },
      { type: "term", term: { kind: "zone", zoneRgb: 123 } },
    ],
  };
  shell.__fishymapLiveSignals = signals;
  panel.connectedCallback();

  let dispatchedPatch = null;
  panel.dispatchPatch = (patch) => {
    dispatchedPatch = patch;
  };

  const source = new FakeElement();
  source.setAttribute("data-expression-drag-path", "root.0");
  source.setAttribute("draggable", "true");
  source.setClosest("[data-expression-drag-path][draggable='true']", source);

  panel._handleDragStart({
    target: source,
    dataTransfer: {
      effectAllowed: "",
      setData() {},
    },
  });

  const slot = new FakeElement();
  slot.setAttribute("data-expression-drop-slot-group-path", "root");
  slot.setAttribute("data-expression-drop-slot-index", "2");
  slot.setClosest(
    "[data-expression-drop-slot-group-path][data-expression-drop-slot-index]",
    slot,
  );

  let prevented = false;
  panel._handleDragOver({
    target: slot,
    preventDefault() {
      prevented = true;
    },
    dataTransfer: {
      dropEffect: "",
    },
  });

  assert.equal(prevented, true);
  assert.equal(slot.dataset.expressionDropMode, "insert");

  panel._handleDrop({
    preventDefault() {},
  });

  assert.deepEqual(dispatchedPatch, {
    _map_ui: {
      search: {
        expression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish", fishId: 912 } },
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            { type: "term", term: { kind: "zone", zoneRgb: 123 } },
          ],
        },
        selectedTerms: [
          { kind: "fish", fishId: 912 },
          { kind: "fish-filter", term: "favourite" },
          { kind: "zone", zoneRgb: 123 },
        ],
      },
    },
    _map_bridged: {
      filters: {
        fishIds: [912],
        zoneRgbs: [123],
        semanticFieldIdsByLayer: { zone_mask: [123] },
        fishFilterTerms: ["favourite"],
        patchId: null,
        fromPatchId: null,
        toPatchId: null,
        searchExpression: {
          type: "group",
          operator: "or",
          children: [
            { type: "term", term: { kind: "fish", fishId: 912 } },
            { type: "term", term: { kind: "fish-filter", term: "favourite" } },
            { type: "term", term: { kind: "zone", zoneRgb: 123 } },
          ],
        },
      },
    },
  });
  assert.equal(source.dataset.dragging, undefined);
  assert.equal(slot.dataset.expressionDropMode, undefined);
});

process.on("exit", () => {
  globalThis.HTMLElement = originalHTMLElement;
  globalThis.document = originalDocument;
  globalThis.requestAnimationFrame = originalRequestAnimationFrame;
  globalThis.cancelAnimationFrame = originalCancelAnimationFrame;
});
