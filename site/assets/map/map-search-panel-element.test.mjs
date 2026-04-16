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
    this.textContent = "";
    this.id = "";
    this._innerHTML = "";
    this._queryMap = new Map();
    this._closestMap = new Map();
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
}

function createDocumentStub() {
  const document = new EventTarget();
  document.activeElement = null;
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

process.on("exit", () => {
  globalThis.HTMLElement = originalHTMLElement;
  globalThis.document = originalDocument;
  globalThis.requestAnimationFrame = originalRequestAnimationFrame;
  globalThis.cancelAnimationFrame = originalCancelAnimationFrame;
});
