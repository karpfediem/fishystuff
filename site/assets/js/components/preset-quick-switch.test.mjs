import { test } from "bun:test";
import assert from "node:assert/strict";

import {
  applyPresetQuickSwitchOption,
  buildPresetQuickSwitchRow,
  buildPresetQuickSwitchRows,
  filterPresetQuickSwitchOptions,
  normalizePresetQuickSwitchEntry,
} from "./preset-quick-switch.js";

function translate(_key, fallback, vars = {}) {
  return String(fallback ?? "").replace(/\{\s*\$([A-Za-z0-9_]+)\s*\}/g, (_match, name) => (
    Object.prototype.hasOwnProperty.call(vars, name) ? String(vars[name]) : ""
  ));
}

function fakeHelper({
  adapters = {},
  collections = {},
  fixedPresets = {},
  calls = [],
} = {}) {
  return {
    collectionAdapter(collectionKey) {
      return adapters[collectionKey] || null;
    },
    collection(collectionKey) {
      return collections[collectionKey] || {
        selectedPresetId: "",
        selectedFixedId: "",
        current: null,
        presets: [],
      };
    },
    fixedPresets(collectionKey) {
      return fixedPresets[collectionKey] || [];
    },
    current(collectionKey) {
      return this.collection(collectionKey).current || null;
    },
    activatePreset(collectionKey, presetId) {
      calls.push(["activatePreset", collectionKey, presetId]);
      return { id: presetId };
    },
    activateFixedPreset(collectionKey, fixedId) {
      calls.push(["activateFixedPreset", collectionKey, fixedId]);
      return { id: fixedId };
    },
    applyPayload(collectionKey, payload) {
      calls.push(["applyPayload", collectionKey, payload]);
      return { ...payload, applied: true };
    },
    trackCurrentPayload(collectionKey, options) {
      calls.push(["trackCurrentPayload", collectionKey, options]);
      return { action: "updated-current", current: options };
    },
  };
}

test("normalizePresetQuickSwitchEntry keeps future preset types data-driven", () => {
  assert.deepEqual(
    normalizePresetQuickSwitchEntry({
      collectionKey: " Zone Presets ",
      labelFallback: "Zones",
      order: 4,
      fixedFallbacks: [{ id: "default", labelFallback: "Default zones" }],
    }),
    {
      collectionKey: "zone-presets",
      labelKey: "",
      labelFallback: "Zones",
      order: 4,
      fixedFallbacks: [{ id: "default", labelKey: "", labelFallback: "Default zones" }],
    },
  );
});

test("buildPresetQuickSwitchRow shows modified current state linked to its origin", () => {
  const helper = fakeHelper({
    adapters: {
      "calculator-layouts": { titleFallback: "Layout presets" },
    },
    fixedPresets: {
      "calculator-layouts": [{ id: "default", name: "Default", payload: { layout: "default" } }],
    },
    collections: {
      "calculator-layouts": {
        selectedPresetId: "alpha",
        selectedFixedId: "",
        current: {
          origin: { kind: "preset", id: "alpha" },
          payload: { layout: "modified" },
        },
        presets: [{ id: "alpha", name: "Alpha", payload: { layout: "saved" } }],
      },
    },
  });

  const row = buildPresetQuickSwitchRow(
    helper,
    { collectionKey: "calculator-layouts", labelFallback: "Layout" },
    translate,
  );

  assert.equal(row.selectedLabel, "Modified: Alpha");
  assert.equal(row.selectedStatus, "Modified");
  assert.deepEqual(
    row.options.map((option) => [option.kind, option.id, option.label]),
    [
      ["current", "preset:alpha", "Modified: Alpha"],
      ["fixed", "default", "Default"],
      ["preset", "alpha", "Alpha"],
    ],
  );
});

test("buildPresetQuickSwitchRows accepts a future preset collection as another entry", () => {
  const rows = buildPresetQuickSwitchRows(
    fakeHelper({
      adapters: {
        "zone-presets": { titleFallback: "Zone presets" },
      },
      fixedPresets: {
        "zone-presets": [{ id: "default", name: "Default zones", payload: { zone: "all" } }],
      },
    }),
    [{ collectionKey: "zone-presets", labelFallback: "Zones" }],
    translate,
  );

  assert.equal(rows.length, 1);
  assert.equal(rows[0].collectionKey, "zone-presets");
  assert.equal(rows[0].selectedLabel, "Default zones");
  assert.equal(rows[0].options[0].key, "fixed:default");
});

test("buildPresetQuickSwitchRow keeps saved selection switchable without a page adapter", () => {
  const row = buildPresetQuickSwitchRow(
    fakeHelper({
      collections: {
        "map-presets": {
          selectedPresetId: "night",
          selectedFixedId: "",
          current: null,
          presets: [{ id: "night", name: "Night fishing", payload: { layers: ["night"] } }],
        },
      },
    }),
    {
      collectionKey: "map-presets",
      labelFallback: "Map",
      fixedFallbacks: [{ id: "default", labelFallback: "Default map" }],
    },
    translate,
  );

  assert.equal(row.selectedLabel, "Night fishing");
  assert.deepEqual(
    row.options.map((option) => [option.kind, option.id, option.label]),
    [
      ["fixed", "default", "Default map"],
      ["preset", "night", "Night fishing"],
    ],
  );
});

test("filterPresetQuickSwitchOptions searches preset names and ids", () => {
  const options = [
    { label: "Default map", searchText: "Default map default" },
    { label: "Night fishing", searchText: "Night fishing night" },
  ];

  assert.deepEqual(filterPresetQuickSwitchOptions(options, "saved").map((option) => option.label), []);
  assert.deepEqual(filterPresetQuickSwitchOptions(options, "map").map((option) => option.label), [
    "Default map",
  ]);
  assert.deepEqual(filterPresetQuickSwitchOptions(options, "night").map((option) => option.label), [
    "Night fishing",
  ]);
});

test("applyPresetQuickSwitchOption delegates saved, fixed, and current application consistently", () => {
  const calls = [];
  const helper = fakeHelper({
    calls,
    collections: {
      "calculator-layouts": {
        selectedPresetId: "",
        selectedFixedId: "",
        current: {
          origin: { kind: "preset", id: "layout-a" },
          payload: { layout: "modified" },
        },
        presets: [],
      },
    },
  });

  applyPresetQuickSwitchOption(helper, {
    collectionKey: "map-presets",
    kind: "fixed",
    id: "default",
  });
  applyPresetQuickSwitchOption(helper, {
    collectionKey: "calculator-presets",
    kind: "preset",
    id: "alpha",
  });
  applyPresetQuickSwitchOption(helper, {
    collectionKey: "calculator-layouts",
    kind: "current",
    id: "preset:layout-a",
    source: { kind: "preset", id: "layout-a" },
    payload: { layout: "modified" },
  });

  assert.deepEqual(calls, [
    ["activateFixedPreset", "map-presets", "default"],
    ["activatePreset", "calculator-presets", "alpha"],
  ]);
});
