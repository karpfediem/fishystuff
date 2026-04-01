import test from "node:test";
import assert from "node:assert/strict";

import {
  addSelectedSearchTerm,
  buildSearchSelectionStatePatch,
  layerSupportsAttachmentClipMode,
  layerSupportsSearchTerm,
  normalizeFishFilterTerms,
  normalizeSelectedSearchTerms,
  projectSelectedSearchTermsToBridgedFilters,
  resolveSelectedSearchTerms,
} from "./map-search-contract.js";

test("normalizeSelectedSearchTerms canonicalizes aliases and deduplicates term kinds", () => {
  assert.deepEqual(
    normalizeSelectedSearchTerms([
      { kind: "fish-filter", term: "favorite" },
      { kind: "fish-filter", term: "favourites" },
      { kind: "fish", fishId: "912" },
      { kind: "semantic", layerId: "zone_mask", fieldId: 123 },
      { kind: "zone", zoneRgb: 123 },
      { kind: "semantic", layerId: "regions", fieldId: 22 },
    ]),
    [
      { kind: "fish-filter", term: "favourite" },
      { kind: "fish", fishId: 912 },
      { kind: "zone", zoneRgb: 123 },
      { kind: "semantic", layerId: "regions", fieldId: 22 },
    ],
  );
});

test("resolveSelectedSearchTerms falls back to legacy bridged filters", () => {
  assert.deepEqual(
    resolveSelectedSearchTerms(undefined, {
      fishIds: [912],
      zoneRgbs: [123],
      fishFilterTerms: ["missing"],
      semanticFieldIdsByLayer: {
        regions: [22],
      },
    }),
    [
      { kind: "fish-filter", term: "missing" },
      { kind: "fish", fishId: 912 },
      { kind: "zone", zoneRgb: 123 },
      { kind: "semantic", layerId: "regions", fieldId: 22 },
    ],
  );
});

test("projectSelectedSearchTermsToBridgedFilters derives explicit runtime filters", () => {
  assert.deepEqual(
    projectSelectedSearchTermsToBridgedFilters([
      { kind: "fish-filter", term: "missing" },
      { kind: "fish", fishId: 912 },
      { kind: "zone", zoneRgb: 123 },
      { kind: "semantic", layerId: "regions", fieldId: 22 },
    ]),
    {
      fishIds: [912],
      zoneRgbs: [123],
      semanticFieldIdsByLayer: {
        regions: [22],
        zone_mask: [123],
      },
      fishFilterTerms: ["missing"],
    },
  );
});

test("buildSearchSelectionStatePatch keeps selected terms page-owned and projects bridged filters", () => {
  assert.deepEqual(
    buildSearchSelectionStatePatch(
      [
        { kind: "fish", fishId: 912 },
        { kind: "zone", zoneRgb: 123 },
      ],
      { query: "", open: false },
    ),
    {
      _map_ui: {
        search: {
          selectedTerms: [
            { kind: "fish", fishId: 912 },
            { kind: "zone", zoneRgb: 123 },
          ],
          query: "",
          open: false,
        },
      },
      _map_bridged: {
        filters: {
          fishIds: [912],
          zoneRgbs: [123],
          semanticFieldIdsByLayer: { zone_mask: [123] },
          fishFilterTerms: [],
        },
      },
    },
  );
});

test("addSelectedSearchTerm keeps insertion order with deduped canonical keys", () => {
  assert.deepEqual(
    addSelectedSearchTerm([{ kind: "fish-filter", term: "missing" }], {
      kind: "fish-filter",
      term: "uncaught",
    }),
    [{ kind: "fish-filter", term: "missing" }],
  );
});

test("search layer support documents direct term and clip capabilities", () => {
  assert.equal(layerSupportsSearchTerm("zone_mask", "fish"), true);
  assert.equal(layerSupportsSearchTerm("regions", "fish"), false);
  assert.equal(layerSupportsAttachmentClipMode("regions", "mask-sample"), true);
  assert.equal(layerSupportsAttachmentClipMode("bookmarks", "mask-sample"), false);
});

test("normalizeFishFilterTerms preserves canonical term order", () => {
  assert.deepEqual(normalizeFishFilterTerms(["favorite", "missing"]), [
    "favourite",
    "missing",
  ]);
});
