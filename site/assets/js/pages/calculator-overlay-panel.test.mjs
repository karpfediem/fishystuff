import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const SOURCE = fs.readFileSync(
  new URL("./calculator-overlay-panel.js", import.meta.url),
  "utf8",
);

function createContext() {
  const registry = new Map();
  const toastCalls = [];
  const overlayState = {
    zones: {
      "zone-a": {
        groups: {
          2: { rawRatePercent: 100 },
        },
        items: {},
      },
    },
  };
  const priceOverrides = {
    123: { basePrice: 999 },
  };
  const calculatorSignals = {
    zone: "zone-a",
    overlay: JSON.parse(JSON.stringify(overlayState)),
    priceOverrides: JSON.parse(JSON.stringify(priceOverrides)),
    _calc: {
      zone_name: "Zone A",
      overlay_editor: {
        zone_rgb_key: "zone-a",
        zone_name: "Zone A",
        groups: [],
        items: [],
      },
    },
  };

  const context = {
    console,
    JSON,
    Object,
    Array,
    String,
    Number,
    Boolean,
    Map,
    Set,
    RegExp,
    Error,
    Date,
    Blob: class Blob {},
    URL: {
      createObjectURL() {
        return "blob:test";
      },
      revokeObjectURL() {},
    },
    CustomEvent: class CustomEvent {
      constructor(type, options = {}) {
        this.type = type;
        this.detail = options.detail;
      }
    },
    HTMLElement: class HTMLElement {
      addEventListener() {}
      removeEventListener() {}
      querySelector() {
        return null;
      }
    },
    customElements: {
      get(name) {
        return registry.get(name);
      },
      define(name, ctor) {
        registry.set(name, ctor);
      },
    },
    document: {
      body: {
        appendChild() {},
      },
      createElement() {
        return {
          click() {},
          remove() {},
        };
      },
      addEventListener() {},
      removeEventListener() {},
      dispatchEvent() {},
    },
    window: {
      __fishystuffCalculator: {
        signalObject() {
          return calculatorSignals;
        },
        patchSignals(patch) {
          Object.assign(calculatorSignals, patch);
        },
      },
      __fishystuffUserOverlays: {
        CHANGED_EVENT: "fishystuff:user-overlays-changed",
        overlaySignals() {
          return JSON.parse(JSON.stringify(overlayState));
        },
        priceOverrides() {
          return JSON.parse(JSON.stringify(priceOverrides));
        },
        setOverlaySignals(nextOverlay) {
          overlayState.zones = JSON.parse(JSON.stringify(nextOverlay.zones || {}));
        },
        setPriceOverrides(nextPriceOverrides) {
          for (const key of Object.keys(priceOverrides)) {
            delete priceOverrides[key];
          }
          Object.assign(priceOverrides, JSON.parse(JSON.stringify(nextPriceOverrides)));
        },
        importText(text) {
          const payload = JSON.parse(String(text ?? ""));
          overlayState.zones = JSON.parse(JSON.stringify(payload.overlay?.zones || {}));
          for (const key of Object.keys(priceOverrides)) {
            delete priceOverrides[key];
          }
          Object.assign(priceOverrides, JSON.parse(JSON.stringify(payload.priceOverrides || {})));
          return {
            overlay: JSON.parse(JSON.stringify({ zones: overlayState.zones })),
            priceOverrides: JSON.parse(JSON.stringify(priceOverrides)),
          };
        },
        clearAll() {
          overlayState.zones = {};
          for (const key of Object.keys(priceOverrides)) {
            delete priceOverrides[key];
          }
        },
      },
      __fishystuffToast: {
        info(message) {
          toastCalls.push({ tone: "info", message });
        },
        success(message) {
          toastCalls.push({ tone: "success", message });
        },
        error(message) {
          toastCalls.push({ tone: "error", message });
        },
      },
      document: null,
      addEventListener() {},
      removeEventListener() {},
      dispatchEvent() {},
    },
    globalThis: null,
  };
  context.window.document = context.document;
  context.globalThis = context;

  vm.runInNewContext(SOURCE, context, { filename: "calculator-overlay-panel.js" });

  return {
    context,
    calculatorSignals,
    overlayState,
    registry,
    toastCalls,
  };
}

test("overlay panel replaces live overlay root when clearing a zone overlay", () => {
  const env = createContext();
  const Panel = env.registry.get("fishy-calculator-overlay-panel");
  const panel = new Panel();

  panel.writeZoneOverlay("zone-a", {});

  assert.deepEqual(env.overlayState, { zones: {} });
  assert.deepEqual(env.calculatorSignals.overlay, { zones: {} });
});

test("overlay panel replaces live price overrides root when clearing all", () => {
  const env = createContext();
  const Panel = env.registry.get("fishy-calculator-overlay-panel");
  const panel = new Panel();

  panel.writePriceOverrides({});

  assert.deepEqual(env.calculatorSignals.priceOverrides, {});
});

test("overlay panel restore removes a group overlay from live calculator signals", () => {
  const env = createContext();
  const Panel = env.registry.get("fishy-calculator-overlay-panel");
  const panel = new Panel();

  panel.resetEntry("group", "zone-a", "2", "");

  assert.deepEqual(env.overlayState, { zones: {} });
  assert.deepEqual(env.calculatorSignals.overlay, { zones: {} });
});

test("overlay panel import replaces live overlay and price roots", async () => {
  const env = createContext();
  const Panel = env.registry.get("fishy-calculator-overlay-panel");
  const panel = new Panel();
  const input = { value: "selected.json" };

  await panel.importOverlayFile({
    async text() {
      return JSON.stringify({
        format: "fishystuff-user-overlay-v2",
        overlay: {
          zones: {
            "zone-b": {
              groups: {
                1: { rawRatePercent: 10.0000001234 },
              },
              items: {},
            },
          },
        },
        priceOverrides: {
          999: { basePrice: 4567.89 },
        },
      });
    },
  }, input);

  assert.equal(input.value, "");
  assert.deepEqual(env.overlayState, {
    zones: {
      "zone-b": {
        groups: {
          1: { rawRatePercent: 10.0000001234 },
        },
        items: {},
      },
    },
  });
  assert.deepEqual(env.calculatorSignals.overlay, {
    zones: {
      "zone-b": {
        groups: {
          1: { rawRatePercent: 10.0000001234 },
        },
        items: {},
      },
    },
  });
  assert.deepEqual(env.calculatorSignals.priceOverrides, {
    999: { basePrice: 4567.89 },
  });
  assert.deepEqual(env.toastCalls, [{ tone: "success", message: "Overlay JSON imported." }]);
});
