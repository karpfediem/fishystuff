import test from "node:test";
import assert from "node:assert/strict";

import { renderBookmarkManager } from "./map-bookmark-panel.js";
import {
  bookmarkCurrentPointSubtitle,
  bookmarkDisplayLabel,
  buildBookmarkOverviewRows,
} from "./map-bookmark-state.js";

function mockElement() {
  return {
    dataset: {},
    innerHTML: "",
    textContent: "",
    disabled: false,
    hidden: false,
  };
}

function setBooleanProperty(element, propertyName, value) {
  element[propertyName] = Boolean(value);
}

function setTextContent(element, text) {
  element.textContent = String(text ?? "");
}

function setMarkup(element, renderKey, markup) {
  element.dataset.renderKey = String(renderKey ?? "");
  element.innerHTML = String(markup ?? "");
}

test("renderBookmarkManager shows a subtitle when the saved title differs from the current point label", () => {
  const elements = {
    shell: { dataset: {} },
    bookmarkPlace: mockElement(),
    bookmarkPlaceLabel: mockElement(),
    bookmarkCopySelected: mockElement(),
    bookmarkExport: mockElement(),
    bookmarkSelectAll: mockElement(),
    bookmarkDeleteSelected: mockElement(),
    bookmarkClearSelection: mockElement(),
    bookmarkClearSelectionLabel: mockElement(),
    bookmarkCancel: mockElement(),
    bookmarksList: mockElement(),
  };

  const stateBundle = {
    state: {
      ready: true,
      view: { viewMode: "2d" },
      selection: {},
      catalog: { layers: [{ layerId: "zone_mask", displayOrder: 10 }] },
    },
    zoneCatalog: [{ zoneRgb: 0x3c963c, name: "Margoria South" }],
  };

  renderBookmarkManager(
    elements,
    stateBundle,
    [
      {
        id: "bookmark-a",
        label: "Margoria (RG218)",
        worldX: 12,
        worldZ: 34,
        layerSamples: [
          {
            layerId: "zone_mask",
            rgbU32: 0x3c963c,
            rgb: [60, 150, 60],
            detailSections: [],
          },
          {
            layerId: "region_groups",
            detailSections: [
              {
                id: "resources",
                kind: "facts",
                title: "Resources",
                facts: [{ key: "resource_group", label: "Resources", value: "Margoria (RG218)" }],
                targets: [],
              },
            ],
          },
        ],
      },
    ],
    {
      placing: false,
      selectedIds: [],
    },
    {
      resolveDisplayBookmarks: (_stateBundle, bookmarks) => bookmarks,
      normalizeSelectedBookmarkIds: (_bookmarks, selectedIds) => selectedIds,
      setBooleanProperty,
      setTextContent,
      setMarkup,
      buildBookmarkOverviewRows,
      bookmarkDisplayLabel,
      bookmarkCurrentPointSubtitle,
    },
  );

  assert.match(elements.bookmarksList.innerHTML, /fishymap-bookmark-subtitle/);
  assert.match(elements.bookmarksList.innerHTML, /Margoria South/);
});
