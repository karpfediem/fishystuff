import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const DATASTAR_STATE_SOURCE = fs.readFileSync(
  new URL("../datastar-state.js", import.meta.url),
  "utf8",
);
const CALCULATOR_PAGE_SOURCE = fs.readFileSync(
  new URL("./calculator-page.js", import.meta.url),
  "utf8",
);

class MemoryStorage {
  constructor(initial = {}) {
    this.map = new Map(Object.entries(initial));
  }

  getItem(key) {
    return this.map.has(key) ? this.map.get(key) : null;
  }

  setItem(key, value) {
    this.map.set(key, String(value));
  }

  removeItem(key) {
    this.map.delete(key);
  }
}

function createContext(localStorageInitial = {}, options = {}) {
  const localStorage = new MemoryStorage(localStorageInitial);
  const location = {
    origin: options.origin || "https://fishystuff.fish",
    pathname: options.pathname || "/calculator/",
    search: options.search || "",
    replace(url) {
      this.replacedUrl = url;
    },
  };
  const toastCalls = [];
  const document = {
    documentElement: {
      lang: options.lang || "en-US",
    },
  };
  const window = {
    location,
    localStorage,
    __fishystuffResolveApiUrl(path) {
      return `https://api.fishystuff.fish${path}`;
    },
    __fishystuffToast: {
      copyText(text, options = {}) {
        toastCalls.push({ type: "copyText", text, options });
      },
      info(message) {
        toastCalls.push({ type: "info", message });
      },
    },
  };
  const context = {
    window,
    document,
    localStorage,
    location,
    URLSearchParams,
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
    Intl,
    console,
    globalThis: null,
    LZString: {
      compressToEncodedURIComponent(value) {
        return `lz:${value}`;
      },
      decompressFromEncodedURIComponent(value) {
        return value.startsWith("lz:") ? value.slice(3) : value;
      },
    },
  };
  context.globalThis = context;
  vm.runInNewContext(DATASTAR_STATE_SOURCE, context, { filename: "datastar-state.js" });
  vm.runInNewContext(CALCULATOR_PAGE_SOURCE, context, { filename: "calculator-page.js" });
  return {
    window,
    document,
    localStorage,
    toastCalls,
  };
}

function defaultSignals() {
  return {
    active: false,
    debug: false,
    level: 0,
    resources: 100,
    food: [],
    buff: [],
    outfit: [],
    discardGrade: "none",
    priceOverrides: {},
    pet1: { skills: [] },
    pet2: { skills: [] },
    pet3: { skills: [] },
    pet4: { skills: [] },
    pet5: { skills: [] },
    _calculator_ui: {
      distribution_tab: "groups",
    },
    _calculator_actions: {
      copyUrlToken: 0,
      copyShareToken: 0,
      clearToken: 0,
    },
    _defaults: {
      active: false,
      debug: false,
      level: 0,
      resources: 100,
      food: [],
      buff: [],
      outfit: [],
      discardGrade: "none",
      priceOverrides: {},
      pet1: { skills: [] },
      pet2: { skills: [] },
      pet3: { skills: [] },
      pet4: { skills: [] },
      pet5: { skills: [] },
      _calculator_ui: { distribution_tab: "groups" },
      _calculator_actions: {
        copyUrlToken: 0,
        copyShareToken: 0,
        clearToken: 0,
      },
    },
    _calc: {
      auto_fish_time_reduction_text: "72%",
      item_drr_text: "11%",
      zone_name: "Velia Beach",
    },
  };
}

test("calculator restore canonicalizes stored signals", () => {
  const env = createContext({
    calculator: JSON.stringify({
      _active: true,
      _distribution_tab: "loot_flow",
      discardTrashFish: true,
      food: ["item:9359", "", "item:9359"],
      buff: ["item:1", "item:2", "item:1"],
      outfit: ["item:77", ""],
      pet1: {
        skills: ["pet-skill:a", "", "pet-skill:a"],
      },
      priceOverrides: {
        "item:8473": {
          tradePriceCurvePercent: "130",
          basePrice: "8800000",
        },
        invalid: null,
      },
    }),
  });
  const signals = defaultSignals();

  env.window.__fishystuffCalculator.restore(signals);

  assert.equal(signals.active, true);
  assert.equal(signals.discardGrade, "white");
  assert.deepEqual(Array.from(signals.food), ["item:9359"]);
  assert.deepEqual(Array.from(signals.buff), ["item:1", "item:2"]);
  assert.deepEqual(Array.from(signals.outfit), ["item:77"]);
  assert.deepEqual(Array.from(signals.pet1.skills), ["pet-skill:a"]);
  assert.equal(signals._calculator_ui.distribution_tab, "loot_flow");
  assert.deepEqual(JSON.parse(JSON.stringify(signals.priceOverrides)), {
    "8473": {
      tradePriceCurvePercent: 130,
      basePrice: 8800000,
    },
  });
});

test("calculator persist stores canonical page state and excludes transient branches", () => {
  const env = createContext();
  const signals = defaultSignals();
  Object.assign(signals, {
    food: ["item:9359", "", "item:9359"],
    _live: { total_time: "123.45" },
    _calc: { zone_name: "Velia Beach" },
  });

  env.window.__fishystuffCalculator.persist(signals);

  const persisted = JSON.parse(env.localStorage.getItem("calculator"));
  assert.deepEqual(persisted.food, ["item:9359"]);
  assert.deepEqual(persisted._calculator_ui, { distribution_tab: "groups" });
  assert.equal("_live" in persisted, false);
  assert.equal("_calc" in persisted, false);
  assert.equal("_defaults" in persisted, false);
});

test("calculator syncActions handles copy and clear tokens once", () => {
  const env = createContext();
  const signals = defaultSignals();
  Object.assign(signals, {
    active: true,
    food: ["item:9359"],
    _calculator_actions: {
      copyUrlToken: 1,
      copyShareToken: 1,
      clearToken: 1,
    },
  });

  env.localStorage.setItem("calculator", JSON.stringify({ food: ["item:9359"] }));
  env.window.__fishystuffCalculator.restore(signals);
  env.window.__fishystuffCalculator.syncActions(signals);
  env.window.__fishystuffCalculator.syncActions(signals);

  assert.equal(env.toastCalls.length, 3);
  assert.equal(env.toastCalls[0].type, "copyText");
  assert.match(env.toastCalls[0].text, /\?preset=lz:/);
  assert.equal(env.toastCalls[1].type, "copyText");
  assert.match(env.toastCalls[1].text, /FishyStuff Calculator Preset/);
  assert.deepEqual(Array.from(signals.food), []);
  assert.equal(env.localStorage.getItem("calculator"), null);
  assert.deepEqual(signals._calculator_ui, { distribution_tab: "groups" });
});
