import test from "node:test";
import assert from "node:assert/strict";

import { renderSearchResults, renderSearchSelection } from "./map-search-panel.js";

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
    searchResultsShell: {
      hidden: false,
    },
    searchResults: {
      dataset: {},
      hidden: false,
      innerHTML: "",
    },
    searchCount: {
      hidden: false,
      textContent: "",
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

test("renderSearchSelection renders the applied search expression tree", () => {
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
        patches: [
          {
            patchId: "2026-03-12",
            label: "New Era",
            startTsUtc: 200,
          },
        ],
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
      search: {
        expression: {
          type: "group",
          operator: "or",
          children: [
            {
              type: "term",
              term: { kind: "fish-filter", term: "favourite" },
              negated: true,
            },
            {
              type: "group",
              operator: "and",
              negated: true,
              children: [
                {
                  type: "term",
                  term: { kind: "fish", fishId: 235 },
                },
                {
                  type: "term",
                  term: { kind: "patch-bound", bound: "to", patchId: "2026-03-12" },
                },
                {
                  type: "term",
                  term: { kind: "zone", zoneRgb: 123456 },
                },
                {
                  type: "term",
                  term: { kind: "semantic", layerId: "regions", fieldId: 77 },
                },
              ],
            },
          ],
        },
      },
      filters: {
        fishIds: [235],
        fishFilterTerms: ["favourite"],
        semanticFieldIdsByLayer: {
          zone_mask: [123456],
          regions: [77],
        },
        patchId: null,
        fromPatchId: null,
        toPatchId: "2026-03-12",
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
  assert.doesNotMatch(elements.searchSelection.innerHTML, />Applied search</);
  assert.doesNotMatch(elements.searchSelection.innerHTML, />\s*4 terms\s*</);
  assert.match(elements.searchSelection.innerHTML, /data-expression-node-kind="group"/);
  assert.match(elements.searchSelection.innerHTML, /data-expression-negate-path="root"/);
  assert.match(elements.searchSelection.innerHTML, /data-expression-negate-path="root\.0"/);
  assert.match(elements.searchSelection.innerHTML, /data-expression-negate-path="root\.1"/);
  assert.match(elements.searchSelection.innerHTML, /data-expression-group-path="root"/);
  assert.match(elements.searchSelection.innerHTML, /data-expression-path="root\.1\.0"/);
  assert.match(elements.searchSelection.innerHTML, /data-expression-path="root\.1\.1"/);
  assert.match(elements.searchSelection.innerHTML, /data-expression-path="root\.1\.2"/);
  assert.match(elements.searchSelection.innerHTML, /data-expression-path="root\.1\.3"/);
  assert.match(elements.searchSelection.innerHTML, /data-expression-operator="and"/);
  assert.match(elements.searchSelection.innerHTML, /join items-stretch max-w-full/);
  assert.match(elements.searchSelection.innerHTML, /data-fish-filter-term="favourite"/);
  assert.match(elements.searchSelection.innerHTML, /data-fish-id="235"/);
  assert.match(elements.searchSelection.innerHTML, /data-expression-patch-toggle-path="root\.1\.1"/);
  assert.doesNotMatch(elements.searchSelection.innerHTML, /data-expression-negate-path="root\.1\.1"/);
  assert.match(elements.searchSelection.innerHTML, /data-patch-bound="to"/);
  assert.match(elements.searchSelection.innerHTML, /data-patch-id="2026-03-12"/);
  assert.match(elements.searchSelection.innerHTML, /2026-03-12/);
  assert.match(elements.searchSelection.innerHTML, /data-zone-rgb="123456"/);
  assert.match(elements.searchSelection.innerHTML, /data-semantic-layer-id="regions"/);
  assert.match(elements.searchSelection.innerHTML, />Date</);
  assert.match(elements.searchSelection.innerHTML, />Before</);
  assert.match(elements.searchSelection.innerHTML, />Fish</);
  assert.match(elements.searchSelection.innerHTML, />Zone</);
  assert.match(elements.searchSelection.innerHTML, />Region</);
  assert.match(elements.searchSelection.innerHTML, /Open-water region/);
  assert.doesNotMatch(elements.searchSelection.innerHTML, />Filters</);
  assert.doesNotMatch(elements.searchSelection.innerHTML, />Zones</);
});

test("renderSearchSelection renders an unresolved date term with an inline patch selector", () => {
  const elements = createRenderElements();

  renderSearchSelection(
    elements,
    {
      state: {
        catalog: {
          patches: [
            {
              patchId: "2026-03-12",
              label: "New Era",
              startTsUtc: 200,
            },
          ],
          semanticTerms: [],
        },
      },
      inputState: {
        search: {
          expression: {
            type: "group",
            operator: "or",
            children: [{ type: "term", term: { kind: "patch-bound", bound: "from" } }],
          },
        },
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
      escapeHtml,
    },
  );

  assert.match(elements.searchSelection.innerHTML, /fishy-searchable-dropdown/);
  assert.match(elements.searchSelection.innerHTML, /custom-option-mode="iso-date"/);
  assert.match(elements.searchSelection.innerHTML, /data-expression-patch-select-path="root\.0"/);
  assert.match(elements.searchSelection.innerHTML, />Choose date</);
  assert.match(elements.searchSelection.innerHTML, /data-expression-patch-toggle-path="root\.0"/);
  assert.doesNotMatch(elements.searchSelection.innerHTML, /data-expression-negate-path="root\.0"/);
  assert.doesNotMatch(elements.searchSelection.innerHTML, /data-patch-id=""/);
});

test("renderSearchResults renders unresolved date prompt rows without a concrete patch id", () => {
  const elements = createRenderElements();
  const stateBundle = {
    inputState: {
      filters: {
        searchText: "",
      },
    },
  };

  renderSearchResults(
    elements,
    [
      {
        kind: "patch-bound",
        bound: "to",
        label: "Before",
        description: "Add a before term, then choose the patch on the term itself.",
      },
    ],
    stateBundle,
    {
      setBooleanProperty: (element, property, value) => {
        element[property] = Boolean(value);
      },
      setTextContent: (element, value) => {
        element.textContent = String(value ?? "");
      },
      escapeHtml,
    },
  );

  assert.equal(elements.searchResultsShell.hidden, false);
  assert.match(elements.searchResults.innerHTML, /data-patch-bound="to"/);
  assert.match(elements.searchResults.innerHTML, />Before:/);
  assert.doesNotMatch(elements.searchResults.innerHTML, /data-patch-id=/);
  assert.match(elements.searchResults.innerHTML, /choose the patch on the term itself/i);
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
        search: {
          expression: {
            type: "group",
            operator: "or",
            children: [],
          },
        },
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
