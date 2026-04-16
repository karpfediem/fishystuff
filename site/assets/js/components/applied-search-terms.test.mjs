import test from "node:test";
import assert from "node:assert/strict";

import { buildAppliedSearchTermsView } from "./applied-search-terms.js";

test("buildAppliedSearchTermsView returns an empty view when there are no active groups", () => {
  const view = buildAppliedSearchTermsView(null);

  assert.equal(view.hasContent, false);
  assert.equal(view.html, "");
  assert.equal(view.renderKey, "null");
});

test("buildAppliedSearchTermsView renders boolean groups with operator badges and term metadata", () => {
  const view = buildAppliedSearchTermsView(
    {
      type: "group",
      path: "root",
      operator: "or",
      children: [
        {
          type: "term",
          path: "root.0",
          key: "fish-filter:favourite",
          label: "Favourite",
          kindLabel: "Filter",
          grade: "favourite",
          contentMarkup: '<span class="font-medium">Favourite</span>',
          removeLabel: "Remove Favourite",
          removeAttributes: {
            "data-fish-filter-term": "favourite",
          },
        },
        {
          type: "group",
          path: "root.1",
          operator: "and",
          children: [
            {
              type: "term",
              path: "root.1.0",
              key: "fish:235",
              label: "Pink Dolphin",
              kindLabel: "Fish",
              grade: "red",
              description: "Prize catch",
              contentMarkup: '<span class="fishy-item-row"><span>Pink Dolphin</span></span>',
              removeAttributes: {
                "data-fish-id": 235,
              },
            },
          ],
        },
      ],
    },
    {
      removeButtonClass: "fishymap-selection-remove",
    },
  );

  assert.equal(view.hasContent, true);
  assert.match(view.html, /class="fishy-applied-expression max-w-full"/);
  assert.match(view.html, /data-expression-node-kind="group"/);
  assert.match(view.html, /data-expression-path="root"/);
  assert.doesNotMatch(view.html, />Applied search</);
  assert.doesNotMatch(view.html, />\s*2 terms\s*</);
  assert.match(view.html, /data-expression-group-path="root"/);
  assert.match(view.html, /data-expression-boundary-index="1"/);
  assert.match(
    view.html,
    /fishy-applied-expression-operator-toggle[\s\S]*data-expression-group-path="root"[\s\S]*data-expression-boundary-index="1"[\s\S]*data-expression-drop-slot-group-path="root"[\s\S]*data-expression-drop-slot-index="1"/,
  );
  assert.doesNotMatch(view.html, /data-expression-group-path="root\.1"/);
  assert.match(view.html, /data-expression-next-operator="and"/);
  assert.doesNotMatch(view.html, /data-expression-next-operator="or"/);
  assert.match(view.html, /data-expression-drop-slot-group-path="root"/);
  assert.match(view.html, /data-expression-drop-slot-group-path="root\.1"/);
  assert.match(view.html, /data-expression-drop-slot-index="0"/);
  assert.match(view.html, /data-expression-drop-slot-index="1"/);
  assert.match(view.html, /data-expression-drop-slot-index="2"/);
  assert.doesNotMatch(
    view.html,
    /fishy-applied-expression-operator-toggle[^>]*data-expression-drop-group-path=/,
  );
  assert.match(view.html, /data-expression-drop-node-path="root\.0"/);
  assert.match(view.html, /data-expression-drop-node-path="root\.1"/);
  assert.match(view.html, /data-expression-drag-path="root\.1"/);
  assert.match(view.html, /title="Drag group"/);
  assert.match(view.html, /fishy-applied-expression-group inline-flex max-w-full flex-wrap items-center gap-2/);
  assert.match(view.html, /join items-stretch max-w-full/);
  assert.match(view.html, /data-expression-remove-path="root\.0"/);
  assert.match(view.html, /data-expression-remove-path="root\.1\.0"/);
  assert.match(view.html, /data-fish-filter-term="favourite"/);
  assert.match(view.html, /data-fish-id="235"/);
  assert.match(view.html, /fishymap-selection-remove/);
  assert.match(view.html, /Prize catch/);
  assert.notEqual(view.renderKey, "null");
});

test("buildAppliedSearchTermsView ignores empty groups", () => {
  const view = buildAppliedSearchTermsView(
    {
      type: "group",
      path: "root",
      operator: "or",
      children: [
        {
          type: "group",
          path: "root.0",
          operator: "and",
          children: [],
        },
        {
          type: "term",
          path: "root.1",
          key: "zone:123",
          label: "Velia Coast",
          kindLabel: "Zone",
          removeAttributes: {
            "data-zone-rgb": 123,
          },
        },
      ],
    },
  );

  assert.equal(view.hasContent, true);
  assert.doesNotMatch(view.html, /data-expression-path="root\.0"/);
  assert.doesNotMatch(view.html, /data-expression-group-path="root"/);
  assert.match(view.html, /data-zone-rgb="123"/);
});
