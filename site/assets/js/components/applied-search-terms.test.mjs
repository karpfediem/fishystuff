import test from "node:test";
import assert from "node:assert/strict";

import { buildAppliedSearchTermsView } from "./applied-search-terms.js";

test("buildAppliedSearchTermsView returns an empty view when there are no active groups", () => {
  const view = buildAppliedSearchTermsView([]);

  assert.equal(view.hasContent, false);
  assert.equal(view.html, "");
  assert.equal(view.renderKey, "[]");
});

test("buildAppliedSearchTermsView renders grouped applied terms with grade and remove attributes", () => {
  const view = buildAppliedSearchTermsView(
    [
      {
        key: "filters",
        label: "Filters",
        items: [
          {
            key: "fish-filter:favourite",
            label: "Favourite",
            grade: "favourite",
            contentMarkup: '<span class="font-medium">Favourite</span>',
            removeLabel: "Remove Favourite",
            removeAttributes: {
              "data-fish-filter-term": "favourite",
            },
          },
        ],
      },
      {
        key: "fish",
        label: "Fish",
        items: [
          {
            key: "fish:235",
            label: "Pink Dolphin",
            grade: "red",
            kindLabel: "Favourite",
            description: "Prize catch",
            contentMarkup: '<span class="fishy-item-row"><span>Pink Dolphin</span></span>',
            removeAttributes: {
              "data-fish-id": 235,
            },
          },
        ],
      },
    ],
    {
      removeButtonClass: "fishymap-selection-remove",
    },
  );

  assert.equal(view.hasContent, true);
  assert.match(view.html, /class="fishy-applied-terms"/);
  assert.match(view.html, />Filters</);
  assert.match(view.html, />Fish</);
  assert.match(view.html, /data-grade="favourite"/);
  assert.match(view.html, /data-grade="red"/);
  assert.match(view.html, /data-fish-filter-term="favourite"/);
  assert.match(view.html, /data-fish-id="235"/);
  assert.match(view.html, /fishymap-selection-remove/);
  assert.match(view.html, /Prize catch/);
  assert.notEqual(view.renderKey, "[]");
});
