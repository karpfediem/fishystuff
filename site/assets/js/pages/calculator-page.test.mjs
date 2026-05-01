import { test } from "bun:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

import { parseFluentMessages } from "../../../scripts/build-i18n.mjs";

const DATASTAR_STATE_SOURCE = fs.readFileSync(
  new URL("../datastar-state.js", import.meta.url),
  "utf8",
);
const DATASTAR_PERSIST_SOURCE = fs.readFileSync(
  new URL("../datastar-persist.js", import.meta.url),
  "utf8",
);
const USER_OVERLAYS_SOURCE = fs.readFileSync(
  new URL("../user-overlays.js", import.meta.url),
  "utf8",
);
const USER_PRESETS_SOURCE = fs.readFileSync(
  new URL("../user-presets.js", import.meta.url),
  "utf8",
);
const PRESET_PREVIEWS_SOURCE = fs.readFileSync(
  new URL("../preset-previews.js", import.meta.url),
  "utf8",
);
const CALCULATOR_PAGE_SOURCE = fs.readFileSync(
  new URL("./calculator-page.js", import.meta.url),
  "utf8",
);
const ENGLISH_MESSAGES = Object.freeze(
  parseFluentMessages(
    fs.readFileSync(
      new URL("../../../i18n/fluent/en-US/calculator.ftl", import.meta.url),
      "utf8",
    ),
  ),
);

function translateMessage(key, vars = {}) {
  return String(ENGLISH_MESSAGES[key] ?? key).replace(/\{\s*\$([A-Za-z0-9_]+)\s*\}/g, (_match, name) => {
    return Object.prototype.hasOwnProperty.call(vars, name) ? String(vars[name]) : "";
  });
}

function calculatorMessage(key, vars = {}) {
  return translateMessage(`calculator.${key}`, vars);
}

function breakdownTitle(key, vars = {}) {
  return calculatorMessage(`breakdown.title.${key}`, vars);
}

function breakdownSection(key) {
  return calculatorMessage(`breakdown.section.${key}`);
}

function breakdownLabel(key, vars = {}) {
  return calculatorMessage(`breakdown.label.${key}`, vars);
}

function timelineLabel(key) {
  return calculatorMessage(`timeline.${key}`);
}

const DEFAULT_CUSTOM_LAYOUT = Object.freeze([
  Object.freeze([Object.freeze(["overview"])]),
  Object.freeze([Object.freeze(["zone"]), Object.freeze(["session"])]),
  Object.freeze([Object.freeze(["bite_time"]), Object.freeze(["loot"])]),
]);
const DEFAULT_CUSTOM_SECTIONS = Object.freeze([
  "overview",
  "zone",
  "session",
  "bite_time",
  "loot",
]);

function cloneTestValue(value) {
  return JSON.parse(JSON.stringify(value));
}

function defaultCalculatorUiState(overrides = {}) {
  return {
    workspace_tab: "basics",
    distribution_tab: "groups",
    custom_layout: cloneTestValue(DEFAULT_CUSTOM_LAYOUT),
    custom_sections: Array.from(DEFAULT_CUSTOM_SECTIONS),
    ...cloneTestValue(overrides),
  };
}

function defaultCalculatorActionState(overrides = {}) {
  return {
    copyUrlToken: 0,
    copyShareToken: 0,
    saveCalculatorToken: 0,
    discardCalculatorToken: 0,
    saveLayoutToken: 0,
    discardLayoutToken: 0,
    ...cloneTestValue(overrides),
  };
}

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
  const documentListeners = new Map();
  const windowListeners = new Map();
  const timers = new Map();
  let nextTimerId = 1;
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
    body: {
      appendChild() {},
    },
    documentElement: {
      lang: options.lang || "en-US",
    },
    getElementById() {
      return null;
    },
    addEventListener(type, listener) {
      if (!documentListeners.has(type)) {
        documentListeners.set(type, []);
      }
      documentListeners.get(type).push(listener);
    },
    removeEventListener(type, listener) {
      const current = documentListeners.get(type) || [];
      documentListeners.set(
        type,
        current.filter((candidate) => candidate !== listener),
      );
    },
    dispatchEvent(event) {
      for (const listener of documentListeners.get(event.type) || []) {
        listener(event);
      }
    },
  };
  const window = {
    location,
    localStorage,
    addEventListener(type, listener) {
      if (!windowListeners.has(type)) {
        windowListeners.set(type, []);
      }
      windowListeners.get(type).push(listener);
    },
    removeEventListener(type, listener) {
      const current = windowListeners.get(type) || [];
      windowListeners.set(
        type,
        current.filter((candidate) => candidate !== listener),
      );
    },
    dispatchEvent(event) {
      for (const listener of windowListeners.get(event.type) || []) {
        listener(event);
      }
    },
    __fishystuffResolveApiUrl(path) {
      return `https://api.fishystuff.fish${path}`;
    },
    __fishystuffResolveCdnUrl(path) {
      return `https://cdn.fishystuff.fish${path}`;
    },
    __fishystuffToast: {
      copyText(text, options = {}) {
        toastCalls.push({ type: "copyText", text, options });
      },
      info(message) {
        toastCalls.push({ type: "info", message });
      },
    },
    __fishystuffLanguage: {
      t(key, vars = {}) {
        return translateMessage(key, vars);
      },
      current() {
        const locale = options.locale || options.lang || "en-US";
        return {
          contentLang: options.contentLang || options.lang || "en-US",
          locale,
          apiLang: options.apiLang || "en",
        };
      },
      apply() {},
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
    HTMLElement: class HTMLElement {},
    Map,
    Set,
    Intl,
    console,
    CustomEvent: class CustomEvent {
      constructor(type, options = {}) {
        this.type = type;
        this.detail = options.detail;
      }
    },
    globalThis: null,
    setTimeout(callback) {
      const id = nextTimerId;
      nextTimerId += 1;
      timers.set(id, callback);
      return id;
    },
    clearTimeout(id) {
      timers.delete(id);
    },
    LZString: {
      compressToEncodedURIComponent(value) {
        return `lz:${value}`;
      },
      decompressFromEncodedURIComponent(value) {
        return value.startsWith("lz:") ? value.slice(3) : value;
      },
    },
  };
  window.HTMLElement = context.HTMLElement;
  context.globalThis = context;
  vm.runInNewContext(DATASTAR_STATE_SOURCE, context, { filename: "datastar-state.js" });
  vm.runInNewContext(DATASTAR_PERSIST_SOURCE, context, { filename: "datastar-persist.js" });
  vm.runInNewContext(USER_OVERLAYS_SOURCE, context, { filename: "user-overlays.js" });
  vm.runInNewContext(PRESET_PREVIEWS_SOURCE, context, { filename: "preset-previews.js" });
  vm.runInNewContext(USER_PRESETS_SOURCE, context, { filename: "user-presets.js" });
  vm.runInNewContext(CALCULATOR_PAGE_SOURCE, context, { filename: "calculator-page.js" });
  return {
    window,
    document,
    localStorage,
    toastCalls,
    flushTimers() {
      const pending = Array.from(timers.values());
      timers.clear();
      for (const callback of pending) {
        callback();
      }
    },
  };
}

function defaultSignals() {
  return {
    active: false,
    fishingMode: "rod",
    debug: false,
    level: 0,
    resources: 100,
    food: [],
    buff: [],
    outfit: [],
    discardGrade: "none",
    priceOverrides: {},
    overlay: { zones: {} },
    pet1: { skills: [] },
    pet2: { skills: [] },
    pet3: { skills: [] },
    pet4: { skills: [] },
    pet5: { skills: [] },
    _calculator_ui: defaultCalculatorUiState(),
    _calculator_actions: defaultCalculatorActionState(),
    _defaults: {
      active: false,
      fishingMode: "rod",
      debug: false,
      level: 0,
      resources: 100,
      food: [],
      buff: [],
      outfit: [],
      discardGrade: "none",
      priceOverrides: {},
      overlay: { zones: {} },
      pet1: { skills: [] },
      pet2: { skills: [] },
      pet3: { skills: [] },
      pet4: { skills: [] },
      pet5: { skills: [] },
      _calculator_ui: defaultCalculatorUiState(),
      _calculator_actions: defaultCalculatorActionState(),
    },
    _calc: {
      auto_fish_time_reduction_text: "72%",
      item_drr_text: "11%",
      zone_name: "Velia Beach",
    },
  };
}

function patchCalculatorSignals(env, patch, options = {}) {
  env.window.__fishystuffCalculator.patchSignals(patch, options);
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: cloneTestValue(patch),
  });
}

test("calculator restore canonicalizes stored signals", () => {
  const env = createContext({
    "fishystuff.calculator.data.v1": JSON.stringify({
      _active: true,
      discardTrashFish: true,
      food: ["item:9359", "", "item:9359"],
      buff: ["item:1", "item:2", "item:1"],
      outfit: ["item:77", ""],
      pet1: {
        packLeader: "true",
        skills: ["pet-skill:a", "", "pet-skill:a"],
      },
      pet2: {
        packLeader: true,
        skills: [],
      },
      priceOverrides: {
        "item:8473": {
          tradePriceCurvePercent: "130",
          basePrice: "8800000",
        },
        invalid: null,
      },
    }),
    "fishystuff.calculator.ui.v1": JSON.stringify({
      workspace_tab: "basics",
      distribution_tab: "loot_flow",
      custom_layout: [[["zone"], ["distribution"]], [["missing"]]],
      custom_sections: ["zone", "distribution", "zone", "missing"],
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
  assert.equal(signals.pet1.packLeader, true);
  assert.equal(signals.pet2.packLeader, false);
  assert.equal(signals._calculator_ui.distribution_tab, "loot_flow");
  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui.custom_layout)), [[["zone"], ["distribution"]]]);
  assert.deepEqual(Array.from(signals._calculator_ui.custom_sections), ["zone", "distribution"]);
  assert.deepEqual(JSON.parse(JSON.stringify(signals.priceOverrides)), {
    "8473": {
      tradePriceCurvePercent: 130,
      basePrice: 8800000,
    },
  });
  assert.deepEqual(env.window.__fishystuffUserOverlays.priceOverrides(), {
    "8473": {
      tradePriceCurvePercent: 130,
      basePrice: 8800000,
    },
  });
  assert.equal(env.window.__fishystuffCalculator.signalObject(), signals);
});

test("pack leader change applies exclusivity without scheduling full pet card replacement", () => {
  const env = createContext();
  const signals = defaultSignals();
  signals.pet1 = { tier: "5", packLeader: true, skills: [] };
  signals.pet2 = { tier: "5", packLeader: false, skills: [] };

  env.window.__fishystuffCalculator.restore(signals);
  env.window.__fishystuffCalculator.applyPackLeaderChange({ checked: true }, 2);

  assert.equal(signals.pet1.packLeader, false);
  assert.equal(signals.pet2.packLeader, true);
  assert.match(
    env.window.__fishystuffCalculator.evalUrl({
      pet1: { packLeader: false },
      pet2: { packLeader: true },
    }),
    /[?&]pet_cards=false\b/,
  );
});

test("pack leader change keeps full pet card replacement for other pending pet edits", () => {
  const env = createContext();
  const signals = defaultSignals();
  signals.pet1 = { tier: "5", packLeader: true, skills: [] };
  signals.pet2 = { tier: "5", packLeader: false, skills: [] };

  env.window.__fishystuffCalculator.restore(signals);
  patchCalculatorSignals(env, { pet1: { tier: "4" } });
  env.window.__fishystuffCalculator.applyPackLeaderChange({ checked: true }, 2);

  assert.doesNotMatch(
    env.window.__fishystuffCalculator.evalUrl({
      pet1: { tier: "4" },
      pet2: { packLeader: true },
    }),
    /[?&]pet_cards=false\b/,
  );
});

test("pack leader change clears stale non-tier-five selections", () => {
  const env = createContext();
  const signals = defaultSignals();
  signals.pet1 = { tier: "4", packLeader: true, skills: [] };

  env.window.__fishystuffCalculator.restore(signals);
  env.window.__fishystuffCalculator.applyPackLeaderChange({ checked: true }, 1);

  assert.equal(signals.pet1.packLeader, false);
});

test("session duration changes request signal-only eval updates", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffCalculator.restore(signals);

  const url = env.window.__fishystuffCalculator.evalUrl({ timespanAmount: 3 });
  assert.match(url, /[?&]pet_cards=false\b/);
  assert.doesNotMatch(url, /[?&]target_fish_select=true\b/);
  assert.doesNotMatch(url, /[?&]trade_origin_select=true\b/);
  assert.doesNotMatch(url, /[?&]trade_destination_select=true\b/);
});

test("zone changes request zone-dependent fish and trade controls", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffCalculator.restore(signals);

  const url = env.window.__fishystuffCalculator.evalUrl({ zone: "240,74,74" });
  assert.match(url, /[?&]pet_cards=false\b/);
  assert.match(url, /[?&]target_fish_select=true\b/);
  assert.match(url, /[?&]trade_origin_select=true\b/);
  assert.match(url, /[?&]trade_destination_select=true\b/);
});

test("trade origin changes request only the trade destination control", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffCalculator.restore(signals);

  const url = env.window.__fishystuffCalculator.evalUrl({ tradeOriginRegion: "740" });
  assert.match(url, /[?&]pet_cards=false\b/);
  assert.doesNotMatch(url, /[?&]target_fish_select=true\b/);
  assert.doesNotMatch(url, /[?&]trade_origin_select=true\b/);
  assert.match(url, /[?&]trade_destination_select=true\b/);
});

test("calculator eval URL classifies direct Datastar patch payloads", () => {
  const env = createContext();

  const durationUrl = env.window.__fishystuffCalculator.evalUrl({ timespanAmount: 10 });
  assert.match(durationUrl, /[?&]pet_cards=false\b/);
  assert.doesNotMatch(durationUrl, /[?&]target_fish_select=true\b/);
  assert.doesNotMatch(durationUrl, /[?&]trade_origin_select=true\b/);
  assert.doesNotMatch(durationUrl, /[?&]trade_destination_select=true\b/);

  const zoneUrl = env.window.__fishystuffCalculator.evalUrl({ zone: "240,74,74" });
  assert.match(zoneUrl, /[?&]pet_cards=false\b/);
  assert.match(zoneUrl, /[?&]target_fish_select=true\b/);
  assert.match(zoneUrl, /[?&]trade_origin_select=true\b/);
  assert.match(zoneUrl, /[?&]trade_destination_select=true\b/);

  const tradeOriginUrl = env.window.__fishystuffCalculator.evalUrl({ tradeOriginRegion: "740" });
  assert.match(tradeOriginUrl, /[?&]pet_cards=false\b/);
  assert.doesNotMatch(tradeOriginUrl, /[?&]trade_origin_select=true\b/);
  assert.match(tradeOriginUrl, /[?&]trade_destination_select=true\b/);
  assert.match(tradeOriginUrl, /[?&]trade_origin_region=740\b/);

  const tradeDestinationUrl = env.window.__fishystuffCalculator.evalUrl({ tradeDestinationNpc: "200" });
  assert.match(tradeDestinationUrl, /[?&]pet_cards=false\b/);
  assert.doesNotMatch(tradeDestinationUrl, /[?&]trade_destination_select=true\b/);
  assert.match(tradeDestinationUrl, /[?&]trade_destination_npc=200\b/);

  const customTradeDestinationUrl = env.window.__fishystuffCalculator.evalUrl({ tradeDestinationNpc: "custom:123.4" });
  assert.match(customTradeDestinationUrl, /[?&]pet_cards=false\b/);
  assert.doesNotMatch(customTradeDestinationUrl, /[?&]trade_destination_select=true\b/);
  assert.match(customTradeDestinationUrl, /[?&]trade_destination_npc=custom%3A123\.4\b/);

  const petUrl = env.window.__fishystuffCalculator.evalUrl({ pet1: { tier: "4" } });
  assert.doesNotMatch(petUrl, /[?&]pet_cards=false\b/);
  assert.doesNotMatch(petUrl, /[?&]target_fish_select=true\b/);
  assert.doesNotMatch(petUrl, /[?&]trade_origin_select=true\b/);
  assert.doesNotMatch(petUrl, /[?&]trade_destination_select=true\b/);

  const packLeaderUrl = env.window.__fishystuffCalculator.evalUrl({ pet1: { packLeader: true } });
  assert.match(packLeaderUrl, /[?&]pet_cards=false\b/);
});

test("calculator restore keeps the current tab while restoring trade, food, and buffs UI state", () => {
  const env = createContext({
    "fishystuff.calculator.ui.v1": JSON.stringify({
      workspace_tab: "basics",
      distribution_tab: "groups",
      custom_layout: [[["overview"], ["trade"]], [["food", "buffs"], ["missing"]]],
      custom_sections: ["overview", "trade", "food", "buffs", "missing"],
    }),
  });
  const signals = defaultSignals();

  env.window.__fishystuffCalculator.restore(signals);

  assert.equal(signals._calculator_ui.distribution_tab, "groups");
  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui.custom_layout)), [
    [["overview"], ["trade"]],
    [["food", "buffs"]],
  ]);
  assert.deepEqual(Array.from(signals._calculator_ui.custom_sections), [
    "overview",
    "trade",
    "food",
    "buffs",
  ]);
});

test("calculator restore ignores incomplete custom UI state without a custom layout", () => {
  const env = createContext({
    "fishystuff.calculator.ui.v1": JSON.stringify({
      workspace_tab: "basics",
      distribution_tab: "groups",
      custom_sections: ["trade", "food", "buffs"],
    }),
  });
  const signals = defaultSignals();

  env.window.__fishystuffCalculator.restore(signals);

  assert.equal(signals._calculator_ui.distribution_tab, "groups");
  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui.custom_layout)), cloneTestValue(DEFAULT_CUSTOM_LAYOUT));
  assert.deepEqual(Array.from(signals._calculator_ui.custom_sections), Array.from(DEFAULT_CUSTOM_SECTIONS));
});

test("calculator restore leaves initial shell state intact when storage is empty", () => {
  const env = createContext();
  const signals = {
    _loading: true,
    _calculator_ui: defaultCalculatorUiState(),
    _calculator_actions: defaultCalculatorActionState(),
  };

  env.window.__fishystuffCalculator.restore(signals);

  assert.equal(signals._user_presets.collections["calculator-presets"].hasCurrent, false);
  assert.equal(signals._user_presets.collections["calculator-layouts"].hasCurrent, false);
  const comparableSignals = JSON.parse(JSON.stringify(signals));
  delete comparableSignals._user_presets;
  assert.deepEqual(comparableSignals, {
    _loading: true,
    overlay: {
      zones: {},
    },
    priceOverrides: {},
    _calculator_ui: defaultCalculatorUiState(),
    _calculator_actions: defaultCalculatorActionState(),
  });
});

test("calculator custom helpers keep custom sections ordered and placeable", () => {
  const env = createContext();
  const calculator = env.window.__fishystuffCalculator;

  assert.deepEqual(
    Array.from(calculator.toggleCustomSection(undefined, "distribution")),
    ["overview", "zone", "session", "bite_time", "loot", "distribution"],
  );
  assert.deepEqual(
    Array.from(calculator.toggleCustomSection(["overview", "zone"], "overview")),
    ["zone"],
  );
  assert.deepEqual(
    Array.from(calculator.addCustomSection(["overview"], "overview")),
    ["overview"],
  );
  assert.deepEqual(
    Array.from(calculator.placeCustomSection(["overview"], "loot", "overview", "before")),
    ["loot", "overview"],
  );
  assert.deepEqual(
    Array.from(calculator.placeCustomSection(["overview", "zone"], "overview", "zone", "after")),
    ["zone", "overview"],
  );
  assert.deepEqual(
    JSON.parse(JSON.stringify(calculator.toggleCustomSection({
      workspace_tab: "basics",
      distribution_tab: "groups",
      custom_layout: [[["overview"]], [["zone"]]],
      custom_sections: ["overview", "zone"],
    }, "distribution"))),
    {
      workspace_tab: "basics",
      distribution_tab: "groups",
      custom_layout: [[["overview"]], [["zone"]], [["distribution"]]],
      custom_sections: ["overview", "zone", "distribution"],
    },
  );
  assert.deepEqual(
    JSON.parse(JSON.stringify(calculator.toggleCustomSection({
      workspace_tab: "basics",
      distribution_tab: "groups",
      custom_layout: [[["overview"]], [["loot"]]],
      custom_sections: ["overview", "loot"],
    }, "trade"))),
    {
      workspace_tab: "basics",
      distribution_tab: "groups",
      custom_layout: [[["overview"]], [["loot"]], [["trade"]]],
      custom_sections: ["overview", "loot", "trade"],
    },
  );
  assert.deepEqual(
    JSON.parse(JSON.stringify(calculator.toggleCustomSection({
      workspace_tab: "basics",
      distribution_tab: "groups",
      custom_layout: [[["overview"]]],
      custom_sections: ["overview"],
    }, "overview"))),
    {
      workspace_tab: "basics",
      distribution_tab: "groups",
      custom_layout: [],
      custom_sections: [],
    },
  );
  const uiState = {
    workspace_tab: "basics",
    distribution_tab: "loot_flow",
    custom_layout: [[["overview"]]],
    custom_sections: ["overview"],
  };
  assert.equal(calculator.toggleCustomSectionInPlace(uiState, "zone"), uiState);
  assert.deepEqual(JSON.parse(JSON.stringify(uiState)), {
    workspace_tab: "basics",
    distribution_tab: "loot_flow",
    custom_layout: [[["overview"]], [["zone"]]],
    custom_sections: ["overview", "zone"],
  });
  const selectedCustomState = {
    workspace_tab: "basics",
    distribution_tab: "groups",
    custom_layout: [[["overview"]], [["loot"]]],
    custom_sections: ["overview", "loot"],
  };
  assert.equal(calculator.toggleCustomSectionInPlace(selectedCustomState, "trade"), selectedCustomState);
  assert.deepEqual(JSON.parse(JSON.stringify(selectedCustomState)), {
    workspace_tab: "basics",
    distribution_tab: "groups",
    custom_layout: [[["overview"]], [["loot"]], [["trade"]]],
    custom_sections: ["overview", "loot", "trade"],
  });
  const removableCustomState = {
    workspace_tab: "basics",
    distribution_tab: "groups",
    custom_layout: [[["overview"]], [["trade"]]],
    custom_sections: ["overview", "trade"],
  };
  assert.equal(calculator.removeCustomSectionInPlace(removableCustomState, "trade"), removableCustomState);
  assert.deepEqual(JSON.parse(JSON.stringify(removableCustomState)), {
    workspace_tab: "basics",
    distribution_tab: "groups",
    custom_layout: [[["overview"]]],
    custom_sections: ["overview"],
  });
  assert.equal(calculator.isCustomSection(["overview", "zone"], "zone"), true);
  assert.equal(calculator.isCustomSection({
    custom_layout: [[["overview", "zone"]]],
    custom_sections: ["overview", "zone"],
  }, "zone"), true);
  assert.equal(
    calculator.sectionVisibleInWorkspace("mode", {
      workspace_tab: "custom",
      custom_layout: [[["overview"]]],
      custom_sections: ["overview"],
    }),
    false,
  );
  assert.equal(
    calculator.sectionVisibleInWorkspace("overview", {
      workspace_tab: "custom",
      custom_layout: [[["overview"]]],
      custom_sections: ["overview"],
    }),
    true,
  );
  assert.equal(
    calculator.sectionVisibleInWorkspace("overview", {
      workspace_tab: "basics",
      custom_sections: [],
    }),
    true,
  );
  assert.equal(
    calculator.sectionVisibleInWorkspace("session", {
      workspace_tab: "basics",
      custom_sections: [],
    }),
    true,
  );
  assert.equal(
    calculator.sectionVisibleInWorkspace("zone", {
      workspace_tab: "basics",
      custom_sections: ["overview"],
    }),
    true,
  );
  assert.equal(
    calculator.sectionVisibleInWorkspace("catch_time", {
      workspace_tab: "advanced",
      custom_sections: [],
    }),
    true,
  );
  assert.equal(
    calculator.sectionVisibleInWorkspace("mode", {
      workspace_tab: "advanced",
      custom_sections: [],
    }),
    true,
  );
  assert.equal(
    calculator.sectionVisibleInWorkspace("trade", {
      workspace_tab: "trade",
      custom_sections: ["overview"],
    }),
    true,
  );
  assert.equal(
    calculator.sectionVisibleInWorkspace("trade", {
      workspace_tab: "loot",
      custom_sections: ["trade"],
    }),
    false,
  );
  assert.equal(
    calculator.sectionVisibleInWorkspace("food", {
      workspace_tab: "loadout",
      custom_sections: ["overview"],
    }),
    true,
  );
  assert.equal(
    calculator.sectionVisibleInWorkspace("trade", {
      workspace_tab: "loadout",
      custom_sections: ["trade"],
    }),
    false,
  );
});

test("calculator reset layout restores the default custom mosaic while keeping the selected tab", () => {
  const env = createContext();
  const calculator = env.window.__fishystuffCalculator;
  const uiState = {
    workspace_tab: "basics",
    distribution_tab: "loot_flow",
    custom_layout: [[["overview"], ["distribution"]], [["food", "buffs"]]],
    custom_sections: ["overview", "distribution", "food", "buffs"],
  };

  assert.deepEqual(JSON.parse(JSON.stringify(calculator.resetCalculatorLayout(uiState))), {
    workspace_tab: "basics",
    distribution_tab: "loot_flow",
    custom_layout: cloneTestValue(DEFAULT_CUSTOM_LAYOUT),
    custom_sections: Array.from(DEFAULT_CUSTOM_SECTIONS),
  });

  assert.equal(calculator.resetCalculatorLayoutInPlace(uiState), uiState);
  assert.deepEqual(JSON.parse(JSON.stringify(uiState)), {
    workspace_tab: "basics",
    distribution_tab: "loot_flow",
    custom_layout: cloneTestValue(DEFAULT_CUSTOM_LAYOUT),
    custom_sections: Array.from(DEFAULT_CUSTOM_SECTIONS),
  });
});

test("calculator effective activity normalizes mode names and forces harpoon to active", () => {
  const env = createContext();
  const calculator = env.window.__fishystuffCalculator;

  assert.equal(calculator.normalizeFishingMode("HOTSPOT"), "hotspot");
  assert.equal(calculator.normalizeFishingMode("unknown"), "rod");
  assert.equal(calculator.effectiveActivity("rod", false), false);
  assert.equal(calculator.effectiveActivity("harpoon", false), true);
});

test("calculator layout presets register a shared adapter and apply layout-only changes", () => {
  const env = createContext();
  const signals = defaultSignals();
  signals._calculator_ui.distribution_tab = "target_fish";

  env.window.__fishystuffCalculator.restore(signals);

  const preset = env.window.__fishystuffUserPresets.createPreset("calculator-layouts", {
    name: "Split loot",
    payload: {
      custom_layout: [[["overview"], ["distribution"]], [["loot"]]],
    },
    select: false,
  });

  env.window.__fishystuffUserPresets.activatePreset("calculator-layouts", preset.id);
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _calculator_ui: cloneTestValue(signals._calculator_ui),
    },
  });
  env.flushTimers();

  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui)), {
    workspace_tab: "basics",
    distribution_tab: "target_fish",
    custom_layout: [[["overview"], ["distribution"]], [["loot"]]],
    custom_sections: ["overview", "distribution", "loot"],
  });
  assert.deepEqual(
    JSON.parse(env.localStorage.getItem("fishystuff.calculator.ui.v1")),
    {
      workspace_tab: "basics",
      distribution_tab: "target_fish",
      custom_layout: [[["overview"], ["distribution"]], [["loot"]]],
      custom_sections: ["overview", "distribution", "loot"],
    },
  );
});

test("calculator preset activation switches selection without overwriting the previous preset", () => {
  const env = createContext();
  const signals = defaultSignals();
  env.window.__fishystuffCalculator.restore(signals);

  const alpha = env.window.__fishystuffUserPresets.createPreset("calculator-layouts", {
    name: "Alpha",
    payload: {
      custom_layout: [[["mode"]], [["overview"]]],
    },
    select: true,
  });
  const beta = env.window.__fishystuffUserPresets.createPreset("calculator-layouts", {
    name: "Beta",
    payload: {
      custom_layout: [[["zone"]], [["loot"]]],
    },
    select: false,
  });

  env.window.__fishystuffUserPresets.activatePreset("calculator-layouts", beta.id);
  env.flushTimers();

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-layouts"), beta.id);
  assert.deepEqual(env.window.__fishystuffUserPresets.preset("calculator-layouts", alpha.id)?.payload, {
    custom_layout: [[["mode"]], [["overview"]]],
  });
  assert.deepEqual(env.window.__fishystuffUserPresets.preset("calculator-layouts", beta.id)?.payload, {
    custom_layout: [[["zone"]], [["loot"]]],
  });
});

test("calculator layout changes create a current modified preset from default without saving", () => {
  const env = createContext();
  const signals = defaultSignals();
  env.window.__fishystuffCalculator.restore(signals);

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-layouts"), "");
  assert.equal(env.window.__fishystuffUserPresets.presets("calculator-layouts").length, 0);

  env.window.__fishystuffCalculator.toggleCustomSectionInPlace(signals._calculator_ui, "trade");
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _calculator_ui: cloneTestValue(signals._calculator_ui),
    },
  });
  env.flushTimers();

  const selectedId = env.window.__fishystuffUserPresets.selectedPresetId("calculator-layouts");
  const current = env.window.__fishystuffUserPresets.current("calculator-layouts");
  assert.equal(selectedId, "");
  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-layouts"), "default");
  assert.equal(env.window.__fishystuffUserPresets.presets("calculator-layouts").length, 0);
  assert.deepEqual(current?.origin, { kind: "fixed", id: "default" });
  assert.deepEqual(current?.payload, {
    custom_layout: [[["overview"]], [["zone"], ["session"]], [["bite_time"], ["loot"]], [["trade"]]],
  });
});

test("calculator layout changes keep the selected saved preset immutable until explicit save", () => {
  const env = createContext();
  const signals = defaultSignals();
  env.window.__fishystuffCalculator.restore(signals);

  const preset = env.window.__fishystuffUserPresets.createPreset("calculator-layouts", {
    name: "Layout 1",
    payload: env.window.__fishystuffCalculator.layoutPresetPayload(signals._calculator_ui),
    select: true,
  });

  env.window.__fishystuffCalculator.toggleCustomSectionInPlace(signals._calculator_ui, "trade");
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _calculator_ui: cloneTestValue(signals._calculator_ui),
    },
  });
  env.flushTimers();

  const selectedPreset = env.window.__fishystuffUserPresets.selectedPreset("calculator-layouts");
  assert.equal(selectedPreset?.id, preset.id);
  assert.deepEqual(selectedPreset?.payload, {
    custom_layout: cloneTestValue(DEFAULT_CUSTOM_LAYOUT),
  });
  assert.deepEqual(env.window.__fishystuffUserPresets.current("calculator-layouts")?.payload, {
    custom_layout: [[["overview"]], [["zone"], ["session"]], [["bite_time"], ["loot"]], [["trade"]]],
  });

  const saved = env.window.__fishystuffUserPresets.saveCurrentToSelectedPreset("calculator-layouts");
  assert.equal(saved.id, preset.id);
  assert.deepEqual(saved.payload, {
    custom_layout: [[["overview"]], [["zone"], ["session"]], [["bite_time"], ["loot"]], [["trade"]]],
  });
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-layouts"), null);
});

test("calculator presets apply durable inputs without changing the layout preset state", () => {
  const env = createContext();
  const signals = defaultSignals();
  signals._calculator_ui = defaultCalculatorUiState({
    custom_layout: [[["overview"]], [["trade"]]],
    custom_sections: ["overview", "trade"],
  });
  env.window.__fishystuffCalculator.restore(signals);

  const preset = env.window.__fishystuffUserPresets.createPreset("calculator-presets", {
    name: "Harpoon setup",
    payload: {
      active: true,
      fishingMode: "harpoon",
      level: 5,
      resources: 44,
      zone: "240,74,74",
      outfit: ["item:14330"],
      food: ["item:9359", "", "item:9359"],
      buff: ["item:1"],
      pet1: {
        tier: "5",
        skills: ["49023", "49014", "49017"],
      },
      priceOverrides: {
        "item:8473": {
          basePrice: "8800000",
        },
      },
      _calculator_ui: defaultCalculatorUiState(),
    },
    select: false,
  });

  env.window.__fishystuffUserPresets.activatePreset("calculator-presets", preset.id);
  env.flushTimers();

  assert.equal(signals.active, true);
  assert.equal(signals.fishingMode, "harpoon");
  assert.equal(signals.level, 5);
  assert.equal(signals.resources, 44);
  assert.equal(signals._resources, 44);
  assert.deepEqual(Array.from(signals.outfit), ["item:14330"]);
  assert.deepEqual(Array.from(signals._outfit_slots), ["item:14330"]);
  assert.deepEqual(Array.from(signals.food), ["item:9359"]);
  assert.deepEqual(Array.from(signals._food_slots), ["item:9359"]);
  assert.deepEqual(Array.from(signals.buff), ["item:1"]);
  assert.deepEqual(Array.from(signals._buff_slots), ["item:1"]);
  assert.deepEqual(Array.from(signals.pet1.skills), ["49023", "49014", "49017"]);
  assert.equal(signals._pet1_skill_slot1, "49023");
  assert.equal(signals._pet1_skill_slot2, "49014");
  assert.equal(signals._pet1_skill_slot3, "49017");
  assert.deepEqual(JSON.parse(JSON.stringify(signals.priceOverrides)), {
    "8473": {
      basePrice: 8800000,
    },
  });
  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui)), {
    workspace_tab: "basics",
    distribution_tab: "groups",
    custom_layout: [[["overview"]], [["trade"]]],
    custom_sections: ["overview", "trade"],
  });
  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-presets"), preset.id);
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets"), null);
});

test("calculator restore reapplies the persisted selected calculator preset after init defaults", () => {
  const initial = createContext();
  const initialSignals = defaultSignals();
  initial.window.__fishystuffCalculator.restore(initialSignals);
  const preset = initial.window.__fishystuffUserPresets.createPreset("calculator-presets", {
    name: "AFK setup",
    payload: {
      active: false,
      fishingMode: "rod",
      level: 4,
      resources: 33,
      zone: "10,20,30",
      timespanAmount: 3,
      timespanUnit: "hours",
    },
    select: true,
  });
  const presetStorage = initial.localStorage.getItem(initial.window.__fishystuffUserPresets.STORAGE_KEY);

  const env = createContext({
    [initial.window.__fishystuffUserPresets.STORAGE_KEY]: presetStorage,
    "fishystuff.calculator.data.v1": JSON.stringify({
      active: true,
      fishingMode: "harpoon",
      level: 0,
      resources: 100,
    }),
  });
  const signals = defaultSignals();
  env.window.__fishystuffCalculator.restore(signals);
  const initDefaults = cloneTestValue(signals._defaults);
  Object.assign(signals, cloneTestValue(initDefaults));
  patchCalculatorSignals(env, {
    _loading: false,
    _defaults: initDefaults,
  });

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-presets"), preset.id);
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets"), null);
  assert.equal(signals.active, false);
  assert.equal(signals.fishingMode, "rod");
  assert.equal(signals.level, 4);
  assert.equal(signals.resources, 33);
  assert.equal(signals.zone, "10,20,30");
  assert.equal(signals.timespanAmount, 3);
  assert.equal(signals.timespanUnit, "hours");
});

test("calculator restore keeps selected calculator preset through late server defaults", () => {
  const initial = createContext();
  const initialSignals = defaultSignals();
  initial.window.__fishystuffCalculator.restore(initialSignals);
  const preset = initial.window.__fishystuffUserPresets.createPreset("calculator-presets", {
    name: "Late defaults setup",
    payload: {
      active: false,
      fishingMode: "rod",
      level: 42,
      resources: 15,
      zone: "10,20,30",
      timespanAmount: 3,
      timespanUnit: "hours",
    },
    select: true,
  });
  const presetStorage = initial.localStorage.getItem(initial.window.__fishystuffUserPresets.STORAGE_KEY);

  const env = createContext({
    [initial.window.__fishystuffUserPresets.STORAGE_KEY]: presetStorage,
    "fishystuff.calculator.data.v1": JSON.stringify({
      active: false,
      fishingMode: "rod",
      level: 42,
      resources: 15,
      zone: "10,20,30",
      timespanAmount: 3,
      timespanUnit: "hours",
    }),
  });
  const signals = defaultSignals();
  const initDefaults = cloneTestValue(signals._defaults);
  delete signals._defaults;
  signals.level = "";
  signals.resources = "";
  env.window.__fishystuffCalculator.restore(signals);

  patchCalculatorSignals(env, {
    _defaults: initDefaults,
    level: 42,
    resources: 15,
  });
  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-presets"), preset.id);
  assert.equal(signals.level, 42);

  patchCalculatorSignals(env, {
    ...cloneTestValue(initDefaults),
    _loading: false,
    _defaults: initDefaults,
  });
  env.flushTimers();

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-presets"), preset.id);
  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-presets"), "");
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets"), null);
  assert.equal(signals.level, 42);
  assert.equal(signals.resources, 15);
  assert.equal(JSON.parse(env.localStorage.getItem("fishystuff.calculator.data.v1")).level, 42);
});

test("calculator presets wait for init defaults before tracking default current state", () => {
  const env = createContext();
  const signals = defaultSignals();
  const initDefaults = cloneTestValue(signals._defaults);
  delete signals._defaults;

  env.window.__fishystuffCalculator.restore(signals);

  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets"), null);
  patchCalculatorSignals(env, {
    _loading: false,
    _defaults: initDefaults,
  });

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-presets"), "");
  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-presets"), "default");
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets"), null);
});

test("calculator preset action signals refresh when late defaults make discard possible", () => {
  const initial = createContext();
  const initialSignals = defaultSignals();
  initial.window.__fishystuffCalculator.restore(initialSignals);
  initial.window.__fishystuffUserPresets.trackCurrentPayload("calculator-presets", {
    payload: {
      ...initial.window.__fishystuffCalculator.calculatorPresetPayload(initialSignals),
      level: 42,
      resources: 15,
    },
    origin: { kind: "fixed", id: "default" },
  });
  const presetStorage = initial.localStorage.getItem(initial.window.__fishystuffUserPresets.STORAGE_KEY);

  const env = createContext({
    [initial.window.__fishystuffUserPresets.STORAGE_KEY]: presetStorage,
    "fishystuff.calculator.data.v1": JSON.stringify({
      level: 42,
      resources: 15,
    }),
  });
  const signals = defaultSignals();
  const initDefaults = cloneTestValue(signals._defaults);
  delete signals._defaults;
  signals.level = 42;
  signals.resources = 15;
  env.window.__fishystuffCalculator.restore(signals);

  assert.equal(signals._user_presets.collections["calculator-presets"].hasCurrent, true);
  assert.equal(signals._user_presets.collections["calculator-presets"].canDiscard, false);
  patchCalculatorSignals(env, {
    _loading: false,
    _defaults: initDefaults,
    level: 42,
    resources: 15,
  });

  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets")?.payload?.level, 42);
  assert.equal(env.window.__fishystuffUserPresets.datastarSnapshot().collections["calculator-presets"].canDiscard, true);
  env.window.__fishystuffUserPresets.refreshDatastar();
  assert.equal(signals._user_presets.collections["calculator-presets"].canDiscard, true);
});

test("calculator restore applies the persisted selected layout preset when UI storage is absent", () => {
  const initial = createContext();
  const initialSignals = defaultSignals();
  initial.window.__fishystuffCalculator.restore(initialSignals);
  const preset = initial.window.__fishystuffUserPresets.createPreset("calculator-layouts", {
    name: "Trade layout",
    payload: {
      custom_layout: [[["overview"]], [["trade"]]],
    },
    select: true,
  });
  const presetStorage = initial.localStorage.getItem(initial.window.__fishystuffUserPresets.STORAGE_KEY);

  const env = createContext({
    [initial.window.__fishystuffUserPresets.STORAGE_KEY]: presetStorage,
  });
  const signals = defaultSignals();
  delete signals._calculator_ui;
  env.window.__fishystuffCalculator.restore(signals);
  const initDefaults = cloneTestValue(signals._defaults);
  patchCalculatorSignals(env, {
    _loading: false,
    _defaults: initDefaults,
  });

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-layouts"), preset.id);
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-layouts"), null);
  assert.deepEqual(JSON.parse(JSON.stringify(env.window.__fishystuffCalculator.layoutPresetPayload(signals._calculator_ui))), {
    custom_layout: [[["overview"]], [["trade"]]],
  });
});

test("calculator restore loads the persisted layout working copy over older UI storage", () => {
  const initial = createContext();
  const initialSignals = defaultSignals();
  initial.window.__fishystuffCalculator.restore(initialSignals);
  const preset = initial.window.__fishystuffUserPresets.createPreset("calculator-layouts", {
    name: "Trade layout",
    payload: {
      custom_layout: [[["overview"]], [["trade"]]],
    },
    select: true,
  });
  const presetStorage = initial.localStorage.getItem(initial.window.__fishystuffUserPresets.STORAGE_KEY);

  const env = createContext({
    [initial.window.__fishystuffUserPresets.STORAGE_KEY]: presetStorage,
    "fishystuff.calculator.ui.v1": JSON.stringify(defaultCalculatorUiState()),
  });
  const signals = defaultSignals();
  env.window.__fishystuffCalculator.restore(signals);
  patchCalculatorSignals(env, {
    _loading: false,
    _defaults: cloneTestValue(signals._defaults),
    _calculator_ui: defaultCalculatorUiState(),
  });

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-layouts"), preset.id);
  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-layouts"), "");
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-layouts"), null);
  assert.deepEqual(JSON.parse(JSON.stringify(env.window.__fishystuffCalculator.layoutPresetPayload(signals._calculator_ui))), {
    custom_layout: [[["overview"]], [["trade"]]],
  });
  assert.equal(preset.id.length > 0, true);
});

test("calculator restore keeps selected layout preset and tab after init", () => {
  const initial = createContext();
  const initialSignals = defaultSignals();
  initial.window.__fishystuffCalculator.restore(initialSignals);
  const preset = initial.window.__fishystuffUserPresets.createPreset("calculator-layouts", {
    name: "Distribution layout",
    payload: {
      custom_layout: [[["overview"], ["distribution"]], [["loot"]]],
    },
    select: true,
  });
  const presetStorage = initial.localStorage.getItem(initial.window.__fishystuffUserPresets.STORAGE_KEY);

  const env = createContext({
    [initial.window.__fishystuffUserPresets.STORAGE_KEY]: presetStorage,
    "fishystuff.calculator.ui.v1": JSON.stringify({
      workspace_tab: "basics",
      distribution_tab: "target_fish",
      custom_layout: [[["overview"], ["distribution"]], [["loot"]]],
      custom_sections: ["overview", "distribution", "loot"],
    }),
  });
  const signals = defaultSignals();
  env.window.__fishystuffCalculator.restore(signals);
  patchCalculatorSignals(env, {
    _loading: false,
    _defaults: cloneTestValue(signals._defaults),
  });

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-layouts"), preset.id);
  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-layouts"), "");
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-layouts"), null);
  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui)), {
    workspace_tab: "basics",
    distribution_tab: "target_fish",
    custom_layout: [[["overview"], ["distribution"]], [["loot"]]],
    custom_sections: ["overview", "distribution", "loot"],
  });
});

test("calculator restore preserves a persisted modified current layout preset when UI matches it", () => {
  const initial = createContext();
  const initialSignals = defaultSignals();
  initial.window.__fishystuffCalculator.restore(initialSignals);
  const preset = initial.window.__fishystuffUserPresets.createPreset("calculator-layouts", {
    name: "Base layout",
    payload: {
      custom_layout: [[["overview"]], [["zone"]]],
    },
    select: true,
  });
  initial.window.__fishystuffUserPresets.trackCurrentPayload("calculator-layouts", {
    payload: {
      custom_layout: [[["overview"]], [["trade"]]],
    },
    origin: { kind: "preset", id: preset.id },
  });
  const presetStorage = initial.localStorage.getItem(initial.window.__fishystuffUserPresets.STORAGE_KEY);

  const env = createContext({
    [initial.window.__fishystuffUserPresets.STORAGE_KEY]: presetStorage,
    "fishystuff.calculator.ui.v1": JSON.stringify(defaultCalculatorUiState({
      custom_layout: [[["overview"]], [["trade"]]],
      custom_sections: ["overview", "trade"],
    })),
  });
  const signals = defaultSignals();
  env.window.__fishystuffCalculator.restore(signals);
  patchCalculatorSignals(env, {
    _loading: false,
    _defaults: cloneTestValue(signals._defaults),
    _calculator_ui: defaultCalculatorUiState(),
  });

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-layouts"), preset.id);
  assert.deepEqual(JSON.parse(JSON.stringify(env.window.__fishystuffUserPresets.current("calculator-layouts")?.payload)), {
    custom_layout: [[["overview"]], [["trade"]]],
  });
  assert.deepEqual(JSON.parse(JSON.stringify(env.window.__fishystuffCalculator.layoutPresetPayload(signals._calculator_ui))), {
    custom_layout: [[["overview"]], [["trade"]]],
  });
});

test("calculator restore loads a persisted modified layout working copy over older UI storage", () => {
  const initial = createContext();
  const initialSignals = defaultSignals();
  initial.window.__fishystuffCalculator.restore(initialSignals);
  const preset = initial.window.__fishystuffUserPresets.createPreset("calculator-layouts", {
    name: "Base layout",
    payload: {
      custom_layout: [[["overview"]], [["zone"]]],
    },
    select: true,
  });
  initial.window.__fishystuffUserPresets.trackCurrentPayload("calculator-layouts", {
    payload: {
      custom_layout: [[["overview"], ["distribution"]], [["loot"]]],
    },
    origin: { kind: "preset", id: preset.id },
  });
  const presetStorage = initial.localStorage.getItem(initial.window.__fishystuffUserPresets.STORAGE_KEY);

  const env = createContext({
    [initial.window.__fishystuffUserPresets.STORAGE_KEY]: presetStorage,
    "fishystuff.calculator.ui.v1": JSON.stringify(defaultCalculatorUiState()),
  });
  const signals = defaultSignals();
  env.window.__fishystuffCalculator.restore(signals);
  patchCalculatorSignals(env, {
    _loading: false,
    _defaults: cloneTestValue(signals._defaults),
    _calculator_ui: defaultCalculatorUiState(),
  });

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-layouts"), preset.id);
  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-layouts"), "");
  assert.deepEqual(JSON.parse(JSON.stringify(env.window.__fishystuffUserPresets.current("calculator-layouts")?.payload)), {
    custom_layout: [[["overview"], ["distribution"]], [["loot"]]],
  });
  assert.deepEqual(JSON.parse(JSON.stringify(env.window.__fishystuffCalculator.layoutPresetPayload(signals._calculator_ui))), {
    custom_layout: [[["overview"], ["distribution"]], [["loot"]]],
  });
});

test("calculator restore loads a persisted modified default layout working copy when UI storage is absent", () => {
  const initial = createContext();
  const initialSignals = defaultSignals();
  initial.window.__fishystuffCalculator.restore(initialSignals);
  initial.window.__fishystuffUserPresets.trackCurrentPayload("calculator-layouts", {
    payload: {
      custom_layout: [[["overview"], ["distribution"]], [["loot"]]],
    },
    origin: { kind: "fixed", id: "default" },
  });
  const presetStorage = initial.localStorage.getItem(initial.window.__fishystuffUserPresets.STORAGE_KEY);

  const env = createContext({
    [initial.window.__fishystuffUserPresets.STORAGE_KEY]: presetStorage,
  });
  const signals = defaultSignals();
  env.window.__fishystuffCalculator.restore(signals);
  patchCalculatorSignals(env, {
    _loading: false,
    _defaults: cloneTestValue(signals._defaults),
    _calculator_ui: defaultCalculatorUiState(),
  });

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-layouts"), "");
  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-layouts"), "default");
  assert.deepEqual(JSON.parse(JSON.stringify(env.window.__fishystuffUserPresets.current("calculator-layouts")?.payload)), {
    custom_layout: [[["overview"], ["distribution"]], [["loot"]]],
  });
  assert.deepEqual(JSON.parse(JSON.stringify(env.window.__fishystuffCalculator.layoutPresetPayload(signals._calculator_ui))), {
    custom_layout: [[["overview"], ["distribution"]], [["loot"]]],
  });
});

test("calculator restore loads a persisted modified default calculator working copy when data storage is absent", () => {
  const initial = createContext();
  const initialSignals = defaultSignals();
  initial.window.__fishystuffCalculator.restore(initialSignals);
  initial.window.__fishystuffUserPresets.trackCurrentPayload("calculator-presets", {
    payload: {
      ...initial.window.__fishystuffCalculator.calculatorPresetPayload(initialSignals),
      level: 42,
      resources: 15,
    },
    origin: { kind: "fixed", id: "default" },
  });
  const presetStorage = initial.localStorage.getItem(initial.window.__fishystuffUserPresets.STORAGE_KEY);

  const env = createContext({
    [initial.window.__fishystuffUserPresets.STORAGE_KEY]: presetStorage,
  });
  const signals = defaultSignals();
  env.window.__fishystuffCalculator.restore(signals);
  patchCalculatorSignals(env, {
    _loading: false,
    _defaults: cloneTestValue(signals._defaults),
  });

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-presets"), "");
  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-presets"), "default");
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets")?.payload?.level, 42);
  assert.equal(signals.level, 42);
  assert.equal(signals.resources, 15);
});

test("calculator restore keeps stored calculator data ahead of stale fixed preset selection", () => {
  const initial = createContext();
  const initialSignals = defaultSignals();
  initial.window.__fishystuffCalculator.restore(initialSignals);
  initial.window.__fishystuffUserPresets.trackCurrentPayload("calculator-presets", {
    payload: {
      ...initial.window.__fishystuffCalculator.calculatorPresetPayload(initialSignals),
      level: 42,
      resources: 15,
    },
    origin: { kind: "fixed", id: "default" },
  });
  const presetStorage = initial.localStorage.getItem(initial.window.__fishystuffUserPresets.STORAGE_KEY);

  const env = createContext({
    [initial.window.__fishystuffUserPresets.STORAGE_KEY]: presetStorage,
    "fishystuff.calculator.data.v1": JSON.stringify({
      level: 42,
      resources: 15,
    }),
  });
  const signals = defaultSignals();
  env.window.__fishystuffCalculator.restore(signals);
  patchCalculatorSignals(env, {
    _loading: false,
    _defaults: cloneTestValue(signals._defaults),
    level: 0,
    resources: 100,
  });

  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-presets"), "default");
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets")?.payload?.level, 42);
  assert.equal(signals.level, 42);
  assert.equal(signals.resources, 15);
});

test("calculator restore reapplies persisted calculator UI after init defaults", () => {
  const env = createContext({
    "fishystuff.calculator.ui.v1": JSON.stringify({
      workspace_tab: "basics",
      distribution_tab: "target_fish",
      custom_layout: [[["overview"], ["distribution"]], [["loot"]]],
      custom_sections: ["overview", "distribution", "loot"],
    }),
  });
  const signals = defaultSignals();
  env.window.__fishystuffCalculator.restore(signals);
  patchCalculatorSignals(env, {
    _loading: false,
    _defaults: cloneTestValue(signals._defaults),
    _calculator_ui: defaultCalculatorUiState(),
  });

  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-layouts"), "default");
  assert.deepEqual(
    JSON.parse(JSON.stringify(env.window.__fishystuffUserPresets.current("calculator-layouts")?.payload)),
    {
      custom_layout: [[["overview"], ["distribution"]], [["loot"]]],
    },
  );
  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui)), {
    workspace_tab: "basics",
    distribution_tab: "target_fish",
    custom_layout: [[["overview"], ["distribution"]], [["loot"]]],
    custom_sections: ["overview", "distribution", "loot"],
  });
});

test("calculator layout preset title icon follows the first custom section", () => {
  const env = createContext();
  const calculator = env.window.__fishystuffCalculator;

  assert.equal(
    calculator.layoutPresetTitleIconAlias({
      custom_layout: [[["zone"], ["loot"]], [["buffs"]]],
    }),
    "fullscreen-fill",
  );
  assert.equal(
    calculator.layoutPresetTitleIconAlias({
      custom_layout: [[["gear"]], [["debug"]]],
    }),
    "gear-fill",
  );
  assert.equal(
    calculator.layoutPresetTitleIconAlias({
      custom_layout: [],
    }),
    "",
  );
});

test("calculator preset adapters expose workspace labels and preset manager icon", () => {
  const env = createContext();
  env.window.__fishystuffCalculator.restore(defaultSignals());

  assert.equal(
    env.window.__fishystuffUserPresets.collectionAdapter("calculator-presets").managerIconAlias,
    "settings-6-fill",
  );
  assert.equal(
    env.window.__fishystuffUserPresets.collectionAdapter("calculator-layouts").openLabelFallback,
    "Workspace Presets",
  );
});

test("calculator API URLs keep locale and apiLang separate", () => {
  const korean = createContext({}, { locale: "ko-KR", lang: "en-US" });
  assert.equal(korean.window.__fishystuffCalculator.lang, "en");
  assert.equal(korean.window.__fishystuffCalculator.locale, "ko-KR");
  assert.equal(korean.window.__fishystuffCalculator.apiLang, "en");
  assert.match(korean.window.__fishystuffCalculator.initUrl(), /\?lang=en&locale=ko-KR$/);
  assert.match(korean.window.__fishystuffCalculator.evalUrl(), /\?lang=en&locale=ko-KR$/);

  const german = createContext({}, { locale: "de-DE", lang: "en-US" });
  assert.equal(german.window.__fishystuffCalculator.lang, "en");
  assert.equal(german.window.__fishystuffCalculator.locale, "de-DE");
  assert.equal(german.window.__fishystuffCalculator.apiLang, "en");
  assert.match(german.window.__fishystuffCalculator.initUrl(), /\?lang=en&locale=de-DE$/);
  assert.match(german.window.__fishystuffCalculator.evalUrl(), /\?lang=en&locale=de-DE$/);
});

test("calculator persist stores canonical page state and excludes transient branches", () => {
  const env = createContext();
  const signals = defaultSignals();
  env.window.__fishystuffCalculator.restore(signals);
  Object.assign(signals, {
    food: ["item:9359", "", "item:9359"],
    _live: { total_time: "123.45" },
    _calc: { zone_name: "Velia Beach" },
  });
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      food: ["item:9359", "", "item:9359"],
    },
  });
  env.flushTimers();

  const persistedData = JSON.parse(env.localStorage.getItem("fishystuff.calculator.data.v1"));
  const persistedUi = JSON.parse(env.localStorage.getItem("fishystuff.calculator.ui.v1"));
  assert.deepEqual(persistedData.food, ["item:9359"]);
  assert.deepEqual(persistedUi, defaultCalculatorUiState());
  assert.equal("_live" in persistedData, false);
  assert.equal("_calc" in persistedData, false);
  assert.equal("_defaults" in persistedData, false);
  assert.equal("overlay" in persistedData, false);
  assert.equal("_calculator_ui" in persistedData, false);
});

test("calculator restore prefers shared overlay storage for prices and zone overlays", () => {
  const env = createContext({
    "fishystuff.calculator.data.v1": JSON.stringify({
      priceOverrides: {
        "item:8473": {
          basePrice: 8800000,
        },
      },
    }),
    "fishystuff.user-overlays.v2": JSON.stringify({
      overlay: {
        zones: {
          "240,74,74": {
            groups: {
              4: {
                rawRatePercent: 82,
              },
            },
            items: {},
          },
        },
      },
      priceOverrides: {
        "9359": {
          basePrice: 12345,
        },
      },
    }),
  });
  const signals = defaultSignals();

  env.window.__fishystuffCalculator.restore(signals);

  assert.deepEqual(JSON.parse(JSON.stringify(signals.overlay)), {
    zones: {
      "240,74,74": {
        groups: {
          4: {
            rawRatePercent: 82,
          },
        },
        items: {},
      },
    },
  });
  assert.deepEqual(JSON.parse(JSON.stringify(signals.priceOverrides)), {
    "9359": {
      basePrice: 12345,
    },
    "8473": {
      basePrice: 8800000,
    },
  });
});

test("calculator action listener handles copy and discard tokens once", () => {
  const env = createContext();
  const signals = defaultSignals();
  env.window.__fishystuffCalculator.restore(signals);
  signals.active = true;
  signals.food = ["item:9359"];
  env.window.__fishystuffUserPresets.trackCurrentPayload("calculator-presets", {
    payload: env.window.__fishystuffCalculator.calculatorPresetPayload(signals),
    origin: { kind: "fixed", id: "default" },
  });
  const dirtyWorkingCopy = env.window.__fishystuffUserPresets.activeWorkingCopy("calculator-presets");
  assert.equal(signals._user_presets.collections["calculator-presets"].hasCurrent, true);
  signals._calculator_actions = defaultCalculatorActionState({
    copyUrlToken: 1,
    copyShareToken: 1,
    discardCalculatorToken: 1,
  });

  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _calculator_actions: {
        copyUrlToken: 1,
        copyShareToken: 1,
        saveCalculatorToken: 0,
        discardCalculatorToken: 1,
        saveLayoutToken: 0,
        discardLayoutToken: 0,
      },
    },
  });
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _calculator_actions: {
        copyUrlToken: 0,
        copyShareToken: 0,
        saveCalculatorToken: 0,
        discardCalculatorToken: 0,
        saveLayoutToken: 0,
        discardLayoutToken: 0,
      },
    },
  });

  assert.equal(env.toastCalls.length, 3);
  assert.equal(env.toastCalls[0].type, "copyText");
  assert.match(env.toastCalls[0].text, /\?preset=lz:/);
  assert.equal(env.toastCalls[0].options.success, calculatorMessage("toast.preset_url_copied"));
  assert.equal(env.toastCalls[1].type, "copyText");
  assert.match(env.toastCalls[1].text, /FishyStuff Calculator Preset/);
  assert.equal(env.toastCalls[1].options.success, calculatorMessage("toast.share_copied"));
  assert.equal(env.toastCalls[2].type, "info");
  assert.equal(env.toastCalls[2].message, translateMessage("presets.toast.discarded"));
  assert.deepEqual(Array.from(signals.food), []);
  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-presets"), "default");
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets"), null);
  assert.deepEqual(env.window.__fishystuffUserPresets.workingCopies("calculator-presets"), []);
  assert.notEqual(
    env.window.__fishystuffUserPresets.activeWorkingCopy("calculator-presets")?.id,
    dirtyWorkingCopy.id,
  );
  assert.equal(signals._user_presets.collections["calculator-presets"].hasCurrent, false);
});

test("calculator action listener discards calculator preset current when defaults are fully restored", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffCalculator.restore(signals);
  signals.level = 20;
  signals.food = ["item:9359"];
  env.window.__fishystuffUserPresets.trackCurrentPayload("calculator-presets", {
    payload: env.window.__fishystuffCalculator.calculatorPresetPayload(signals),
    origin: { kind: "fixed", id: "default" },
  });
  assert.notEqual(env.window.__fishystuffUserPresets.current("calculator-presets"), null);
  signals._calculator_actions = defaultCalculatorActionState({
    discardCalculatorToken: 1,
  });

  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _calculator_actions: {
        copyUrlToken: 0,
        copyShareToken: 0,
        saveCalculatorToken: 0,
        discardCalculatorToken: 1,
        saveLayoutToken: 0,
        discardLayoutToken: 0,
      },
    },
  });

  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-presets"), "default");
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets"), null);
  assert.equal(signals.level, 0);
  assert.deepEqual(Array.from(signals.food), []);
});

test("calculator action listener saves modified default calculator preset as a new preset", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffCalculator.restore(signals);
  signals.level = 42;
  signals.food = ["item:9359"];
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets"), null);
  signals._calculator_actions = defaultCalculatorActionState({
    saveCalculatorToken: 1,
  });

  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _calculator_actions: {
        copyUrlToken: 0,
        copyShareToken: 0,
        saveCalculatorToken: 1,
        discardCalculatorToken: 0,
        saveLayoutToken: 0,
        discardLayoutToken: 0,
      },
    },
  });

  const presets = env.window.__fishystuffUserPresets.presets("calculator-presets");
  assert.equal(presets.length, 1);
  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-presets"), presets[0].id);
  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-presets"), "");
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets"), null);
  assert.equal(presets[0].payload.level, 42);
  assert.deepEqual(Array.from(presets[0].payload.food), ["item:9359"]);
  assert.equal(env.toastCalls.length, 1);
  assert.equal(env.toastCalls[0].type, "info");
  assert.equal(env.toastCalls[0].message, "presets.toast.created");
});

test("calculator can return from a saved preset to the fixed default preset", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffCalculator.restore(signals);
  signals.level = 42;
  signals.food = ["item:9359"];
  env.window.__fishystuffUserPresets.trackCurrentPayload("calculator-presets", {
    payload: env.window.__fishystuffCalculator.calculatorPresetPayload(signals),
    origin: { kind: "fixed", id: "default" },
  });
  const saved = env.window.__fishystuffUserPresets.saveCurrent("calculator-presets");

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-presets"), saved.preset.id);
  assert.equal(signals.level, 42);

  env.window.__fishystuffUserPresets.activateFixedPreset("calculator-presets", "default");

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-presets"), "");
  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-presets"), "default");
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets"), null);
  assert.equal(signals.level, 0);
  assert.deepEqual(Array.from(signals.food), []);
});

test("calculator fixed preset apply replaces object-map fields", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffCalculator.restore(signals);
  signals.level = 42;
  signals.food = ["item:9359"];
  signals.priceOverrides = {
    9359: {
      basePrice: 12345,
    },
  };
  env.window.__fishystuffUserPresets.trackCurrentPayload("calculator-presets", {
    payload: env.window.__fishystuffCalculator.calculatorPresetPayload(signals),
    origin: { kind: "fixed", id: "default" },
  });
  const saved = env.window.__fishystuffUserPresets.saveCurrent("calculator-presets");
  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-presets"), saved.preset.id);

  env.window.__fishystuffUserPresets.activateFixedPreset("calculator-presets", "default");

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-presets"), "");
  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-presets"), "default");
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets"), null);
  assert.equal(signals.level, 0);
  assert.deepEqual(Array.from(signals.food), []);
  assert.deepEqual(JSON.parse(JSON.stringify(signals.priceOverrides)), {});
});

test("calculator action listener can discard default modifications with object-map fields", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffCalculator.restore(signals);
  signals.priceOverrides = {
    9359: {
      basePrice: 12345,
    },
  };
  env.window.__fishystuffUserPresets.trackCurrentPayload("calculator-presets", {
    payload: env.window.__fishystuffCalculator.calculatorPresetPayload(signals),
    origin: { kind: "fixed", id: "default" },
  });
  assert.notEqual(env.window.__fishystuffUserPresets.current("calculator-presets"), null);
  signals._calculator_actions = defaultCalculatorActionState({
    discardCalculatorToken: 1,
  });

  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _calculator_actions: {
        copyUrlToken: 0,
        copyShareToken: 0,
        saveCalculatorToken: 0,
        discardCalculatorToken: 1,
        saveLayoutToken: 0,
        discardLayoutToken: 0,
      },
    },
  });

  assert.equal(env.toastCalls.length, 1);
  assert.equal(env.toastCalls[0].type, "info");
  assert.equal(env.toastCalls[0].message, translateMessage("presets.toast.discarded"));
  assert.deepEqual(JSON.parse(JSON.stringify(signals.priceOverrides)), {});
  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-presets"), "default");
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets"), null);
});

test("calculator action listener saves modified layout preset back to its saved preset", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffCalculator.restore(signals);
  const preset = env.window.__fishystuffUserPresets.createPreset("calculator-layouts", {
    name: "Workspace 1",
    payload: env.window.__fishystuffCalculator.layoutPresetPayload(signals._calculator_ui),
    select: true,
  });
  signals._calculator_ui = {
    workspace_tab: "basics",
    distribution_tab: "target_fish",
    custom_layout: [[["overview"], ["distribution"]]],
    custom_sections: ["overview", "distribution"],
  };
  env.window.__fishystuffUserPresets.trackCurrentPayload("calculator-layouts", {
    payload: env.window.__fishystuffCalculator.layoutPresetPayload(signals._calculator_ui),
    origin: { kind: "preset", id: preset.id },
  });
  signals._calculator_actions = defaultCalculatorActionState({
    saveLayoutToken: 1,
  });

  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _calculator_actions: {
        copyUrlToken: 0,
        copyShareToken: 0,
        saveCalculatorToken: 0,
        discardCalculatorToken: 0,
        saveLayoutToken: 1,
        discardLayoutToken: 0,
      },
    },
  });

  assert.equal(env.window.__fishystuffUserPresets.presets("calculator-layouts").length, 1);
  assert.deepEqual(env.window.__fishystuffUserPresets.preset("calculator-layouts", preset.id).payload, {
    custom_layout: [[["overview"], ["distribution"]]],
  });
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-layouts"), null);
  assert.equal(env.toastCalls.length, 1);
  assert.equal(env.toastCalls[0].type, "info");
  assert.equal(env.toastCalls[0].message, "presets.toast.saved");
});

test("calculator action listener discards only layout preset modifications", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffCalculator.restore(signals);
  signals._calculator_ui = {
    workspace_tab: "basics",
    distribution_tab: "target_fish",
    custom_layout: [[["overview"], ["distribution"]]],
    custom_sections: ["overview", "distribution"],
  };
  env.window.__fishystuffUserPresets.trackCurrentPayload("calculator-layouts", {
    payload: env.window.__fishystuffCalculator.layoutPresetPayload(signals._calculator_ui),
    origin: { kind: "fixed", id: "default" },
  });
  assert.notEqual(env.window.__fishystuffUserPresets.current("calculator-layouts"), null);
  signals._calculator_actions = defaultCalculatorActionState({
    discardLayoutToken: 1,
  });

  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _calculator_actions: {
        copyUrlToken: 0,
        copyShareToken: 0,
        saveCalculatorToken: 0,
        discardCalculatorToken: 0,
        saveLayoutToken: 0,
        discardLayoutToken: 1,
      },
    },
  });
  env.flushTimers();

  assert.equal(env.toastCalls.length, 1);
  assert.equal(env.toastCalls[0].type, "info");
  assert.equal(env.toastCalls[0].message, translateMessage("presets.toast.discarded"));
  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui)), {
    workspace_tab: "basics",
    distribution_tab: "target_fish",
    custom_layout: cloneTestValue(DEFAULT_CUSTOM_LAYOUT),
    custom_sections: Array.from(DEFAULT_CUSTOM_SECTIONS),
  });
  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-layouts"), "default");
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-layouts"), null);
  assert.deepEqual(
    JSON.parse(env.localStorage.getItem("fishystuff.calculator.ui.v1")),
    {
      workspace_tab: "basics",
      distribution_tab: "target_fish",
      custom_layout: cloneTestValue(DEFAULT_CUSTOM_LAYOUT),
      custom_sections: Array.from(DEFAULT_CUSTOM_SECTIONS),
    },
  );
});

test("calculator liveCalc keeps stat breakdown payloads aligned with local derived values", () => {
  const env = createContext();
  const live = env.window.__fishystuffCalculator.liveCalc(
    2,
    50,
    false,
    2,
    3,
    2,
    "hours",
    {
      zone_bite_min: "10",
      zone_bite_max: "20",
      zone_name: "Velia Beach",
      auto_fish_time: "90",
      auto_fish_time_reduction_text: "50%",
      chance_to_consume_durability_text: "25.00%",
      fish_multiplier_raw: 1.5,
      loot_profit_per_catch_raw: 1000,
      loot_total_profit: "1,000",
      trade_sale_multiplier_text: "120.00%",
      stat_breakdowns: {
        total_time: JSON.stringify({
          title: breakdownTitle("total_time"),
          value_text: "0.00",
          sections: [{ label: breakdownSection("inputs"), rows: [] }, { label: breakdownSection("composition"), rows: [] }],
        }),
        bite_time: JSON.stringify({
          title: breakdownTitle("bite_time"),
          value_text: "0.00",
          sections: [{ label: breakdownSection("inputs"), rows: [] }, { label: breakdownSection("composition"), rows: [] }],
        }),
        auto_fish_time: JSON.stringify({
          title: breakdownTitle("auto_fish_time"),
          value_text: "0.00",
          sections: [{ label: breakdownSection("inputs"), rows: [] }, { label: breakdownSection("composition"), rows: [] }],
        }),
        catch_time: JSON.stringify({
          title: breakdownTitle("catch_time"),
          value_text: "0.00",
          sections: [{ label: breakdownSection("inputs"), rows: [] }, { label: breakdownSection("composition"), rows: [] }],
        }),
        time_saved: JSON.stringify({
          title: breakdownTitle("time_saved"),
          value_text: "0.00%",
          sections: [{ label: breakdownSection("inputs"), rows: [] }, { label: breakdownSection("composition"), rows: [] }],
        }),
        casts_average: JSON.stringify({
          title: breakdownTitle("casts_average", { timespan: "8 hours" }),
          value_text: "0.00",
          sections: [{ label: breakdownSection("inputs"), rows: [] }, { label: breakdownSection("composition"), rows: [] }],
        }),
        effective_bite_avg: JSON.stringify({
          title: breakdownTitle("effective_bite_avg"),
          value_text: "0.00",
          sections: [{ label: breakdownSection("inputs"), rows: [] }, { label: breakdownSection("composition"), rows: [] }],
        }),
        loot_total_catches: JSON.stringify({
          title: breakdownTitle("loot_total_catches", { timespan: "8 hours" }),
          value_text: "0.00",
          sections: [
            { label: breakdownSection("inputs"), rows: [] },
            { label: breakdownSection("composition"), rows: [] },
          ],
        }),
        loot_profit_per_hour: JSON.stringify({
          title: breakdownTitle("loot_profit_per_hour"),
          value_text: "0",
          sections: [{ label: breakdownSection("inputs"), rows: [] }, { label: breakdownSection("composition"), rows: [] }],
        }),
      },
    },
  );

  const totalTime = JSON.parse(live.stat_breakdowns.total_time);
  const castsAverage = JSON.parse(live.stat_breakdowns.casts_average);
  const catchTime = JSON.parse(live.stat_breakdowns.catch_time);
  const timeSaved = JSON.parse(live.stat_breakdowns.time_saved);
  const effectiveBiteAverage = JSON.parse(live.stat_breakdowns.effective_bite_avg);
  const totalCatches = JSON.parse(live.stat_breakdowns.loot_total_catches);
  const profitPerHour = JSON.parse(live.stat_breakdowns.loot_profit_per_hour);

  assert.equal(totalTime.value_text, live.total_time);
  assert.equal(totalTime.sections[0].rows[0].value_text, live.bite_time);
  assert.equal(totalTime.sections[0].rows[1].label, breakdownLabel("auto_fishing_time"));
  assert.equal(totalTime.sections[1].rows[0].label, breakdownLabel("average_total"));
  assert.equal(castsAverage.title, breakdownTitle("casts_average", { timespan: "2 hours" }));
  assert.equal(castsAverage.sections[0].rows[0].value_text, "2 hours");
  assert.equal(castsAverage.formula_terms[1].label, breakdownLabel("session_seconds"));
  assert.equal(castsAverage.formula_terms[1].value_text, "7200");
  assert.equal(catchTime.sections[0].rows[0].label, breakdownLabel("afk_catch_time"));
  assert.equal(catchTime.sections[1].rows[0].value_text, "3.00");
  assert.equal(timeSaved.sections[1].rows[1].label, breakdownLabel("saved_share"));
  assert.equal(timeSaved.value_text, "45.64%");
  assert.equal(effectiveBiteAverage.formula_terms[1].label, breakdownLabel("zone_bite_average"));
  assert.equal(effectiveBiteAverage.formula_terms[1].value_text, "15.00");
  assert.equal(totalCatches.title, breakdownTitle("loot_total_catches", { timespan: "2 hours" }));
  assert.equal(totalCatches.value_text, live.loot_total_catches);
  assert.equal(totalCatches.sections[0].rows[0].label, breakdownLabel("average_casts"));
  assert.equal(totalCatches.sections[0].rows[0].value_text, live.casts_average);
  assert.equal(totalCatches.sections[1].rows[0].label, breakdownLabel("expected_catches"));
  assert.equal(profitPerHour.sections[0].rows[0].value_text, live.loot_total_profit);
  assert.equal(profitPerHour.value_text, live.loot_profit_per_hour);
  assert.equal(live.fishing_timeline_chart.segments.length, 4);
  assert.equal(live.fishing_timeline_chart.segments[0].label, timelineLabel("bite_time"));
  assert.equal(live.fishing_timeline_chart.segments[2].label, timelineLabel("catch_time"));
  assert.equal(live.fishing_timeline_chart.segments[3].label, timelineLabel("time_saved"));
  assert.equal(live.fishing_timeline_chart.segments[0].breakdown.title, breakdownTitle("bite_time"));
  assert.equal(live.fishing_timeline_chart.segments[3].breakdown.title, breakdownTitle("time_saved"));
});
