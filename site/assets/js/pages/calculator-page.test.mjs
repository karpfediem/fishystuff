import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const DATASTAR_STATE_SOURCE = fs.readFileSync(
  new URL("../datastar-state.js", import.meta.url),
  "utf8",
);
const DATASTAR_PERSIST_SOURCE = fs.readFileSync(
  new URL("../datastar-persist.js", import.meta.url),
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
  const listeners = new Map();
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
    documentElement: {
      lang: options.lang || "en-US",
    },
    addEventListener(type, listener) {
      if (!listeners.has(type)) {
        listeners.set(type, []);
      }
      listeners.get(type).push(listener);
    },
    removeEventListener(type, listener) {
      const current = listeners.get(type) || [];
      listeners.set(
        type,
        current.filter((candidate) => candidate !== listener),
      );
    },
    dispatchEvent(event) {
      for (const listener of listeners.get(event.type) || []) {
        listener(event);
      }
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
  context.globalThis = context;
  vm.runInNewContext(DATASTAR_STATE_SOURCE, context, { filename: "datastar-state.js" });
  vm.runInNewContext(DATASTAR_PERSIST_SOURCE, context, { filename: "datastar-persist.js" });
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
  assert.equal(env.window.__fishystuffCalculator.signalObject(), signals);
});

test("calculator restore leaves initial shell state intact when storage is empty", () => {
  const env = createContext();
  const signals = {
    _loading: true,
    _calculator_ui: {
      distribution_tab: "groups",
    },
    _calculator_actions: {
      copyUrlToken: 0,
      copyShareToken: 0,
      clearToken: 0,
    },
  };

  env.window.__fishystuffCalculator.restore(signals);

  assert.deepEqual(JSON.parse(JSON.stringify(signals)), {
    _loading: true,
    _calculator_ui: {
      distribution_tab: "groups",
    },
    _calculator_actions: {
      copyUrlToken: 0,
      copyShareToken: 0,
      clearToken: 0,
    },
  });
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

  const persisted = JSON.parse(env.localStorage.getItem("calculator"));
  assert.deepEqual(persisted.food, ["item:9359"]);
  assert.deepEqual(persisted._calculator_ui, { distribution_tab: "groups" });
  assert.equal("_live" in persisted, false);
  assert.equal("_calc" in persisted, false);
  assert.equal("_defaults" in persisted, false);
});

test("calculator action listener handles copy and clear tokens once", () => {
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
  env.document.dispatchEvent({
    type: "datastar-signal-patch",
    detail: {
      _calculator_actions: {
        copyUrlToken: 1,
        copyShareToken: 1,
        clearToken: 1,
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
      },
    },
  });

  assert.equal(env.toastCalls.length, 3);
  assert.equal(env.toastCalls[0].type, "copyText");
  assert.match(env.toastCalls[0].text, /\?preset=lz:/);
  assert.equal(env.toastCalls[1].type, "copyText");
  assert.match(env.toastCalls[1].text, /FishyStuff Calculator Preset/);
  assert.deepEqual(Array.from(signals.food), []);
  assert.equal(env.localStorage.getItem("calculator"), null);
  assert.deepEqual(signals._calculator_ui, { distribution_tab: "groups" });
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
          title: "Average Total Fishing Time",
          value_text: "0.00",
          sections: [{ label: "Inputs", rows: [] }, { label: "Composition", rows: [] }],
        }),
        casts_average: JSON.stringify({
          title: "Average Casts (8 hours)",
          value_text: "0.00",
          sections: [{ label: "Inputs", rows: [] }, { label: "Composition", rows: [] }],
        }),
        loot_total_catches: JSON.stringify({
          title: "Expected Catches (8 hours)",
          value_text: "0.00",
          sections: [
            { label: "Inputs", rows: [] },
            { label: "Composition", rows: [] },
          ],
        }),
        loot_profit_per_hour: JSON.stringify({
          title: "Profit / Hour",
          value_text: "0",
          sections: [{ label: "Inputs", rows: [] }, { label: "Composition", rows: [] }],
        }),
      },
    },
  );

  const totalTime = JSON.parse(live.stat_breakdowns.total_time);
  const castsAverage = JSON.parse(live.stat_breakdowns.casts_average);
  const totalCatches = JSON.parse(live.stat_breakdowns.loot_total_catches);
  const profitPerHour = JSON.parse(live.stat_breakdowns.loot_profit_per_hour);

  assert.equal(totalTime.value_text, live.total_time);
  assert.equal(totalTime.sections[0].rows[0].value_text, live.bite_time);
  assert.equal(totalTime.sections[0].rows[1].label, "Auto-Fishing Time");
  assert.equal(totalTime.sections[1].rows[0].label, "Average total");
  assert.equal(castsAverage.title, "Average Casts (2 hours)");
  assert.equal(castsAverage.sections[0].rows[0].value_text, "2 hours");
  assert.equal(totalCatches.title, "Expected Catches (2 hours)");
  assert.equal(totalCatches.value_text, live.loot_total_catches);
  assert.equal(totalCatches.sections[0].rows[0].label, "Average casts");
  assert.equal(totalCatches.sections[0].rows[0].value_text, live.casts_average);
  assert.equal(totalCatches.sections[1].rows[0].label, "Expected catches");
  assert.equal(profitPerHour.sections[0].rows[0].value_text, live.loot_total_profit);
  assert.equal(profitPerHour.value_text, live.loot_profit_per_hour);
});
