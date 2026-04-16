import test from "node:test";
import assert from "node:assert/strict";

import { renderSearchSelection } from "./map-search-panel.js";

function createRenderElements() {
  return {
    zoneCatalog: [
      {
        zoneRgb: 123456,
        name: "Velia Coast",
        rgbKey: "velia_coast",
        r: 1,
        g: 2,
        b: 3,
      },
    ],
    searchSelection: {
      dataset: {},
      hidden: false,
      innerHTML: "",
    },
    searchSelectionShell: {
      hidden: false,
    },
    searchWindow: {
      dataset: {},
    },
  };
}

function escapeHtml(value) {
  return String(value ?? "").replace(
    /[&<>"']/g,
    (char) =>
      ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;",
      })[char] || char,
  );
}

test("renderSearchSelection groups selected search terms into applied-term sections", () => {
  const elements = createRenderElements();
  const fishLookup = new Map([
    [
      235,
      {
        fishId: 235,
        name: "Pink Dolphin",
        grade: "red",
        itemId: 9001,
      },
    ],
  ]);
  const stateBundle = {
    state: {
      catalog: {
        semanticTerms: [
          {
            layerId: "regions",
            fieldId: 77,
            label: "Serendia Sea",
            description: "Open-water region",
            layerName: "Region",
          },
        ],
      },
    },
    inputState: {
      filters: {
        fishIds: [235],
        fishFilterTerms: ["favourite", "red"],
        semanticFieldIdsByLayer: {
          zone_mask: [123456],
          regions: [77],
        },
      },
    },
  };

  renderSearchSelection(elements, stateBundle, fishLookup, {
    resolveSelectedFishIds: (bundle) => bundle.inputState.filters.fishIds,
    resolveSelectedFishFilterTerms: (bundle) => bundle.inputState.filters.fishFilterTerms,
    resolveSelectedSemanticFieldIdsByLayer: (bundle) => bundle.inputState.filters.semanticFieldIdsByLayer,
    resolveSelectedZoneRgbs: (bundle) => bundle.inputState.filters.semanticFieldIdsByLayer.zone_mask,
    buildSemanticTermLookup: (bundle) =>
      new Map(
        bundle.state.catalog.semanticTerms.map((term) => [
          `${term.layerId}:${term.fieldId}`,
          term,
        ]),
      ),
    escapeHtml,
    fishFilterTermIconMarkup: (term) => `<span class="icon">${escapeHtml(term)}</span>`,
    fishIdentityMarkup: (fish) => `<span class="fishy-item-row">${escapeHtml(fish.name)}</span>`,
    zoneIdentityMarkup: (zone) => `<span>${escapeHtml(zone.name)}</span>`,
    semanticIdentityMarkup: (label) => `<span>${escapeHtml(label)}</span>`,
    resolveFishGrade: (fish) => String(fish?.grade || "unknown"),
    formatZone: (zoneRgb) => `#${Number(zoneRgb).toString(16).padStart(6, "0")}`,
    fishFilterTermMetadata: {
      favourite: { label: "Favourite" },
      red: { label: "Red" },
    },
  });

  assert.equal(elements.searchSelection.hidden, false);
  assert.equal(elements.searchSelectionShell.hidden, false);
  assert.equal(elements.searchWindow.dataset.hasSelection, "true");
  assert.match(elements.searchSelection.innerHTML, />Filters</);
  assert.match(elements.searchSelection.innerHTML, />Fish</);
  assert.match(elements.searchSelection.innerHTML, />Zones</);
  assert.match(elements.searchSelection.innerHTML, />Map Terms</);
  assert.match(elements.searchSelection.innerHTML, /data-fish-filter-term="favourite"/);
  assert.match(elements.searchSelection.innerHTML, /data-fish-id="235"/);
  assert.match(elements.searchSelection.innerHTML, /data-zone-rgb="123456"/);
  assert.match(elements.searchSelection.innerHTML, /data-semantic-layer-id="regions"/);
  assert.match(elements.searchSelection.innerHTML, /Open-water region/);
});

test("renderSearchSelection hides the selection shell when no terms are applied", () => {
  const elements = createRenderElements();

  renderSearchSelection(
    elements,
    {
      state: {
        catalog: {
          semanticTerms: [],
        },
      },
      inputState: {
        filters: {
          fishIds: [],
          fishFilterTerms: [],
          semanticFieldIdsByLayer: {},
        },
      },
    },
    new Map(),
    {
      resolveSelectedFishIds: () => [],
      resolveSelectedFishFilterTerms: () => [],
      resolveSelectedSemanticFieldIdsByLayer: () => ({}),
      resolveSelectedZoneRgbs: () => [],
      buildSemanticTermLookup: () => new Map(),
    },
  );

  assert.equal(elements.searchSelection.hidden, true);
  assert.equal(elements.searchSelectionShell.hidden, true);
  assert.equal(elements.searchSelection.innerHTML, "");
  assert.equal(elements.searchWindow.dataset.hasSelection, "false");
});
