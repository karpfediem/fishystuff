import { test } from "bun:test";
import assert from "node:assert/strict";

import {
  applyPresetQuickSwitchOption,
  buildPresetQuickSwitchRow,
  buildPresetQuickSwitchRows,
  filterPresetQuickSwitchOptions,
  normalizePresetQuickSwitchEntry,
  presetQuickSwitchEntries,
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
        workingCopies: [],
        activeWorkingCopyId: "",
        presets: [],
      };
    },
    fixedPresets(collectionKey) {
      return fixedPresets[collectionKey] || [];
    },
    activateWorkingCopy(collectionKey, workingCopyId) {
      calls.push(["activateWorkingCopy", collectionKey, workingCopyId]);
      return { id: workingCopyId };
    },
    activatePreset(collectionKey, presetId) {
      calls.push(["activatePreset", collectionKey, presetId]);
      return { id: presetId };
    },
    activateFixedPreset(collectionKey, fixedId) {
      calls.push(["activateFixedPreset", collectionKey, fixedId]);
      return { id: fixedId };
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

test("buildPresetQuickSwitchRow shows modified working copy state linked to its origin", () => {
  const helper = fakeHelper({
    adapters: {
      "calculator-layouts": { titleFallback: "Workspace presets" },
    },
    fixedPresets: {
      "calculator-layouts": [{ id: "default", name: "Default", payload: { layout: "default" } }],
    },
    collections: {
      "calculator-layouts": {
        selectedPresetId: "alpha",
        selectedFixedId: "",
        activeWorkingCopyId: "work-alpha",
        workingCopies: [{
          id: "work-alpha",
          source: { kind: "preset", id: "alpha" },
          payload: { layout: "modified" },
          modified: true,
        }],
        presets: [{ id: "alpha", name: "Alpha", payload: { layout: "saved" } }],
      },
    },
  });

  const row = buildPresetQuickSwitchRow(
    helper,
    { collectionKey: "calculator-layouts", labelFallback: "Workspace" },
    translate,
  );

  assert.equal(row.selectedLabel, "Modified: Alpha");
  assert.equal(row.selectedStatus, "Modified");
  assert.deepEqual(
    row.options.map((option) => [option.kind, option.id, option.label]),
    [
      ["fixed", "default", "Default"],
      ["preset", "alpha", "Alpha"],
      ["working", "work-alpha", "Modified: Alpha"],
    ],
  );
});

test("buildPresetQuickSwitchRow keeps the selected source active over inactive dirty working copies", () => {
  const helper = fakeHelper({
    adapters: {
      "calculator-presets": { titleFallback: "Calculator presets" },
    },
    fixedPresets: {
      "calculator-presets": [{ id: "default", name: "Default", payload: { level: 0 } }],
    },
    collections: {
      "calculator-presets": {
        selectedPresetId: "",
        selectedFixedId: "default",
        activeWorkingCopyId: "work-default",
        workingCopies: [{
          id: "work-alpha",
          source: { kind: "preset", id: "alpha" },
          payload: { level: 42 },
          modified: true,
        }],
        presets: [{ id: "alpha", name: "Alpha", payload: { level: 20 } }],
      },
    },
  });

  const row = buildPresetQuickSwitchRow(
    helper,
    { collectionKey: "calculator-presets", labelFallback: "Calculator" },
    translate,
  );

  assert.equal(row.selectedLabel, "Default");
  assert.equal(row.selectedOptionKey, "fixed:default");
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

test("preset quick switch default entries include Fishydex presets", () => {
  const rows = buildPresetQuickSwitchRows(
    fakeHelper(),
    presetQuickSwitchEntries(),
    translate,
  );

  assert.deepEqual(
    rows.map((row) => [row.collectionKey, row.label, row.selectedLabel]),
    [
      ["calculator-layouts", "Workspace", "Default"],
      ["calculator-presets", "Calculator", "Default calculator"],
      ["map-presets", "Map", "Default map"],
      ["fishydex-presets", "Dex", "Default dex"],
    ],
  );
});

test("buildPresetQuickSwitchRow keeps saved selection switchable without a page adapter", () => {
  const row = buildPresetQuickSwitchRow(
    fakeHelper({
      collections: {
        "map-presets": {
          selectedPresetId: "night",
          selectedFixedId: "",
          workingCopies: [],
          activeWorkingCopyId: "",
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

test("applyPresetQuickSwitchOption delegates saved, fixed, and working-copy application consistently", () => {
  const calls = [];
  const helper = fakeHelper({
    calls,
    collections: {
      "calculator-layouts": {
        selectedPresetId: "",
        selectedFixedId: "",
        activeWorkingCopyId: "work-layout-a",
        workingCopies: [{
          id: "work-layout-a",
          source: { kind: "preset", id: "layout-a" },
          payload: { layout: "modified" },
          modified: true,
        }],
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
    kind: "working",
    id: "work-layout-a",
    source: { kind: "preset", id: "layout-a" },
    payload: { layout: "modified" },
  });

  assert.deepEqual(calls, [
    ["activateFixedPreset", "map-presets", "default"],
    ["activatePreset", "calculator-presets", "alpha"],
    ["activateWorkingCopy", "calculator-layouts", "work-layout-a"],
  ]);
});
