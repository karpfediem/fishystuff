import test from "node:test";
import assert from "node:assert/strict";

import {
  bookmarkDisplayLabel,
  buildRuntimeBookmarkDetailsPatch,
  buildBookmarkOverviewRows,
  buildBookmarkPanelStateBundle,
  createBookmarkFromSelection,
  moveBookmarkBefore,
  normalizeBookmarks,
  normalizeSelectedBookmarkIds,
  patchTouchesBookmarkSignals,
  renameBookmark,
  selectionBookmarkKey,
} from "./map-bookmark-state.js";

test("normalizeBookmarks keeps only valid bookmark entries", () => {
  assert.deepEqual(
    normalizeBookmarks([
      { id: "a", label: "Alpha", worldX: 12.4, worldZ: 88.9 },
      { id: "b", worldX: 1, worldZ: 2, zoneRgb: 123 },
      { id: "", worldX: 9, worldZ: 9 },
    ]),
    [
      { id: "a", label: "Alpha", worldX: 12, worldZ: 89 },
      { id: "b", worldX: 1, worldZ: 2, zoneRgb: 123 },
    ],
  );
});

test("normalizeSelectedBookmarkIds filters missing bookmark ids", () => {
  assert.deepEqual(
    normalizeSelectedBookmarkIds(
      [{ id: "a", worldX: 1, worldZ: 2 }],
      ["a", "missing", "a"],
    ),
    ["a"],
  );
});

test("createBookmarkFromSelection builds a bookmark from runtime selection", () => {
  const bookmark = createBookmarkFromSelection(
    {
      worldX: 123.2,
      worldZ: 456.7,
      pointLabel: "Cron Castle",
      layerSamples: [{ layerId: "zone_mask", rgbU32: 65535 }],
    },
    [],
  );
  assert.equal(typeof bookmark.id, "string");
  assert.equal(bookmark.label, "Cron Castle");
  assert.equal(bookmark.worldX, 123);
  assert.equal(bookmark.worldZ, 457);
  assert.equal(bookmark.zoneRgb, 65535);
});

test("createBookmarkFromSelection prefers the zone overview label over point label", () => {
  const bookmark = createBookmarkFromSelection(
    {
      worldX: 123.2,
      worldZ: 456.7,
      pointLabel: "Margoria (RG218)",
      layerSamples: [
        {
          layerId: "zone_mask",
          rgbU32: 0x39e58d,
          detailSections: [
            {
              id: "zone",
              kind: "facts",
              title: "Zone",
              facts: [{ key: "zone", label: "Zone", value: "Valencia Sea - Depth 5" }],
              targets: [],
            },
          ],
        },
      ],
    },
    [],
  );

  assert.equal(bookmark.label, "Valencia Sea - Depth 5");
});

test("createBookmarkFromSelection can resolve the zone name from zone catalog context", () => {
  const bookmark = createBookmarkFromSelection(
    {
      worldX: 123.2,
      worldZ: 456.7,
      pointLabel: "Margoria (RG218)",
      layerSamples: [
        {
          layerId: "zone_mask",
          rgbU32: 0x3c963c,
          rgb: [60, 150, 60],
        },
      ],
    },
    [],
    {
      zoneCatalog: [
        {
          zoneRgb: 0x3c963c,
          name: "Margoria South",
        },
      ],
    },
  );

  assert.equal(bookmark.label, "Margoria South");
});

test("bookmark helpers expose display and ordering utilities", () => {
  const bookmarks = [
    { id: "a", label: "Alpha", worldX: 1, worldZ: 2 },
    { id: "b", label: "Beta", worldX: 3, worldZ: 4 },
    { id: "c", worldX: 5, worldZ: 6 },
  ];
  assert.equal(bookmarkDisplayLabel(bookmarks[2], 2), "Bookmark 3");
  assert.equal(buildBookmarkOverviewRows(bookmarks[0])[0].value, "Alpha");
  assert.deepEqual(
    moveBookmarkBefore(bookmarks, "c", "a", "before").map((bookmark) => bookmark.id),
    ["c", "a", "b"],
  );
  assert.equal(renameBookmark(bookmarks, "b", "Renamed")[1].label, "Renamed");
});

test("buildBookmarkOverviewRows keeps semantic facts and drops legacy world coordinates", () => {
  const rows = buildBookmarkOverviewRows(
    {
      id: "probe",
      label: "",
      worldX: 123,
      worldZ: 456,
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
              facts: [{ key: "zone", label: "Zone", value: "Valencia Sea - Depth 5", icon: "hover-zone" }],
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
              facts: [{ key: "resource_group", label: "Resources", value: "(RG212|Arehaza)", icon: "hover-resources" }],
              targets: [],
            },
          ],
        },
        {
          layerId: "regions",
          detailSections: [
            {
              id: "origin-region",
              kind: "facts",
              title: "Origin",
              facts: [{ key: "origin_region", label: "Origin", value: "(R430|Hakoven Islands)", icon: "trade-origin" }],
              targets: [],
            },
          ],
        },
      ],
    },
    0,
  );

  assert.deepEqual(
    rows.map((row) => [row.icon, row.label, row.value]),
    [
      ["bookmark", "Bookmark", "Valencia Sea - Depth 5"],
      ["hover-resources", "Resources", "(RG212|Arehaza)"],
      ["trade-origin", "Origin", "(R430|Hakoven Islands)"],
    ],
  );
});

test("buildRuntimeBookmarkDetailsPatch enriches imported bookmarks with runtime facts", () => {
  const patch = buildRuntimeBookmarkDetailsPatch(
    [{ id: "bookmark-a", label: "Imported", worldX: 12, worldZ: 34 }],
    [
      {
        id: "bookmark-a",
        label: "Imported",
        worldX: 12,
        worldZ: 34,
        zoneRgb: 0x39e58d,
        layerSamples: [
          {
            layerId: "zone_mask",
            detailSections: [
              {
                id: "zone",
                kind: "facts",
                title: "Zone",
                facts: [{ key: "zone", label: "Zone", value: "Valencia Sea - Depth 5" }],
                targets: [],
              },
            ],
          },
        ],
      },
    ],
  );

  assert.deepEqual(patch, {
    _map_bookmarks: {
      entries: [
        {
          id: "bookmark-a",
          label: "Imported",
          worldX: 12,
          worldZ: 34,
          zoneRgb: 0x39e58d,
          layerSamples: [
            {
              layerId: "zone_mask",
              detailSections: [
                {
                  id: "zone",
                  kind: "facts",
                  title: "Zone",
                  facts: [{ key: "zone", label: "Zone", value: "Valencia Sea - Depth 5" }],
                  targets: [],
                },
              ],
            },
          ],
        },
      ],
    },
  });
});

test("buildBookmarkPanelStateBundle derives bookmark ui from canonical signals", () => {
  const bundle = buildBookmarkPanelStateBundle({
    _map_runtime: {
      ready: true,
      view: { viewMode: "2d" },
      selection: { worldX: 1, worldZ: 2 },
    },
    _map_bookmarks: {
      entries: [{ id: "a", worldX: 5, worldZ: 6 }],
    },
    _map_ui: {
      bookmarks: {
        placing: true,
        selectedIds: ["a", "missing"],
      },
    },
  });
  assert.equal(bundle.state.ready, true);
  assert.equal(bundle.bookmarkUi.placing, true);
  assert.deepEqual(bundle.bookmarkUi.selectedIds, ["a"]);
});

test("bookmark signal helpers stay scoped to bookmark-related branches", () => {
  assert.equal(
    patchTouchesBookmarkSignals({ _map_runtime: { selection: { worldX: 1 } } }),
    true,
  );
  assert.equal(
    patchTouchesBookmarkSignals({ _map_runtime: { statuses: { layersStatus: "ok" } } }),
    false,
  );
  assert.notEqual(
    selectionBookmarkKey({ worldX: 1, worldZ: 2, pointKind: "clicked" }),
    "",
  );
});
