import test from "node:test";
import assert from "node:assert/strict";

globalThis.__fishystuffLoaderAutoStart = false;
const {
  buildBookmarkOverviewRows,
  buildDefaultWindowUiStateSerialized,
  buildHoverOverviewRows,
  buildMapUiResetMountOptions,
  buildSearchMatches,
  computeDragAutoScrollDelta,
  createBookmarkFromPlacement,
  mergeImportedBookmarks,
  moveBookmarkBefore,
  normalizeZoneCatalog,
  normalizeBookmarks,
  normalizeBookmarkCoordinate,
  normalizeWindowUiState,
  parseZoneRgbSearch,
  parseImportedBookmarks,
  parseWindowUiState,
  renameBookmark,
  resolveDisplayBookmarks,
  renderSearchSelection,
  serializeBookmarksForExport,
  serializeWindowUiState,
} = await import("./loader.js");
delete globalThis.__fishystuffLoaderAutoStart;

const TEST_ZONE_CATALOG = normalizeZoneCatalog([
  {
    r: 193,
    g: 127,
    b: 127,
    name: "Cron Islands - Depth 2",
    confirmed: 1,
    order: 21,
  },
  {
    r: 60,
    g: 150,
    b: 60,
    name: "Serendia - Terrain",
    confirmed: 1,
    order: 3,
  },
]);

function buildStateBundle(selectedFishIds = []) {
  return {
    state: {
      catalog: {
        layers: [],
        fish: [
          {
            fishId: 912,
            itemId: 3012,
            encyclopediaId: 4012,
            name: "Cron Dart",
            grade: "Rare",
            isPrize: false,
          },
          {
            fishId: 77,
            itemId: 3077,
            encyclopediaId: 4077,
            name: "Serendia Carp",
            grade: "General",
            isPrize: false,
          },
        ],
      },
    },
    inputState: {
      filters: {
        fishIds: selectedFishIds,
        zoneRgbs: [],
      },
    },
  };
}

function buildHoverStateBundle() {
  return {
    state: {
      catalog: {
        layers: [
          {
            layerId: "regions",
            name: "Regions",
            visible: true,
            opacity: 1,
            opacityDefault: 1,
            displayOrder: 40,
            kind: "vector-geojson",
          },
          {
            layerId: "region_groups",
            name: "Region Groups",
            visible: true,
            opacity: 1,
            opacityDefault: 1,
            displayOrder: 30,
            kind: "vector-geojson",
          },
          {
            layerId: "zone_mask",
            name: "Zone Mask",
            visible: true,
            opacity: 1,
            opacityDefault: 1,
            displayOrder: 20,
            kind: "tiled-raster",
          },
        ],
        fish: [],
      },
    },
    inputState: {
      filters: {
        layerIdsOrdered: ["regions", "region_groups", "zone_mask"],
      },
    },
  };
}

test("parseZoneRgbSearch handles hex, byte triplets, and normalized triplets", () => {
  assert.equal(parseZoneRgbSearch("193,127,127"), 0xc17f7f);
  assert.equal(parseZoneRgbSearch("193 127 127"), 0xc17f7f);
  assert.equal(parseZoneRgbSearch("1,0,0"), 0x010000);
  assert.equal(parseZoneRgbSearch("#c17f7f"), 0xc17f7f);
  assert.equal(parseZoneRgbSearch("0xc17f7f"), 0xc17f7f);
  assert.equal(parseZoneRgbSearch("rgb(0.75686276, 0.49803924, 0.49803924)"), 0xc17f7f);
  assert.equal(parseZoneRgbSearch("Cron Islands"), null);
});

test("buildSearchMatches returns zone hits for zone names and normalized RGB", () => {
  const stateBundle = buildStateBundle();

  const zoneByName = buildSearchMatches(stateBundle, "Cron Islands", TEST_ZONE_CATALOG);
  assert.equal(zoneByName[0]?.kind, "zone");
  assert.equal(zoneByName[0]?.zoneRgb, 0xc17f7f);

  const zoneByRgb = buildSearchMatches(
    stateBundle,
    "0.75686276 0.49803924 0.49803924",
    TEST_ZONE_CATALOG,
  );
  assert.equal(zoneByRgb[0]?.kind, "zone");
  assert.equal(zoneByRgb[0]?.zoneRgb, 0xc17f7f);
});

test("buildSearchMatches keeps fish search working and filters already selected fish", () => {
  const matches = buildSearchMatches(buildStateBundle([912]), "Cron", TEST_ZONE_CATALOG);

  assert.equal(
    matches.some((match) => match.kind === "fish" && match.fishId === 912),
    false,
  );
  assert.equal(
    matches.some((match) => match.kind === "zone" && match.zoneRgb === 0xc17f7f),
    true,
  );

  const fishMatches = buildSearchMatches(buildStateBundle(), "Serendia Carp", TEST_ZONE_CATALOG);
  assert.equal(fishMatches[0]?.kind, "fish");
  assert.equal(fishMatches[0]?.fishId, 77);
});

test("buildHoverOverviewRows renders supported hover layers from bottom to top", () => {
  assert.deepEqual(
    buildHoverOverviewRows(
      {
        zoneName: "Demi River",
        layerSamples: [
          {
            layerId: "region_groups",
            regionName: "Tarif",
          },
          {
            layerId: "regions",
            regionName: "Tarif",
          },
        ],
      },
      buildHoverStateBundle(),
    ),
    [
      {
        layerId: "zone_mask",
        icon: "hover-zone",
        label: "Zone",
        value: "Demi River",
      },
      {
        layerId: "region_groups",
        icon: "hover-resources",
        label: "Resources",
        value: "Tarif",
      },
      {
        layerId: "regions",
        icon: "hover-origin",
        label: "Origin",
        value: "Tarif",
      },
    ],
  );
});

test("buildBookmarkOverviewRows mirrors the hover row style without duplicating the zone", () => {
  assert.deepEqual(
    buildBookmarkOverviewRows(
      {
        label: "Tarif hotspot",
        zoneName: "Tarif",
        resourceName: "Tarif",
        originName: "Tarif",
      },
      0,
    ),
    [
      {
        icon: "bookmarks",
        label: "Bookmark",
        value: "Tarif hotspot",
      },
      {
        icon: "hover-zone",
        label: "Zone",
        value: "Tarif",
      },
      {
        icon: "hover-resources",
        label: "Resources",
        value: "Tarif",
      },
      {
        icon: "hover-origin",
        label: "Origin",
        value: "Tarif",
      },
    ],
  );

  assert.deepEqual(
    buildBookmarkOverviewRows(
      {
        label: "Tarif",
        zoneName: "Tarif",
        resourceName: "Tarif",
        originName: "Tarif",
      },
      0,
    ),
    [
      {
        icon: "bookmarks",
        label: "Bookmark",
        value: "Tarif",
      },
      {
        icon: "hover-resources",
        label: "Resources",
        value: "Tarif",
      },
      {
        icon: "hover-origin",
        label: "Origin",
        value: "Tarif",
      },
    ],
  );
});

test("buildSearchMatches filters already selected zones from zone results", () => {
  const stateBundle = buildStateBundle();
  stateBundle.inputState.filters.zoneRgbs = [0xc17f7f];

  const matches = buildSearchMatches(stateBundle, "Cron Islands", TEST_ZONE_CATALOG);

  assert.equal(
    matches.some((match) => match.kind === "zone" && match.zoneRgb === 0xc17f7f),
    false,
  );
});

test("renderSearchSelection restores visible chips after the search window is re-shown", () => {
  const searchSelection = {
    dataset: {},
    hidden: true,
    innerHTML: "",
  };
  const searchSelectionShell = {
    hidden: true,
  };
  const searchWindow = {
    dataset: {},
  };
  const elements = {
    searchSelection,
    searchSelectionShell,
    searchWindow,
    zoneCatalog: TEST_ZONE_CATALOG,
  };
  const stateBundle = buildStateBundle();
  stateBundle.inputState.filters.zoneRgbs = [0xc17f7f];
  const fishLookup = new Map();

  renderSearchSelection(elements, stateBundle, fishLookup);
  assert.equal(searchSelection.hidden, false);
  assert.equal(searchSelectionShell.hidden, false);

  searchSelection.hidden = true;
  searchSelectionShell.hidden = true;

  renderSearchSelection(elements, stateBundle, fishLookup);
  assert.equal(searchSelection.hidden, false);
  assert.equal(searchSelectionShell.hidden, false);
});

test("parseWindowUiState falls back to defaults for invalid persisted state", () => {
  assert.deepEqual(parseWindowUiState("not json"), normalizeWindowUiState(null));
  assert.deepEqual(parseWindowUiState(""), normalizeWindowUiState(null));
});

test("serializeWindowUiState normalizes persisted window geometry and flags", () => {
  const serialized = serializeWindowUiState({
    search: { open: false, collapsed: "yes", x: 42.8, y: "13" },
    settings: { open: true, collapsed: false, x: null, y: null },
    zoneInfo: { open: true, collapsed: false, x: undefined, y: 5.2 },
    layers: { open: false, collapsed: 0, x: "bad", y: 99.9 },
    bookmarks: { open: true, collapsed: true, x: "14", y: 7.8 },
  });

  assert.deepEqual(JSON.parse(serialized), {
    search: { open: false, collapsed: false, x: 43, y: 13 },
    settings: { open: true, collapsed: false, x: null, y: null },
    zoneInfo: { open: true, collapsed: false, x: null, y: 5 },
    layers: { open: false, collapsed: false, x: null, y: 100 },
    bookmarks: { open: true, collapsed: true, x: 14, y: 8 },
  });
});

test("buildDefaultWindowUiStateSerialized matches the default managed window layout", () => {
  assert.deepEqual(
    JSON.parse(buildDefaultWindowUiStateSerialized()),
    normalizeWindowUiState(null),
  );
});

test("buildMapUiResetMountOptions preserves the current view while clearing UI state", () => {
  assert.deepEqual(buildMapUiResetMountOptions(null), {});

  assert.deepEqual(buildMapUiResetMountOptions({
    view: {
      viewMode: "3d",
      camera: {
        centerWorldX: 12,
        centerWorldZ: 34,
        distance: 56,
      },
    },
  }), {
    initialState: {
      version: 1,
      commands: {
        setViewMode: "3d",
        restoreView: {
          viewMode: "3d",
          camera: {
            centerWorldX: 12,
            centerWorldZ: 34,
            distance: 56,
          },
        },
      },
    },
  });
});

test("normalizeBookmarkCoordinate keeps finite floating point coordinates", () => {
  assert.equal(normalizeBookmarkCoordinate("123.45678"), 123.457);
  assert.equal(normalizeBookmarkCoordinate("-987.65432"), -987.654);
  assert.equal(normalizeBookmarkCoordinate("nope"), null);
});

test("normalizeBookmarks filters invalid entries and keeps bookmark metadata", () => {
  assert.deepEqual(
    normalizeBookmarks({
      bookmarks: [
        {
          id: "a",
          label: "",
          zoneName: "Velia Coast",
          resourceName: "Velia",
          originName: "Velia",
          worldX: 123.4567,
          worldZ: -45.6789,
        },
        { id: "a", label: "duplicate", worldX: 999, worldZ: 999 },
        { id: "b", label: "Manual", worldX: "bad", worldZ: 12 },
        { id: "c", label: "Manual", zoneRgb: "255", worldX: "12.5", worldZ: "8.25" },
      ],
    }),
    [
      {
        id: "a",
        label: "Velia Coast",
        zoneName: "Velia Coast",
        resourceName: "Velia",
        originName: "Velia",
        zoneRgb: null,
        worldX: 123.457,
        worldZ: -45.679,
        createdAt: null,
      },
      {
        id: "c",
        label: "Manual",
        zoneName: null,
        zoneRgb: 255,
        worldX: 12.5,
        worldZ: 8.25,
        createdAt: null,
      },
    ],
  );
});

test("resolveDisplayBookmarks fills imported bookmark metadata from the snapshot state", () => {
  const bookmarks = [
    {
      id: "bookmark-a",
      label: "Tarif",
      worldX: 12.5,
      worldZ: 8.25,
    },
  ];

  const stateBundle = {
    state: {
      ui: {
        bookmarks: [
          {
            id: "bookmark-a",
            label: "Tarif",
            zoneName: "Mediah",
            resourceName: "Tarif",
            originName: "Tarif",
            worldX: 12.5,
            worldZ: 8.25,
          },
        ],
      },
    },
  };

  assert.deepEqual(resolveDisplayBookmarks(stateBundle, bookmarks), [
    {
      id: "bookmark-a",
      label: "Tarif",
      zoneName: "Mediah",
      resourceName: "Tarif",
      originName: "Tarif",
      zoneRgb: null,
      worldX: 12.5,
      worldZ: 8.25,
      createdAt: null,
    },
  ]);
});

test("createBookmarkFromPlacement uses zone name as the default label", () => {
  assert.deepEqual(
    createBookmarkFromPlacement(
      {
        worldX: 123.4567,
        worldZ: -45.6789,
        zoneName: "Cron Islands - Depth 2",
        resourceName: "Tarif",
        originName: "Tarif",
        zoneRgb: 12345,
      },
      [],
      {
        idFactory: () => "bookmark-1",
        now: Date.UTC(2026, 2, 20, 12, 0, 0),
      },
    ),
    {
      id: "bookmark-1",
      label: "Cron Islands - Depth 2",
      zoneName: "Cron Islands - Depth 2",
      resourceName: "Tarif",
      originName: "Tarif",
      zoneRgb: 12345,
      worldX: 123.457,
      worldZ: -45.679,
      createdAt: "2026-03-20T12:00:00.000Z",
    },
  );
});

test("renameBookmark updates the label and falls back to the zone name when cleared", () => {
  const bookmarks = normalizeBookmarks([
    {
      id: "bookmark-1",
      label: "Cron Islands - Depth 2",
      zoneName: "Cron Islands - Depth 2",
      worldX: 123.4567,
      worldZ: -45.6789,
    },
  ]);

  assert.deepEqual(renameBookmark(bookmarks, "bookmark-1", "Shipwreck Route"), [
    {
      id: "bookmark-1",
      label: "Shipwreck Route",
      zoneName: "Cron Islands - Depth 2",
      zoneRgb: null,
      worldX: 123.457,
      worldZ: -45.679,
      createdAt: null,
    },
  ]);

  assert.deepEqual(renameBookmark(bookmarks, "bookmark-1", "   "), [
    {
      id: "bookmark-1",
      label: "Cron Islands - Depth 2",
      zoneName: "Cron Islands - Depth 2",
      zoneRgb: null,
      worldX: 123.457,
      worldZ: -45.679,
      createdAt: null,
    },
  ]);
});

test("serializeBookmarksForExport writes WorldmapBookMark XML with comments", () => {
  assert.equal(
    serializeBookmarksForExport([
      {
        id: "bookmark-1",
        label: "Shipwreck Route",
        zoneName: "Cron Islands - Depth 2",
        worldX: 123.4567,
        worldZ: -45.6789,
      },
      {
        id: "bookmark-2",
        label: "Harbor Loop",
        zoneName: "Cron Islands - Depth 2",
        worldX: 14.25,
        worldZ: 80.5,
      },
    ]),
    [
      "<!--",
      "\tWaypoints for: Cron Islands - Depth 2",
      "\tAuto-Generated by: FishyStuff",
      "\tPreview at: https://fishystuff.fish/map/",
      "-->",
      "<WorldmapBookMark>",
      '\t<BookMark BookMarkName="1: Shipwreck Route" PosX="123.457" PosY="-8175.0" PosZ="-45.679" />',
      '\t<BookMark BookMarkName="2: Harbor Loop" PosX="14.25" PosY="-8175.0" PosZ="80.5" />',
      "</WorldmapBookMark>",
    ].join("\n"),
  );
});

test("parseImportedBookmarks accepts wrapped WorldmapBookMark XML", () => {
  const importedBookmarks = parseImportedBookmarks(
    [
      "<!--",
      "\tWaypoints for: Cron Islands - Depth 2",
      "\tAuto-Generated by: FishyStuff",
      "-->",
      "<WorldmapBookMark>",
      '\t<BookMark BookMarkName="1: Shipwreck Route" PosX="123.4567" PosY="-8175.0" PosZ="-45.6789" />',
      '\t<BookMark BookMarkName="2: Harbor Loop" PosX="14.25" PosY="-8175.0" PosZ="80.5" />',
      "</WorldmapBookMark>",
    ].join("\n"),
    {
      idFactory: (() => {
        let index = 0;
        return () => `bookmark-${++index}`;
      })(),
    },
  );

  assert.deepEqual(importedBookmarks, [
    {
      id: "bookmark-1",
      label: "Shipwreck Route",
      zoneName: "Cron Islands - Depth 2",
      zoneRgb: null,
      worldX: 123.457,
      worldZ: -45.679,
      createdAt: null,
    },
    {
      id: "bookmark-2",
      label: "Harbor Loop",
      zoneName: "Cron Islands - Depth 2",
      zoneRgb: null,
      worldX: 14.25,
      worldZ: 80.5,
      createdAt: null,
    },
  ]);
});

test("parseImportedBookmarks accepts bare BookMark nodes and mergeImportedBookmarks skips duplicates by content", () => {
  const importedBookmarks = parseImportedBookmarks(
    [
      '<BookMark BookMarkName="1: Shipwreck Route" PosX="123.4567" PosY="-8175.0" PosZ="-45.6789" />',
      '<BookMark BookMarkName="2: Harbor Loop" PosX="14.25" PosY="-8175.0" PosZ="80.5" />',
    ].join("\n"),
    {
      idFactory: (() => {
        let index = 10;
        return () => `bookmark-${++index}`;
      })(),
    },
  );

  assert.deepEqual(
    mergeImportedBookmarks(
      [
        {
          id: "bookmark-1",
          label: "Shipwreck Route",
          zoneName: "Cron Islands - Depth 2",
          worldX: 123.4567,
          worldZ: -45.6789,
        },
      ],
      importedBookmarks,
    ),
    [
      {
        id: "bookmark-1",
        label: "Shipwreck Route",
        zoneName: "Cron Islands - Depth 2",
        zoneRgb: null,
        worldX: 123.457,
        worldZ: -45.679,
        createdAt: null,
      },
      {
        id: "bookmark-12",
        label: "Harbor Loop",
        zoneName: "Harbor Loop",
        zoneRgb: null,
        worldX: 14.25,
        worldZ: 80.5,
        createdAt: null,
      },
    ],
  );
});

test("moveBookmarkBefore reorders bookmarks relative to the dragged target", () => {
  assert.deepEqual(
    moveBookmarkBefore(
      [
        { id: "bookmark-1", label: "One", worldX: 1, worldZ: 1 },
        { id: "bookmark-2", label: "Two", worldX: 2, worldZ: 2 },
        { id: "bookmark-3", label: "Three", worldX: 3, worldZ: 3 },
      ],
      "bookmark-1",
      "bookmark-3",
      "after",
    ).map((bookmark) => bookmark.id),
    ["bookmark-2", "bookmark-3", "bookmark-1"],
  );
});

test("computeDragAutoScrollDelta scrolls toward the nearest list edge", () => {
  const rect = { top: 100, bottom: 300 };

  assert.ok(computeDragAutoScrollDelta(rect, 112) < 0);
  assert.ok(computeDragAutoScrollDelta(rect, 288) > 0);
  assert.equal(computeDragAutoScrollDelta(rect, 200), 0);
});

test("computeDragAutoScrollDelta stops when the pointer is too far from the container", () => {
  const rect = { top: 100, bottom: 300 };

  assert.equal(computeDragAutoScrollDelta(rect, 20), 0);
  assert.equal(computeDragAutoScrollDelta(rect, 380), 0);
});
