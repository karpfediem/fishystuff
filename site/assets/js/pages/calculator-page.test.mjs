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

const DEFAULT_PINNED_LAYOUT = Object.freeze([
  Object.freeze([Object.freeze(["overview"])]),
  Object.freeze([Object.freeze(["zone"]), Object.freeze(["session"])]),
  Object.freeze([Object.freeze(["bite_time"]), Object.freeze(["loot"])]),
]);
const DEFAULT_PINNED_SECTIONS = Object.freeze([
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
    top_level_tab: "mode",
    distribution_tab: "groups",
    pinned_layout: cloneTestValue(DEFAULT_PINNED_LAYOUT),
    pinned_sections: Array.from(DEFAULT_PINNED_SECTIONS),
    unpinned_insert_index: [0, 0],
    ...cloneTestValue(overrides),
  };
}

function defaultCalculatorActionState(overrides = {}) {
  return {
    copyUrlToken: 0,
    copyShareToken: 0,
    clearToken: 0,
    resetLayoutToken: 0,
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
          apiLang: options.apiLang || (String(locale).toLowerCase().startsWith("ko") ? "ko" : "en"),
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
      top_level_tab: "distribution",
      distribution_tab: "loot_flow",
      pinned_layout: [[["zone"], ["distribution"]], [["missing"]]],
      pinned_sections: ["zone", "distribution", "zone", "missing"],
      unpinned_insert_index: [2, 0],
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
  assert.equal(signals._calculator_ui.top_level_tab, "distribution");
  assert.equal(signals._calculator_ui.distribution_tab, "loot_flow");
  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui.pinned_layout)), [[["zone"], ["distribution"]]]);
  assert.deepEqual(Array.from(signals._calculator_ui.pinned_sections), ["zone", "distribution"]);
  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui.unpinned_insert_index)), [2, 0]);
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

test("calculator restore keeps the current tab while restoring trade, food, and buffs UI state", () => {
  const env = createContext({
    "fishystuff.calculator.ui.v1": JSON.stringify({
      top_level_tab: "food",
      distribution_tab: "groups",
      pinned_layout: [[["overview"], ["trade"]], [["food", "buffs"], ["missing"]]],
      pinned_sections: ["overview", "trade", "food", "buffs", "missing"],
      unpinned_insert_index: [1, 0],
    }),
  });
  const signals = defaultSignals();
  signals._calculator_ui.top_level_tab = "trade";

  env.window.__fishystuffCalculator.restore(signals);

  assert.equal(signals._calculator_ui.top_level_tab, "trade");
  assert.equal(signals._calculator_ui.distribution_tab, "groups");
  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui.pinned_layout)), [
    [["overview"], ["trade"]],
    [["food", "buffs"]],
  ]);
  assert.deepEqual(Array.from(signals._calculator_ui.pinned_sections), [
    "overview",
    "trade",
    "food",
    "buffs",
  ]);
  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui.unpinned_insert_index)), [1, 0]);
});

test("calculator restore ignores legacy pinned UI state without a pinned layout", () => {
  const env = createContext({
    "fishystuff.calculator.ui.v1": JSON.stringify({
      top_level_tab: "food",
      distribution_tab: "groups",
      pinned_sections: ["trade", "food", "buffs"],
    }),
  });
  const signals = defaultSignals();

  env.window.__fishystuffCalculator.restore(signals);

  assert.equal(signals._calculator_ui.top_level_tab, "food");
  assert.equal(signals._calculator_ui.distribution_tab, "groups");
  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui.pinned_layout)), cloneTestValue(DEFAULT_PINNED_LAYOUT));
  assert.deepEqual(Array.from(signals._calculator_ui.pinned_sections), Array.from(DEFAULT_PINNED_SECTIONS));
  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui.unpinned_insert_index)), [0, 0]);
});

test("calculator restore leaves initial shell state intact when storage is empty", () => {
  const env = createContext();
  const signals = {
    _loading: true,
    _calculator_ui: defaultCalculatorUiState(),
    _calculator_actions: defaultCalculatorActionState(),
  };

  env.window.__fishystuffCalculator.restore(signals);

  assert.deepEqual(JSON.parse(JSON.stringify(signals)), {
    _loading: true,
    overlay: {
      zones: {},
    },
    priceOverrides: {},
    _calculator_ui: defaultCalculatorUiState(),
    _calculator_actions: defaultCalculatorActionState(),
  });
});

test("calculator pin helpers keep pinned sections ordered and placeable", () => {
  const env = createContext();
  const calculator = env.window.__fishystuffCalculator;

  assert.deepEqual(
    Array.from(calculator.togglePinnedSection(undefined, "distribution")),
    ["overview", "zone", "session", "bite_time", "loot", "distribution"],
  );
  assert.deepEqual(
    Array.from(calculator.togglePinnedSection(["overview", "zone"], "overview")),
    ["zone"],
  );
  assert.deepEqual(
    Array.from(calculator.movePinnedSection(["overview", "zone", "loot"], "loot", -1)),
    ["overview", "loot", "zone"],
  );
  assert.deepEqual(
    Array.from(calculator.pinSection(["overview"], "overview")),
    ["overview"],
  );
  assert.deepEqual(
    Array.from(calculator.placePinnedSection(["overview"], "loot", "overview", "before")),
    ["loot", "overview"],
  );
  assert.deepEqual(
    Array.from(calculator.placePinnedSection(["overview", "zone"], "overview", "zone", "after")),
    ["zone", "overview"],
  );
  assert.deepEqual(
    JSON.parse(JSON.stringify(calculator.togglePinnedSection({
      top_level_tab: "overview",
      distribution_tab: "groups",
      pinned_layout: [[["overview"]], [["zone"]]],
      pinned_sections: ["overview", "zone"],
      unpinned_insert_index: [0, 0],
    }, "distribution"))),
    {
      top_level_tab: "overview",
      distribution_tab: "groups",
      pinned_layout: [[["overview"]], [["zone"]], [["distribution"]]],
      pinned_sections: ["overview", "zone", "distribution"],
      unpinned_insert_index: [0, 0],
    },
  );
  assert.deepEqual(
    JSON.parse(JSON.stringify(calculator.togglePinnedSection({
      top_level_tab: "trade",
      distribution_tab: "groups",
      pinned_layout: [[["overview"]], [["loot"]]],
      pinned_sections: ["overview", "loot"],
      unpinned_insert_index: [1, 0],
    }, "trade"))),
    {
      top_level_tab: "trade",
      distribution_tab: "groups",
      pinned_layout: [[["overview"]], [["trade"]], [["loot"]]],
      pinned_sections: ["overview", "trade", "loot"],
      unpinned_insert_index: [1, 0],
    },
  );
  assert.deepEqual(
    JSON.parse(JSON.stringify(calculator.togglePinnedSection({
      top_level_tab: "overview",
      distribution_tab: "groups",
      pinned_layout: [[["overview"]]],
      pinned_sections: ["overview"],
      unpinned_insert_index: [0, 0],
    }, "overview"))),
    {
      top_level_tab: "overview",
      distribution_tab: "groups",
      pinned_layout: [],
      pinned_sections: [],
      unpinned_insert_index: [0, 0],
    },
  );
  const uiState = {
    top_level_tab: "loot",
    distribution_tab: "loot_flow",
    pinned_layout: [[["overview"]]],
    pinned_sections: ["overview"],
    unpinned_insert_index: [3, 0],
  };
  assert.equal(calculator.togglePinnedSectionInPlace(uiState, "zone"), uiState);
  assert.deepEqual(JSON.parse(JSON.stringify(uiState)), {
    top_level_tab: "loot",
    distribution_tab: "loot_flow",
    pinned_layout: [[["overview"]], [["zone"]]],
    pinned_sections: ["overview", "zone"],
    unpinned_insert_index: [3, 0],
  });
  const selectedUnpinnedState = {
    top_level_tab: "trade",
    distribution_tab: "groups",
    pinned_layout: [[["overview"]], [["loot"]]],
    pinned_sections: ["overview", "loot"],
    unpinned_insert_index: [1, 0],
  };
  assert.equal(calculator.togglePinnedSectionInPlace(selectedUnpinnedState, "trade"), selectedUnpinnedState);
  assert.deepEqual(JSON.parse(JSON.stringify(selectedUnpinnedState)), {
    top_level_tab: "trade",
    distribution_tab: "groups",
    pinned_layout: [[["overview"]], [["trade"]], [["loot"]]],
    pinned_sections: ["overview", "trade", "loot"],
    unpinned_insert_index: [1, 0],
  });
  const selectedPinnedState = {
    top_level_tab: "trade",
    distribution_tab: "groups",
    pinned_layout: [[["overview"]], [["trade"]]],
    pinned_sections: ["overview", "trade"],
    unpinned_insert_index: [1, 0],
  };
  assert.equal(calculator.togglePinnedSectionInPlace(selectedPinnedState, "trade"), selectedPinnedState);
  assert.deepEqual(JSON.parse(JSON.stringify(selectedPinnedState)), {
    top_level_tab: "trade",
    distribution_tab: "groups",
    pinned_layout: [[["overview"]]],
    pinned_sections: ["overview"],
    unpinned_insert_index: [1, 0],
  });
  assert.equal(
    calculator.sectionVisible("trade", selectedPinnedState.top_level_tab, selectedPinnedState.pinned_sections),
    true,
  );
  assert.equal(calculator.canMovePinnedSection(["overview", "zone"], "overview", -1), false);
  assert.equal(calculator.canMovePinnedSection(["overview", "zone"], "overview", 1), true);
  assert.equal(calculator.isPinnedSection(["overview", "zone"], "zone"), true);
  assert.equal(calculator.isPinnedSection({
    pinned_layout: [[["overview", "zone"]]],
    pinned_sections: ["overview", "zone"],
  }, "zone"), true);
  assert.deepEqual(
    JSON.parse(JSON.stringify(calculator.normalizeUnpinnedInsertIndex(["-4", "3"]))),
    [0, 3],
  );
  assert.equal(calculator.sectionVisible("overview", "loot", []), false);
  assert.equal(calculator.sectionVisible("overview", "loot", ["overview"]), true);
  assert.equal(calculator.sectionOrder("loot", "loot", ["overview", "zone"]), 2);
});

test("calculator reset layout restores the default pinned mosaic while keeping the selected tab", () => {
  const env = createContext();
  const calculator = env.window.__fishystuffCalculator;
  const uiState = {
    top_level_tab: "trade",
    distribution_tab: "loot_flow",
    pinned_layout: [[["overview"], ["distribution"]], [["food", "buffs"]]],
    pinned_sections: ["overview", "distribution", "food", "buffs"],
    unpinned_insert_index: [4, 2],
  };

  assert.deepEqual(JSON.parse(JSON.stringify(calculator.resetCalculatorLayout(uiState))), {
    top_level_tab: "trade",
    distribution_tab: "loot_flow",
    pinned_layout: cloneTestValue(DEFAULT_PINNED_LAYOUT),
    pinned_sections: Array.from(DEFAULT_PINNED_SECTIONS),
    unpinned_insert_index: [0, 0],
  });

  assert.equal(calculator.resetCalculatorLayoutInPlace(uiState), uiState);
  assert.deepEqual(JSON.parse(JSON.stringify(uiState)), {
    top_level_tab: "trade",
    distribution_tab: "loot_flow",
    pinned_layout: cloneTestValue(DEFAULT_PINNED_LAYOUT),
    pinned_sections: Array.from(DEFAULT_PINNED_SECTIONS),
    unpinned_insert_index: [0, 0],
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
  signals._calculator_ui.top_level_tab = "trade";
  signals._calculator_ui.distribution_tab = "target_fish";

  env.window.__fishystuffCalculator.restore(signals);

  const preset = env.window.__fishystuffUserPresets.createPreset("calculator-layouts", {
    name: "Split loot",
    payload: {
      pinned_layout: [[["overview"], ["distribution"]], [["loot"]]],
      unpinned_insert_index: [2, 1],
    },
    select: false,
  });

  env.window.__fishystuffUserPresets.activatePreset("calculator-layouts", preset.id);
  env.flushTimers();

  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui)), {
    top_level_tab: "trade",
    distribution_tab: "target_fish",
    pinned_layout: [[["overview"], ["distribution"]], [["loot"]]],
    pinned_sections: ["overview", "distribution", "loot"],
    unpinned_insert_index: [2, 1],
  });
  assert.deepEqual(
    JSON.parse(env.localStorage.getItem("fishystuff.calculator.ui.v1")),
    {
      top_level_tab: "trade",
      distribution_tab: "target_fish",
      pinned_layout: [[["overview"], ["distribution"]], [["loot"]]],
      pinned_sections: ["overview", "distribution", "loot"],
      unpinned_insert_index: [2, 1],
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
      pinned_layout: [[["mode"]], [["overview"]]],
      unpinned_insert_index: [0, 0],
    },
    select: true,
  });
  const beta = env.window.__fishystuffUserPresets.createPreset("calculator-layouts", {
    name: "Beta",
    payload: {
      pinned_layout: [[["zone"]], [["loot"]]],
      unpinned_insert_index: [1, 0],
    },
    select: false,
  });

  env.window.__fishystuffUserPresets.activatePreset("calculator-layouts", beta.id);
  env.flushTimers();

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-layouts"), beta.id);
  assert.deepEqual(env.window.__fishystuffUserPresets.preset("calculator-layouts", alpha.id)?.payload, {
    pinned_layout: [[["mode"]], [["overview"]]],
    unpinned_insert_index: [0, 0],
  });
  assert.deepEqual(env.window.__fishystuffUserPresets.preset("calculator-layouts", beta.id)?.payload, {
    pinned_layout: [[["zone"]], [["loot"]]],
    unpinned_insert_index: [1, 0],
  });
});

test("calculator layout changes create a current modified preset from default without saving", () => {
  const env = createContext();
  const signals = defaultSignals();
  env.window.__fishystuffCalculator.restore(signals);

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-layouts"), "");
  assert.equal(env.window.__fishystuffUserPresets.presets("calculator-layouts").length, 0);

  env.window.__fishystuffCalculator.togglePinnedSectionInPlace(signals._calculator_ui, "trade");
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
    pinned_layout: [[["overview"]], [["zone"], ["session"]], [["bite_time"], ["loot"]], [["trade"]]],
    unpinned_insert_index: [0, 0],
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

  env.window.__fishystuffCalculator.togglePinnedSectionInPlace(signals._calculator_ui, "trade");
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
    pinned_layout: cloneTestValue(DEFAULT_PINNED_LAYOUT),
    unpinned_insert_index: [0, 0],
  });
  assert.deepEqual(env.window.__fishystuffUserPresets.current("calculator-layouts")?.payload, {
    pinned_layout: [[["overview"]], [["zone"], ["session"]], [["bite_time"], ["loot"]], [["trade"]]],
    unpinned_insert_index: [0, 0],
  });

  const saved = env.window.__fishystuffUserPresets.saveCurrentToSelectedPreset("calculator-layouts");
  assert.equal(saved.id, preset.id);
  assert.deepEqual(saved.payload, {
    pinned_layout: [[["overview"]], [["zone"], ["session"]], [["bite_time"], ["loot"]], [["trade"]]],
    unpinned_insert_index: [0, 0],
  });
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-layouts"), null);
});

test("calculator presets apply durable inputs without changing the layout preset state", () => {
  const env = createContext();
  const signals = defaultSignals();
  signals._calculator_ui = defaultCalculatorUiState({
    top_level_tab: "trade",
    pinned_layout: [[["overview"]], [["trade"]]],
    pinned_sections: ["overview", "trade"],
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
      food: ["item:9359", "", "item:9359"],
      buff: ["item:1"],
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
  assert.deepEqual(Array.from(signals.food), ["item:9359"]);
  assert.deepEqual(Array.from(signals.buff), ["item:1"]);
  assert.deepEqual(JSON.parse(JSON.stringify(signals.priceOverrides)), {
    "8473": {
      basePrice: 8800000,
    },
  });
  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui)), {
    top_level_tab: "trade",
    distribution_tab: "groups",
    pinned_layout: [[["overview"]], [["trade"]]],
    pinned_sections: ["overview", "trade"],
    unpinned_insert_index: [0, 0],
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
  env.window.__fishystuffCalculator.patchSignals({
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

test("calculator presets wait for init defaults before tracking default current state", () => {
  const env = createContext();
  const signals = defaultSignals();
  const initDefaults = cloneTestValue(signals._defaults);
  delete signals._defaults;

  env.window.__fishystuffCalculator.restore(signals);

  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets"), null);
  env.window.__fishystuffCalculator.patchSignals({
    _loading: false,
    _defaults: initDefaults,
  });

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-presets"), "");
  assert.equal(env.window.__fishystuffUserPresets.selectedFixedId("calculator-presets"), "default");
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-presets"), null);
});

test("calculator restore applies the persisted selected layout preset before tracking current state", () => {
  const initial = createContext();
  const initialSignals = defaultSignals();
  initial.window.__fishystuffCalculator.restore(initialSignals);
  const preset = initial.window.__fishystuffUserPresets.createPreset("calculator-layouts", {
    name: "Trade layout",
    payload: {
      pinned_layout: [[["overview"]], [["trade"]]],
      unpinned_insert_index: [1, 0],
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
  const initDefaults = cloneTestValue(signals._defaults);
  signals._calculator_ui = defaultCalculatorUiState();
  env.window.__fishystuffCalculator.patchSignals({
    _loading: false,
    _defaults: initDefaults,
  });

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-layouts"), preset.id);
  assert.equal(env.window.__fishystuffUserPresets.current("calculator-layouts"), null);
  assert.deepEqual(JSON.parse(JSON.stringify(env.window.__fishystuffCalculator.layoutPresetPayload(signals._calculator_ui))), {
    pinned_layout: [[["overview"]], [["trade"]]],
    unpinned_insert_index: [1, 0],
  });
});

test("calculator restore preserves a persisted modified current layout preset", () => {
  const initial = createContext();
  const initialSignals = defaultSignals();
  initial.window.__fishystuffCalculator.restore(initialSignals);
  const preset = initial.window.__fishystuffUserPresets.createPreset("calculator-layouts", {
    name: "Base layout",
    payload: {
      pinned_layout: [[["overview"]], [["zone"]]],
      unpinned_insert_index: [0, 0],
    },
    select: true,
  });
  initial.window.__fishystuffUserPresets.trackCurrentPayload("calculator-layouts", {
    payload: {
      pinned_layout: [[["overview"]], [["trade"]]],
      unpinned_insert_index: [1, 0],
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
  env.window.__fishystuffCalculator.patchSignals({
    _loading: false,
    _defaults: cloneTestValue(signals._defaults),
    _calculator_ui: defaultCalculatorUiState(),
  });

  assert.equal(env.window.__fishystuffUserPresets.selectedPresetId("calculator-layouts"), preset.id);
  assert.deepEqual(JSON.parse(JSON.stringify(env.window.__fishystuffUserPresets.current("calculator-layouts")?.payload)), {
    pinned_layout: [[["overview"]], [["trade"]]],
    unpinned_insert_index: [1, 0],
  });
  assert.deepEqual(JSON.parse(JSON.stringify(env.window.__fishystuffCalculator.layoutPresetPayload(signals._calculator_ui))), {
    pinned_layout: [[["overview"]], [["trade"]]],
    unpinned_insert_index: [1, 0],
  });
});

test("calculator layout preset title icon follows the first pinned section", () => {
  const env = createContext();
  const calculator = env.window.__fishystuffCalculator;

  assert.equal(
    calculator.layoutPresetTitleIconAlias({
      pinned_layout: [[["zone"], ["loot"]], [["buffs"]]],
      unpinned_insert_index: [0, 0],
    }),
    "fullscreen-fill",
  );
  assert.equal(
    calculator.layoutPresetTitleIconAlias({
      pinned_layout: [[["gear"]], [["debug"]]],
      unpinned_insert_index: [0, 0],
    }),
    "bug-fill",
  );
  assert.equal(
    calculator.layoutPresetTitleIconAlias({
      pinned_layout: [],
      unpinned_insert_index: [0, 0],
    }),
    "",
  );
});

test("calculator API URLs keep locale and apiLang separate", () => {
  const korean = createContext({}, { locale: "ko-KR", lang: "en-US" });
  assert.equal(korean.window.__fishystuffCalculator.lang, "ko");
  assert.equal(korean.window.__fishystuffCalculator.locale, "ko-KR");
  assert.equal(korean.window.__fishystuffCalculator.apiLang, "ko");
  assert.match(korean.window.__fishystuffCalculator.initUrl(), /\?lang=ko&locale=ko-KR$/);
  assert.match(korean.window.__fishystuffCalculator.evalUrl(), /\?lang=ko&locale=ko-KR$/);

  const german = createContext({}, { locale: "de-DE", lang: "en-US" });
  assert.equal(german.window.__fishystuffCalculator.lang, "en");
  assert.equal(german.window.__fishystuffCalculator.locale, "de-DE");
  assert.equal(german.window.__fishystuffCalculator.apiLang, "en");
  assert.match(german.window.__fishystuffCalculator.initUrl(), /\?lang=en&locale=de-DE$/);
  assert.match(german.window.__fishystuffCalculator.evalUrl(), /\?lang=en&locale=de-DE$/);

  const mixed = createContext({}, { locale: "de-DE", apiLang: "ko", lang: "en-US" });
  assert.equal(mixed.window.__fishystuffCalculator.lang, "ko");
  assert.equal(mixed.window.__fishystuffCalculator.locale, "de-DE");
  assert.equal(mixed.window.__fishystuffCalculator.apiLang, "ko");
  assert.match(mixed.window.__fishystuffCalculator.initUrl(), /\?lang=ko&locale=de-DE$/);
  assert.match(mixed.window.__fishystuffCalculator.evalUrl(), /\?lang=ko&locale=de-DE$/);
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

test("calculator action listener handles copy and clear tokens once without clearing shared overlays", () => {
  const env = createContext({
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
        "8473": {
          basePrice: 8800000,
        },
      },
    }),
  });
  const signals = defaultSignals();
  Object.assign(signals, {
    active: true,
    food: ["item:9359"],
    _calculator_actions: defaultCalculatorActionState({
      copyUrlToken: 1,
      copyShareToken: 1,
      clearToken: 1,
    }),
  });

  env.localStorage.setItem(
    "fishystuff.calculator.data.v1",
    JSON.stringify({ food: ["item:9359"] }),
  );
  env.localStorage.setItem(
    "fishystuff.calculator.ui.v1",
    JSON.stringify(defaultCalculatorUiState({
      unpinned_insert_index: [2, 0],
    })),
  );
  env.window.__fishystuffCalculator.restore(signals);
  signals._calculator_ui = {
    top_level_tab: "loot",
    distribution_tab: "loot_flow",
    pinned_layout: [[["overview"], ["distribution"]]],
    pinned_sections: ["overview", "distribution"],
    unpinned_insert_index: [2, 0],
  };
  signals.overlay = {
    zones: {
      stale: {
        groups: {
          2: {
            rawRatePercent: 5,
          },
        },
        items: {},
      },
    },
  };
  signals.priceOverrides = {
    "9999": {
      basePrice: 1,
    },
  };
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _calculator_actions: {
        copyUrlToken: 1,
        copyShareToken: 1,
        clearToken: 1,
        resetLayoutToken: 0,
      },
    },
  });
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _calculator_actions: {
        copyUrlToken: 0,
        copyShareToken: 0,
        clearToken: 0,
        resetLayoutToken: 0,
      },
    },
  });

  assert.equal(env.toastCalls.length, 3);
  assert.equal(env.toastCalls[0].type, "copyText");
  assert.match(env.toastCalls[0].text, /\?preset=lz:/);
  assert.equal(env.toastCalls[0].options.success, calculatorMessage("toast.preset_url_copied"));
  assert.equal(env.toastCalls[1].type, "copyText");
  assert.match(env.toastCalls[1].text, /Fishy Stuff Calculator Preset/);
  assert.equal(env.toastCalls[1].options.success, calculatorMessage("toast.share_copied"));
  assert.equal(env.toastCalls[2].type, "info");
  assert.equal(env.toastCalls[2].message, calculatorMessage("toast.cleared"));
  assert.deepEqual(Array.from(signals.food), []);
  assert.equal(env.localStorage.getItem("fishystuff.calculator.data.v1"), null);
  assert.deepEqual(
    JSON.parse(env.localStorage.getItem("fishystuff.calculator.ui.v1")),
    {
      top_level_tab: "loot",
      distribution_tab: "loot_flow",
      pinned_layout: [[["overview"], ["distribution"]]],
      pinned_sections: ["overview", "distribution"],
      unpinned_insert_index: [2, 0],
    },
  );
  assert.deepEqual(signals._calculator_ui, {
    top_level_tab: "loot",
    distribution_tab: "loot_flow",
    pinned_layout: [[["overview"], ["distribution"]]],
    pinned_sections: ["overview", "distribution"],
    unpinned_insert_index: [2, 0],
  });
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
    "8473": {
      basePrice: 8800000,
    },
  });
  assert.deepEqual(env.window.__fishystuffUserOverlays.snapshot(), {
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
      "8473": {
        basePrice: 8800000,
      },
    },
  });
});

test("calculator action listener resets only layout state", () => {
  const env = createContext();
  const signals = defaultSignals();

  env.window.__fishystuffCalculator.restore(signals);
  signals._calculator_ui = {
    top_level_tab: "trade",
    distribution_tab: "target_fish",
    pinned_layout: [[["overview"], ["distribution"]]],
    pinned_sections: ["overview", "distribution"],
    unpinned_insert_index: [3, 2],
  };
  signals._calculator_actions = defaultCalculatorActionState({
    resetLayoutToken: 1,
  });

  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _calculator_actions: {
        copyUrlToken: 0,
        copyShareToken: 0,
        clearToken: 0,
        resetLayoutToken: 1,
      },
    },
  });
  env.flushTimers();

  assert.equal(env.toastCalls.length, 1);
  assert.equal(env.toastCalls[0].type, "info");
  assert.equal(env.toastCalls[0].message, calculatorMessage("toast.layout_reset"));
  assert.deepEqual(JSON.parse(JSON.stringify(signals._calculator_ui)), {
    top_level_tab: "trade",
    distribution_tab: "target_fish",
    pinned_layout: cloneTestValue(DEFAULT_PINNED_LAYOUT),
    pinned_sections: Array.from(DEFAULT_PINNED_SECTIONS),
    unpinned_insert_index: [0, 0],
  });
  assert.deepEqual(
    JSON.parse(env.localStorage.getItem("fishystuff.calculator.ui.v1")),
    {
      top_level_tab: "trade",
      distribution_tab: "target_fish",
      pinned_layout: cloneTestValue(DEFAULT_PINNED_LAYOUT),
      pinned_sections: Array.from(DEFAULT_PINNED_SECTIONS),
      unpinned_insert_index: [0, 0],
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
