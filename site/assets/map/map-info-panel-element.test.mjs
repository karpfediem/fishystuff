import { test } from "bun:test";
import assert from "node:assert/strict";
import { installMapTestI18n } from "./test-i18n.js";

import {
  FishyMapInfoPanelElement,
  readMapInfoPanelShellSignals,
  registerFishyMapInfoPanelElement,
} from "./map-info-panel-element.js";

function detailSectionFact(key, label, value, icon) {
  return {
    id: key,
    kind: "facts",
    title: label,
    facts: [
      {
        key,
        label,
        value,
        icon,
      },
    ],
    targets: [],
  };
}

function renderSlot() {
  return {
    dataset: {},
    hidden: false,
    innerHTML: "",
    textContent: "",
  };
}

test("readMapInfoPanelShellSignals prefers live shell signals over initial signals", () => {
  const initialSignals = { _map_runtime: { selection: { pointLabel: "Initial" } } };
  const liveSignals = { _map_runtime: { selection: { pointLabel: "Live" } } };
  const shell = {
    __fishymapInitialSignals: initialSignals,
    __fishymapLiveSignals: liveSignals,
  };

  assert.equal(readMapInfoPanelShellSignals(shell), liveSignals);
});

test("registerFishyMapInfoPanelElement defines the custom element once", () => {
  const registry = {
    definitions: new Map(),
    get(name) {
      return this.definitions.get(name) || null;
    },
    define(name, constructor) {
      this.definitions.set(name, constructor);
    },
  };

  assert.equal(registerFishyMapInfoPanelElement(registry), true);
  assert.equal(registerFishyMapInfoPanelElement(registry), true);
  assert.equal(registry.definitions.size, 1);
  assert.ok(registry.get("fishymap-info-panel"));
});

test("info panel element exposes refresh and signal patch handlers", () => {
  const element = new FishyMapInfoPanelElement();
  assert.equal(typeof element.handleSignalPatch, "function");
  assert.equal(typeof element.refreshZoneLootSummary, "function");
  assert.equal(typeof element.render, "function");
});

test("render shows the calculator warning and a consolidated calculator notice without the defaults badge", () => {
  const element = new FishyMapInfoPanelElement();
  const panelSlot = renderSlot();
  element._shell = {
    __fishymapInitialSignals: {
      _map_runtime: {
        selection: {
          pointKind: "clicked",
          pointLabel: "Valencia Sea - Depth 5",
          layerSamples: [
            {
              layerId: "zone_mask",
              rgbU32: 0x39e58d,
              rgb: [57, 229, 141],
              detailSections: [detailSectionFact("zone", "Zone", "Valencia Sea - Depth 5", "hover-zone")],
            },
          ],
        },
        catalog: {
          layers: [{ layerId: "zone_mask", displayOrder: 20 }],
        },
      },
    },
  };
  element._state = {
    zoneCatalog: [{ zoneRgb: 0x39e58d, name: "Valencia Sea - Depth 5", biteTimeMin: 5, biteTimeMax: 7 }],
    zoneLootStatus: "loaded",
    zoneLootSummary: {
      available: true,
      profileLabel: "Calculator defaults",
      dataQualityNote:
        "Expected loot uses average session casts, the current Fish multiplier, normalized group shares, and actual source-backed item prices.",
      note:
        "Groups follow the current calculator ordering, and rows show each fish or item's in-group droprate.",
      groups: [
        {
          slotIdx: 4,
          label: "General",
          fillColor: "#eef6ff",
          strokeColor: "#89a8d8",
          textColor: "#1f2937",
          dropRateText: "80%",
          dropRateSourceKind: "database",
          dropRateTooltip: "Source-backed General group share",
          conditionText: "Zone base rate 80%",
          conditionTooltip: "Zone base rate: 80%",
          catchMethods: ["rod"],
        },
      ],
      speciesRows: [
        {
          slotIdx: 4,
          groupLabel: "General",
          label: "Sea Eel",
          dropRateText: "80%",
          catchMethods: ["rod"],
        },
      ],
    },
    zoneLootRgb: 0x39e58d,
    zoneLootRequestToken: 1,
  };
  element._elements = {
    title: renderSlot(),
    titleIcon: renderSlot(),
    statusIcon: renderSlot(),
    statusText: renderSlot(),
    tabs: renderSlot(),
    panel: panelSlot,
  };

  element.render();

  assert.match(panelSlot.innerHTML, /Data Quality Warning/);
  assert.match(panelSlot.innerHTML, /The data we currently have is INCOMPLETE/);
  assert.match(panelSlot.innerHTML, /Calculator Inputs Used/);
  assert.match(panelSlot.innerHTML, /Expected loot uses average session casts/);
  assert.match(panelSlot.innerHTML, /Groups follow the current calculator ordering/);
  assert.doesNotMatch(panelSlot.innerHTML, /Calculator defaults/);
});

test("condition arrow buttons switch the visible zone loot branch", () => {
  const element = new FishyMapInfoPanelElement();
  const panelSlot = renderSlot();
  element._shell = {
    __fishymapInitialSignals: {
      _map_runtime: {
        selection: {
          pointKind: "clicked",
          pointLabel: "Velia Event",
          layerSamples: [
            {
              layerId: "zone_mask",
              rgbU32: 0x39e58d,
              rgb: [57, 229, 141],
              detailSections: [detailSectionFact("zone", "Zone", "Velia Event", "hover-zone")],
            },
          ],
        },
        catalog: {
          layers: [{ layerId: "zone_mask", displayOrder: 20 }],
        },
      },
    },
  };
  element._state = {
    zoneCatalog: [{ zoneRgb: 0x39e58d, name: "Velia Event", biteTimeMin: 5, biteTimeMax: 7 }],
    zoneLootStatus: "loaded",
    zoneLootRgb: 0x39e58d,
    zoneLootRequestToken: 1,
    zoneLootConditionSelection: {},
    zoneLootSummary: {
      available: true,
      profileLabel: "Calculator defaults",
      dataQualityNote: "Expected loot uses average session casts.",
      note: "Groups follow the current calculator ordering.",
      groups: [
        {
          slotIdx: 2,
          label: "Rare",
          dropRateText: "1%",
          catchMethods: ["rod"],
          conditionText: "Default",
          conditionOptions: [
            {
              conditionText: "Default",
              active: true,
              speciesRows: [
                {
                  slotIdx: 2,
                  groupLabel: "Rare",
                  label: "Grunt",
                  dropRateText: "100%",
                  catchMethods: ["rod"],
                },
              ],
            },
            {
              conditionText: "Fishing Level Guru 1+",
              active: false,
              speciesRows: [
                {
                  slotIdx: 2,
                  groupLabel: "Rare",
                  label: "Mystical Fish",
                  dropRateText: "0.005%",
                  catchMethods: ["rod"],
                },
              ],
            },
          ],
        },
      ],
      speciesRows: [],
    },
  };
  element._elements = {
    title: renderSlot(),
    titleIcon: renderSlot(),
    statusIcon: renderSlot(),
    statusText: renderSlot(),
    tabs: renderSlot(),
    panel: panelSlot,
  };

  element.render();

  assert.match(panelSlot.innerHTML, /data-zone-loot-condition-direction="1"/);
  assert.match(panelSlot.innerHTML, /Default/);
  assert.match(panelSlot.innerHTML, /Grunt/);

  const button = {
    getAttribute(name) {
      return {
        "data-zone-loot-condition-key": "2:Rare",
        "data-zone-loot-condition-direction": "1",
        "data-zone-loot-condition-current": "0",
        "data-zone-loot-condition-count": "2",
      }[name];
    },
  };
  element._handleClick({
    preventDefault() {},
    target: {
      closest(selector) {
        return selector === "button[data-zone-loot-condition-direction]" ? button : null;
      },
    },
  });

  assert.equal(element._state.zoneLootConditionSelection["2:Rare"], 1);
  assert.match(panelSlot.innerHTML, /Fishing Level Guru 1\+/);
  assert.match(panelSlot.innerHTML, /Mystical Fish/);
});
installMapTestI18n();
