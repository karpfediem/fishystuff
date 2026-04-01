import test from "node:test";
import assert from "node:assert/strict";

import { createMapInfoPanelController } from "./map-info-panel-live.js";
import { FISHYMAP_ZONE_CATALOG_READY_EVENT } from "./map-zone-catalog-live.js";

const originalHTMLElement = globalThis.HTMLElement;
const originalFetch = globalThis.fetch;
const originalWindow = globalThis.window;

class FakeElement extends EventTarget {
  constructor() {
    super();
    this.hidden = false;
    this.innerHTML = "";
    this.textContent = "";
    this.dataset = {};
    this._queryMap = new Map();
  }

  setQuery(selector, element) {
    this._queryMap.set(selector, element);
  }

  querySelector(selector) {
    return this._queryMap.get(selector) || null;
  }
}

globalThis.HTMLElement = FakeElement;

function createShell() {
  const shell = new FakeElement();
  const title = new FakeElement();
  const titleIcon = new FakeElement();
  const statusIcon = new FakeElement();
  const statusText = new FakeElement();
  const tabs = new FakeElement();
  const panel = new FakeElement();

  shell.setQuery("#fishymap-zone-info-title", title);
  shell.setQuery("#fishymap-zone-info-title-icon", titleIcon);
  shell.setQuery("#fishymap-zone-info-status-icon", statusIcon);
  shell.setQuery("#fishymap-zone-info-status-text", statusText);
  shell.setQuery("#fishymap-zone-info-tabs", tabs);
  shell.setQuery("#fishymap-zone-info-panel", panel);

  return { shell, title, titleIcon, statusIcon, statusText, tabs, panel };
}

function createSignals() {
  return {
    _map_ui: {
      windowUi: {
        zoneInfo: {
          tab: "zone",
        },
      },
    },
    _map_runtime: {
      selection: {
        pointKind: "clicked",
        pointLabel: "Valencia Sea - Depth 5",
        layerSamples: [
          {
            layerId: "zone_mask",
            rgbU32: 0x39e58d,
            rgb: [57, 229, 141],
            detailSections: [
              {
                id: "zone",
                kind: "facts",
                title: "Zone",
                facts: [
                  {
                    key: "zone_name",
                    label: "Zone Name",
                    value: "Valencia Sea - Depth 5",
                    icon: "hover-zone",
                  },
                ],
                targets: [],
              },
            ],
          },
          {
            layerId: "region_groups",
            detailSections: [
              {
                id: "resource-group",
                kind: "facts",
                title: "Resources",
                facts: [
                  {
                    key: "resource_group",
                    label: "Resources",
                    value: "(RG212|Arehaza)",
                    icon: "hover-resources",
                  },
                ],
                targets: [],
              },
            ],
          },
        ],
      },
      catalog: {
        layers: [
          { layerId: "zone_mask", displayOrder: 20 },
          { layerId: "region_groups", displayOrder: 30 },
        ],
      },
    },
  };
}

async function flushAsyncWork() {
  await Promise.resolve();
  await Promise.resolve();
  await new Promise((resolve) => setTimeout(resolve, 0));
}

test("createMapInfoPanelController refreshes zone loot on selection patches through the app-driven hook", async () => {
  const { shell, panel, statusText } = createShell();
  const signals = createSignals();
  const fetchCalls = [];

  globalThis.window = {
    location: {
      hostname: "127.0.0.1",
      protocol: "http:",
    },
  };
  globalThis.fetch = async (url, init) => {
    fetchCalls.push({
      url,
      body: init?.body || "",
    });
    return {
      ok: true,
      async json() {
        return {
          available: true,
          zoneName: "Valencia Sea - Depth 5",
          profileLabel: "Calculator defaults",
          note: "",
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
            },
          ],
          speciesRows: [
            {
              slotIdx: 4,
              groupLabel: "General",
              label: "Sea Eel",
              iconUrl: "/i/sea-eel.png",
              iconGradeTone: "general",
              fillColor: "#eef6ff",
              strokeColor: "#89a8d8",
              textColor: "#1f2937",
              dropRateText: "80%",
              dropRateSourceKind: "database",
              dropRateTooltip: "DB-backed drop rate",
              presenceText: "Presence",
            },
          ],
        };
      },
    };
  };

  const controller = createMapInfoPanelController({
    shell,
    getSignals: () => signals,
    requestAnimationFrameImpl: null,
  });

  shell.dispatchEvent(
    new CustomEvent(FISHYMAP_ZONE_CATALOG_READY_EVENT, {
      detail: {
        zoneCatalog: [
          {
            zoneRgb: 0x39e58d,
            name: "Valencia Sea - Depth 5",
            biteTimeMin: 5,
            biteTimeMax: 7,
          },
        ],
      },
    }),
  );

  shell.dispatchEvent(
    new CustomEvent("fishymap:signal-patched", {
      detail: {
        _map_runtime: {
          selection: signals._map_runtime.selection,
        },
      },
    }),
  );
  await flushAsyncWork();

  assert.equal(fetchCalls.length, 1);
  assert.match(fetchCalls[0].url, /\/api\/v1\/zone_loot_summary$/);
  assert.match(fetchCalls[0].body, /57,229,141/);
  assert.match(panel.innerHTML, /Catch Profile/);
  assert.match(panel.innerHTML, /Sea Eel/);
  assert.match(panel.innerHTML, /80%/);
  assert.match(panel.innerHTML, /Source-backed General group share/);
  assert.match(panel.innerHTML, /fishy-provenance-rail/);
  assert.match(panel.innerHTML, /fishymap-zone-loot-group-rate[\s\S]*fishy-provenance-rail/);
  assert.match(
    panel.innerHTML,
    /fishymap-zone-loot-item-surface[\s\S]*fishy-provenance-rail/,
  );
  assert.match(panel.innerHTML, /data-fishy-provenance-label="Presence"[\s\S]*data-fishy-provenance-label="Rate"/);
  assert.match(panel.innerHTML, /data-fishy-provenance-source="Database"/);
  assert.equal(statusText.textContent, "Clicked point");
});

process.on("exit", () => {
  globalThis.HTMLElement = originalHTMLElement;
  globalThis.fetch = originalFetch;
  globalThis.window = originalWindow;
});
