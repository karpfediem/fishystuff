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

test("normalize rates Datastar prop re-renders without refetching zone loot", () => {
  const element = new FishyMapInfoPanelElement();
  let refreshCount = 0;
  let renderCount = 0;
  element.refreshZoneLootSummary = () => {
    refreshCount += 1;
    return Promise.resolve();
  };
  element.scheduleRender = () => {
    renderCount += 1;
  };

  element.attributeChangedCallback("data-normalize-rates", "true", "false");

  assert.equal(refreshCount, 0);
  assert.equal(renderCount, 1);
});

test("render switches loaded zone loot rates from the Datastar normalize rates prop", () => {
  const element = new FishyMapInfoPanelElement();
  const panelSlot = renderSlot();
  let normalizeRates = "true";
  element.getAttribute = (name) => (name === "data-normalize-rates" ? normalizeRates : null);
  element._shell = {
    __fishymapInitialSignals: {
      _map_runtime: {
        selection: {
          pointKind: "clicked",
          pointLabel: "Velia Coast",
          layerSamples: [
            {
              layerId: "zone_mask",
              rgbU32: 0x39e58d,
              rgb: [57, 229, 141],
              detailSections: [detailSectionFact("zone", "Zone", "Velia Coast", "hover-zone")],
            },
          ],
        },
        catalog: {
          layers: [{ layerId: "zone_mask", displayOrder: 20 }],
        },
      },
      _map_ui: {
        windowUi: {
          zoneInfo: { tab: "" },
          settings: {},
        },
      },
    },
  };
  element._state = {
    zoneCatalog: [{ zoneRgb: 0x39e58d, name: "Velia Coast", biteTimeMin: 5, biteTimeMax: 7 }],
    zoneLootStatus: "loaded",
    zoneLootRgb: 0x39e58d,
    zoneLootRequestToken: 1,
    zoneLootConditionSelection: {},
    zoneLootSummary: {
      available: true,
      profileLabel: "Calculator defaults",
      groups: [
        {
          slotIdx: 2,
          label: "Rare",
          fillColor: "#eef6ff",
          strokeColor: "#89a8d8",
          textColor: "#1f2937",
          dropRateText: "77.7%",
          dropRateSourceKind: "database",
          rawDropRateText: "12.3%",
          rawDropRateTooltip: "Raw group rate",
          normalizedDropRateText: "77.7%",
          normalizedDropRateTooltip: "Normalized group rate",
          catchMethods: ["rod"],
        },
      ],
      speciesRows: [
        {
          slotIdx: 2,
          groupLabel: "Rare",
          label: "Grunt",
          dropRateText: "22.2%",
          rawDropRateText: "4.5%",
          rawDropRateTooltip: "Raw species rate",
          normalizedDropRateText: "22.2%",
          normalizedDropRateTooltip: "Normalized species rate",
          catchMethods: ["rod"],
        },
      ],
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

  assert.match(panelSlot.innerHTML, /77\.7%/);
  assert.match(panelSlot.innerHTML, /22\.2%/);
  assert.doesNotMatch(panelSlot.innerHTML, /12\.3%/);
  assert.doesNotMatch(panelSlot.innerHTML, /4\.5%/);

  normalizeRates = "false";
  element.attributeChangedCallback("data-normalize-rates", "true", "false");

  assert.match(panelSlot.innerHTML, /12\.3%/);
  assert.match(panelSlot.innerHTML, /4\.5%/);
  assert.doesNotMatch(panelSlot.innerHTML, /77\.7%/);
  assert.doesNotMatch(panelSlot.innerHTML, /22\.2%/);
});

test("normalize rates signal patch swaps loaded zone loot rates in place", () => {
  const element = new FishyMapInfoPanelElement();
  const panelSlot = renderSlot();
  const signals = {
    _map_runtime: {
      selection: {
        pointKind: "clicked",
        pointLabel: "Velia Coast",
        layerSamples: [
          {
            layerId: "zone_mask",
            rgbU32: 0x39e58d,
            rgb: [57, 229, 141],
            detailSections: [detailSectionFact("zone", "Zone", "Velia Coast", "hover-zone")],
          },
        ],
      },
      catalog: {
        layers: [{ layerId: "zone_mask", displayOrder: 20 }],
      },
    },
    _map_ui: {
      windowUi: {
        zoneInfo: { tab: "" },
        settings: { normalizeRates: true },
      },
    },
  };
  element._shell = {
    __fishymapLiveSignals: signals,
  };
  element._state = {
    zoneCatalog: [{ zoneRgb: 0x39e58d, name: "Velia Coast", biteTimeMin: 5, biteTimeMax: 7 }],
    zoneLootStatus: "loaded",
    zoneLootRgb: 0x39e58d,
    zoneLootRequestToken: 1,
    zoneLootConditionSelection: {},
    zoneLootSummary: {
      available: true,
      profileLabel: "Calculator defaults",
      groups: [
        {
          slotIdx: 2,
          label: "Rare",
          dropRateSourceKind: "database",
          rawDropRateText: "12.3%",
          rawDropRateTooltip: "Raw group rate",
          normalizedDropRateText: "77.7%",
          normalizedDropRateTooltip: "Normalized group rate",
          catchMethods: ["rod"],
        },
      ],
      speciesRows: [
        {
          slotIdx: 2,
          groupLabel: "Rare",
          label: "Grunt",
          rawDropRateText: "4.5%",
          rawDropRateTooltip: "Raw species rate",
          normalizedDropRateText: "22.2%",
          normalizedDropRateTooltip: "Normalized species rate",
          catchMethods: ["rod"],
        },
      ],
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
  let refreshCount = 0;
  element.refreshZoneLootSummary = () => {
    refreshCount += 1;
    return Promise.resolve();
  };

  element.render();

  assert.match(panelSlot.innerHTML, /77\.7%/);
  assert.match(panelSlot.innerHTML, /22\.2%/);

  signals._map_ui.windowUi.settings.normalizeRates = false;
  element.handleSignalPatch({
    _map_ui: { windowUi: { settings: { normalizeRates: false } } },
  });

  assert.equal(refreshCount, 0);
  assert.match(panelSlot.innerHTML, /12\.3%/);
  assert.match(panelSlot.innerHTML, /4\.5%/);
  assert.doesNotMatch(panelSlot.innerHTML, /77\.7%/);
  assert.doesNotMatch(panelSlot.innerHTML, /22\.2%/);
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
          dropRateTooltip: "DB General group share",
          rawDropRateText: "80%",
          rawDropRateTooltip: "DB General group share",
          normalizedDropRateText: "80%",
          normalizedDropRateTooltip: "DB General group share",
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
          rawDropRateText: "80%",
          rawDropRateTooltip: "DB-backed drop rate",
          normalizedDropRateText: "80%",
          normalizedDropRateTooltip: "DB-backed drop rate",
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
          rawDropRateText: "1%",
          rawDropRateTooltip: "stale parent lineage",
          normalizedDropRateText: "1%",
          normalizedDropRateTooltip: "stale parent lineage",
          catchMethods: ["rod"],
          conditionText: "Default",
          conditionOptions: [
            {
              conditionText: "Default",
              dropRateText: "1%",
              dropRateSourceKind: "database",
              dropRateTooltip: "stale parent lineage",
              rawDropRateText: "1%",
              rawDropRateTooltip: "stale parent lineage",
              normalizedDropRateText: "1%",
              normalizedDropRateTooltip: "stale parent lineage",
              active: true,
              speciesRows: [
                {
                  slotIdx: 2,
                  groupLabel: "Rare",
                  label: "Grunt",
                  dropRateText: "100%",
                  dropRateTooltip: "DB 100% · main group 10990 -> subgroup 10990 · option 1",
                  rawDropRateText: "100%",
                  rawDropRateTooltip: "DB 100% · main group 10990 -> subgroup 10990 · option 1",
                  normalizedDropRateText: "100%",
                  normalizedDropRateTooltip: "DB 100% · main group 10990 -> subgroup 10990 · option 1",
                  catchMethods: ["rod"],
                },
              ],
            },
            {
              conditionText: "Fishing Level Guru 1+",
              dropRateText: "1%",
              dropRateSourceKind: "database",
              dropRateTooltip: "stale parent lineage",
              rawDropRateText: "1%",
              rawDropRateTooltip: "stale parent lineage",
              normalizedDropRateText: "1%",
              normalizedDropRateTooltip: "stale parent lineage",
              active: false,
              speciesRows: [
                {
                  slotIdx: 2,
                  groupLabel: "Rare",
                  label: "Mystical Fish",
                  dropRateText: "0.005%",
                  dropRateTooltip: "DB 0.005% · main group 10990 -> subgroup 11152 · option 0",
                  rawDropRateText: "0.005%",
                  rawDropRateTooltip: "DB 0.005% · main group 10990 -> subgroup 11152 · option 0",
                  normalizedDropRateText: "0.005%",
                  normalizedDropRateTooltip: "DB 0.005% · main group 10990 -> subgroup 11152 · option 0",
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
  assert.match(panelSlot.innerHTML, /main group 10990 -&gt; subgroup 11152 · option 0/);
  assert.match(panelSlot.innerHTML, /Mystical Fish/);
});
installMapTestI18n();
