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
const USER_OVERLAYS_SOURCE = fs.readFileSync(
  new URL("../user-overlays.js", import.meta.url),
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
  context.globalThis = context;
  vm.runInNewContext(DATASTAR_STATE_SOURCE, context, { filename: "datastar-state.js" });
  vm.runInNewContext(DATASTAR_PERSIST_SOURCE, context, { filename: "datastar-persist.js" });
  vm.runInNewContext(USER_OVERLAYS_SOURCE, context, { filename: "user-overlays.js" });
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
    overlay: { zones: {} },
    pet1: { skills: [] },
    pet2: { skills: [] },
    pet3: { skills: [] },
    pet4: { skills: [] },
    pet5: { skills: [] },
    _calculator_ui: {
      distribution_tab: "groups",
      overlay_panel_collapsed: true,
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
      overlay: { zones: {} },
      pet1: { skills: [] },
      pet2: { skills: [] },
      pet3: { skills: [] },
      pet4: { skills: [] },
      pet5: { skills: [] },
      _calculator_ui: {
        distribution_tab: "groups",
        overlay_panel_collapsed: true,
      },
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
    "fishystuff.calculator.data.v1": JSON.stringify({
      _active: true,
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
    "fishystuff.calculator.ui.v1": JSON.stringify({
      distribution_tab: "loot_flow",
      overlay_panel_collapsed: "false",
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
  assert.equal(signals._calculator_ui.overlay_panel_collapsed, false);
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

test("calculator restore leaves initial shell state intact when storage is empty", () => {
  const env = createContext();
  const signals = {
    _loading: true,
    _calculator_ui: {
      distribution_tab: "groups",
      overlay_panel_collapsed: true,
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
    overlay: {
      zones: {},
    },
    priceOverrides: {},
    _calculator_ui: {
      distribution_tab: "groups",
      overlay_panel_collapsed: true,
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

  const persistedData = JSON.parse(env.localStorage.getItem("fishystuff.calculator.data.v1"));
  const persistedUi = JSON.parse(env.localStorage.getItem("fishystuff.calculator.ui.v1"));
  assert.deepEqual(persistedData.food, ["item:9359"]);
  assert.deepEqual(persistedUi, {
    distribution_tab: "groups",
    overlay_panel_collapsed: true,
  });
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
    _calculator_actions: {
      copyUrlToken: 1,
      copyShareToken: 1,
      clearToken: 1,
    },
  });

  env.localStorage.setItem(
    "fishystuff.calculator.data.v1",
    JSON.stringify({ food: ["item:9359"] }),
  );
  env.localStorage.setItem(
    "fishystuff.calculator.ui.v1",
    JSON.stringify({
      distribution_tab: "groups",
      overlay_panel_collapsed: true,
    }),
  );
  env.window.__fishystuffCalculator.restore(signals);
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
  assert.equal(env.localStorage.getItem("fishystuff.calculator.data.v1"), null);
  assert.equal(env.localStorage.getItem("fishystuff.calculator.ui.v1"), null);
  assert.deepEqual(signals._calculator_ui, {
    distribution_tab: "groups",
    overlay_panel_collapsed: true,
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
        bite_time: JSON.stringify({
          title: "Average Bite Time",
          value_text: "0.00",
          sections: [{ label: "Inputs", rows: [] }, { label: "Composition", rows: [] }],
        }),
        auto_fish_time: JSON.stringify({
          title: "Auto-Fishing Time",
          value_text: "0.00",
          sections: [{ label: "Inputs", rows: [] }, { label: "Composition", rows: [] }],
        }),
        catch_time: JSON.stringify({
          title: "Catch Time",
          value_text: "0.00",
          sections: [{ label: "Inputs", rows: [] }, { label: "Composition", rows: [] }],
        }),
        time_saved: JSON.stringify({
          title: "Time Saved",
          value_text: "0.00%",
          sections: [{ label: "Inputs", rows: [] }, { label: "Composition", rows: [] }],
        }),
        casts_average: JSON.stringify({
          title: "Average Casts (8 hours)",
          value_text: "0.00",
          sections: [{ label: "Inputs", rows: [] }, { label: "Composition", rows: [] }],
        }),
        effective_bite_avg: JSON.stringify({
          title: "Effective Bite Average",
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
  const catchTime = JSON.parse(live.stat_breakdowns.catch_time);
  const timeSaved = JSON.parse(live.stat_breakdowns.time_saved);
  const effectiveBiteAverage = JSON.parse(live.stat_breakdowns.effective_bite_avg);
  const totalCatches = JSON.parse(live.stat_breakdowns.loot_total_catches);
  const profitPerHour = JSON.parse(live.stat_breakdowns.loot_profit_per_hour);

  assert.equal(totalTime.value_text, live.total_time);
  assert.equal(totalTime.sections[0].rows[0].value_text, live.bite_time);
  assert.equal(totalTime.sections[0].rows[1].label, "Auto-Fishing Time");
  assert.equal(totalTime.sections[1].rows[0].label, "Average total");
  assert.equal(castsAverage.title, "Average Casts (2 hours)");
  assert.equal(castsAverage.sections[0].rows[0].value_text, "2 hours");
  assert.equal(castsAverage.formula_terms[1].label, "Session seconds");
  assert.equal(castsAverage.formula_terms[1].value_text, "7200");
  assert.equal(catchTime.sections[0].rows[0].label, "AFK catch time");
  assert.equal(catchTime.sections[1].rows[0].value_text, "3.00");
  assert.equal(timeSaved.sections[1].rows[1].label, "Saved share");
  assert.equal(timeSaved.value_text, "45.64%");
  assert.equal(effectiveBiteAverage.formula_terms[1].label, "Zone Bite Average");
  assert.equal(effectiveBiteAverage.formula_terms[1].value_text, "15.00");
  assert.equal(totalCatches.title, "Expected Catches (2 hours)");
  assert.equal(totalCatches.value_text, live.loot_total_catches);
  assert.equal(totalCatches.sections[0].rows[0].label, "Average casts");
  assert.equal(totalCatches.sections[0].rows[0].value_text, live.casts_average);
  assert.equal(totalCatches.sections[1].rows[0].label, "Expected catches");
  assert.equal(profitPerHour.sections[0].rows[0].value_text, live.loot_total_profit);
  assert.equal(profitPerHour.value_text, live.loot_profit_per_hour);
  assert.equal(live.fishing_timeline_chart.segments.length, 4);
  assert.equal(live.fishing_timeline_chart.segments[0].label, "Bite Time");
  assert.equal(live.fishing_timeline_chart.segments[2].label, "Catch Time");
  assert.equal(live.fishing_timeline_chart.segments[3].label, "Time Saved");
  assert.equal(live.fishing_timeline_chart.segments[0].breakdown.title, "Average Bite Time");
  assert.equal(live.fishing_timeline_chart.segments[3].breakdown.title, "Time Saved");
});
