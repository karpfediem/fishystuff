import test from "node:test";
import assert from "node:assert/strict";

globalThis.__fishystuffLoaderAutoStart = false;
const {
  buildBookmarkDeletionPrompt,
  buildBookmarkOverviewRows,
  buildDefaultWindowUiStateSerialized,
  buildFocusWorldRect,
  buildHoverOverviewRows,
  buildRestoreViewForWorldRect,
  buildSemanticIdentityCommand,
  buildSelectWorldPointCommand,
  buildWaypointFocusIndex,
  buildZoneEvidenceSummary,
  buildSelectionSummaryText,
  buildSelectionOverviewRows,
  buildZoneEvidenceListMarkup,
  buildMapUiResetMountOptions,
  buildPointDetailPanes,
  buildPointDetailViewModel,
  pointDetailPaneMarkupKey,
  territoryPointDetailPaneMarkup,
  buildSearchMatches,
  computeDragAutoScrollDelta,
  createBookmarkFromPlacement,
  mergeImportedBookmarks,
  moveBookmarkBefore,
  normalizeZoneCatalog,
  normalizeBookmarks,
  normalizeBookmarkCoordinate,
  normalizeZoneInfoTab,
  normalizeWindowUiState,
  parseZoneRgbSearch,
  parseImportedBookmarks,
  parseWindowUiState,
  projectStateBundleStatePatch,
  renameBookmark,
  resolveHoveredBookmark,
  resolveDisplayBookmarks,
  resolveZoneInfoActiveTab,
  renderSearchSelection,
  selectionHasZoneEvidence,
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
        semanticTerms: [],
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
      view: {
        viewMode: "2d",
        camera: {
          zoom: 1,
        },
      },
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

function factEntry(key, value, icon, extra = {}) {
  return {
    key,
    label: key,
    value,
    icon,
    ...extra,
  };
}

function detailSection(id, title, facts) {
  return {
    id,
    kind: "facts",
    title,
    facts,
  };
}

function zoneLayerSample(zoneName = "Demi River") {
  return {
    layerId: "zone_mask",
    detailSections: [
      detailSection("zone", "Zone", [factEntry("zone", zoneName, "hover-zone", { label: "Zone" })]),
    ],
  };
}

function regionGroupLayerSample({
  resourceBarNode = null,
  containingRegion = null,
  containingRegionStatusIcon = null,
  containingRegionStatusIconTone = null,
} = {}) {
  const facts = [];
  if (resourceBarNode) {
    facts.push(
      factEntry("resource_group", resourceBarNode, "hover-resources", { label: "Resource group" }),
    );
  }
  if (resourceBarNode) {
    facts.push(factEntry("resource_waypoint", resourceBarNode, "map-pin", { label: "Waypoint" }));
  }
  if (containingRegion) {
    facts.push(
      factEntry("resource_region", containingRegion, "hover-zone", {
        label: "Region",
        ...(containingRegionStatusIcon ? { statusIcon: containingRegionStatusIcon } : {}),
        ...(containingRegionStatusIconTone
          ? { statusIconTone: containingRegionStatusIconTone }
          : {}),
      }),
    );
  }
  return {
    layerId: "region_groups",
    detailSections: [detailSection("resource-bar", "Resources", facts)],
  };
}

function regionLayerSample({
  tradeOrigin = null,
  originNode = null,
  tradeOriginStatusIcon = null,
  tradeOriginStatusIconTone = null,
} = {}) {
  const facts = [];
  if (tradeOrigin) {
    facts.push(
      factEntry("origin_region", tradeOrigin, "hover-origin", {
        label: "Region",
        ...(tradeOriginStatusIcon ? { statusIcon: tradeOriginStatusIcon } : {}),
        ...(tradeOriginStatusIconTone ? { statusIconTone: tradeOriginStatusIconTone } : {}),
      }),
    );
  }
  if (originNode) {
    facts.push(factEntry("origin_node", originNode, "map-pin", { label: "Node" }));
  }
  return {
    layerId: "regions",
    detailSections: [detailSection("trade-origin", "Trade Origin", facts)],
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

test("buildSearchMatches returns semantic hits and skips selected semantic fields", () => {
  const stateBundle = buildStateBundle();
  stateBundle.state.catalog.semanticTerms = [
    {
      layerId: "regions",
      layerName: "Regions",
      fieldId: 76,
      label: "Southern Mountain Range",
      description: "Castle Ruins",
      searchText: "Regions Southern Mountain Range Castle Ruins 76",
    },
    {
      layerId: "region_groups",
      layerName: "Region Groups",
      fieldId: 295,
      label: "Olvia",
      description: "Olvia Academy",
      searchText: "Region Groups Olvia Olvia Academy 295",
    },
  ];

  const semanticMatches = buildSearchMatches(stateBundle, "southern mountain", TEST_ZONE_CATALOG);
  assert.equal(semanticMatches[0]?.kind, "semantic");
  assert.equal(semanticMatches[0]?.layerId, "regions");
  assert.equal(semanticMatches[0]?.fieldId, 76);

  stateBundle.inputState.filters.semanticFieldIdsByLayer = {
    regions: [76],
  };
  const filteredMatches = buildSearchMatches(
    stateBundle,
    "southern mountain",
    TEST_ZONE_CATALOG,
  );
  assert.equal(filteredMatches.some((match) => match.kind === "semantic" && match.fieldId === 76), false);
});

test("buildHoverOverviewRows renders supported hover layers from bottom to top", () => {
  assert.deepEqual(
    buildHoverOverviewRows(
      {
        layerSamples: [
          zoneLayerSample("Demi River"),
          regionGroupLayerSample({ containingRegion: "Tarif" }),
          regionLayerSample({ tradeOrigin: "Tarif" }),
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
        icon: "trade-origin",
        label: "Origin",
        value: "Tarif",
      },
    ],
  );
});

test("buildSelectionOverviewRows keeps field semantics while omitting a duplicate zone row", () => {
  assert.deepEqual(
    buildSelectionOverviewRows(
      {
        layerSamples: [
          zoneLayerSample("Demi River"),
          regionGroupLayerSample({ containingRegion: "Tarif" }),
          regionLayerSample({ tradeOrigin: "Tarif" }),
        ],
      },
      buildHoverStateBundle(),
    ),
    [
      {
        layerId: "region_groups",
        icon: "hover-resources",
        label: "Resources",
        value: "Tarif",
      },
      {
        layerId: "regions",
        icon: "trade-origin",
        label: "Origin",
        value: "Tarif",
      },
    ],
  );
});

test("buildSelectionOverviewRows keeps the zone row when no zone summary is available", () => {
  assert.deepEqual(
    buildSelectionOverviewRows(
      {
        layerSamples: [zoneLayerSample("Demi River")],
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
    ],
  );
});

test("buildSelectionSummaryText falls back to semantic rows for non-zone selections", () => {
  assert.equal(
    buildSelectionSummaryText(
      {
        layerSamples: [
          regionGroupLayerSample({ containingRegion: "Olvia" }),
          regionLayerSample({ tradeOrigin: "Castle Ruins" }),
        ],
      },
      buildHoverStateBundle(),
    ),
    "Olvia",
  );
});

test("buildSelectionSummaryText uses the primary zone row when present", () => {
  assert.equal(
    buildSelectionSummaryText(
      {
        layerSamples: [
          zoneLayerSample("Demi River"),
          regionGroupLayerSample({ containingRegion: "Tarif" }),
        ],
      },
      buildHoverStateBundle(),
    ),
    "Demi River",
  );
});

test("buildSelectionSummaryText prefers semantic row keys over display labels", () => {
  assert.equal(
    buildSelectionSummaryText(
      {
        layerSamples: [
          regionGroupLayerSample({ containingRegion: "Olvia" }),
          regionLayerSample({ tradeOrigin: "Castle Ruins" }),
        ],
      },
      buildHoverStateBundle(),
    ),
    "Olvia",
  );
});

test("projectStateBundleStatePatch preserves unrelated applied filters for optimistic search selection", () => {
  const stateBundle = {
    state: {},
    inputState: {
      version: 1,
      filters: {
        fishIds: [912],
        zoneRgbs: [0xc17f7f],
        searchText: "cron",
      },
    },
  };

  const nextFilters = projectStateBundleStatePatch(stateBundle, {
    version: 1,
    filters: {
      searchText: "",
      fishIds: [912, 77],
    },
  }).inputState.filters;

  assert.deepEqual(nextFilters.fishIds, [912, 77]);
  assert.deepEqual(nextFilters.zoneRgbs, [0xc17f7f]);
  assert.equal(nextFilters.searchText, "");
});

test("buildZoneEvidenceSummary explains that non-zone selections have no zone evidence", () => {
  assert.equal(buildZoneEvidenceSummary({ zoneRgb: null }, null), "Zone evidence is only available for zone selections.");
  assert.equal(buildZoneEvidenceSummary({ zoneRgb: 0xc17f7f }, null), "No zone evidence loaded.");
});

test("selectionHasZoneEvidence distinguishes zone-backed selections", () => {
  assert.equal(selectionHasZoneEvidence(null, null), false);
  assert.equal(selectionHasZoneEvidence({ layerSamples: [] }, null), false);
  assert.equal(
    selectionHasZoneEvidence(
      {
        layerSamples: [{ layerId: "zone_mask", rgbU32: 0xc17f7f }],
      },
      null,
    ),
    true,
  );
});

test("resolveZoneInfoActiveTab falls back to the first sampled layer", () => {
  assert.equal(normalizeZoneInfoTab(" regions "), "regions");
  assert.equal(
    resolveZoneInfoActiveTab(
      {
        zoneInfo: { tab: "region_groups" },
      },
      {
        layerSamples: [
          { layerId: "zone_mask", rgbU32: 0xc17f7f },
          { layerId: "region_groups", fieldId: 295 },
        ],
      },
      buildStateBundle(),
    ),
    "region_groups",
  );
  assert.equal(
    resolveZoneInfoActiveTab(
      {
        zoneInfo: { tab: "missing" },
      },
      {
        layerSamples: [
          { layerId: "zone_mask", rgbU32: 0xc17f7f },
          { layerId: "region_groups", fieldId: 295 },
        ],
      },
      buildStateBundle(),
    ),
    "zone_mask",
  );
});

test("buildPointDetailPanes preserves layer order and summaries", () => {
  const panes = buildPointDetailPanes(
    {
      layerSamples: [
        {
          layerId: "zone_mask",
          layerName: "Zone Mask",
          detailPane: { id: "zone_mask", label: "Zone", icon: "hover-zone", order: 100 },
          detailSections: [
            {
              id: "zone",
              kind: "facts",
              title: "Zone",
              facts: [{ key: "zone", label: "Zone", value: "Demi River", icon: "hover-zone" }],
            },
          ],
        },
        {
          layerId: "region_groups",
          layerName: "Region Groups",
          detailPane: { id: "territory", label: "Territory", icon: "hover-resources", order: 200 },
          detailSections: [
            {
              id: "resource-bar",
              kind: "facts",
              title: "Resources",
              facts: [{ key: "resource_region", label: "Containing region", value: "Tarif", icon: "hover-zone" }],
            },
          ],
        },
        {
          layerId: "regions",
          layerName: "Regions",
          detailPane: { id: "territory", label: "Territory", icon: "hover-origin", order: 200 },
          detailSections: [
            {
              id: "trade-origin",
              kind: "facts",
              title: "Trade Origin",
              facts: [{ key: "origin_region", label: "Region", value: "Tarif", icon: "hover-origin" }],
            },
          ],
        },
      ],
    },
    buildHoverStateBundle(),
  );

  assert.deepEqual(
    panes.map((pane) => [pane.id, pane.label, pane.summary]),
    [
      ["zone_mask", "Zone", "Demi River"],
      ["territory", "Territory", "Tarif"],
    ],
  );
});

test("buildPointDetailPanes keeps territory panes when source layers are hidden", () => {
  const stateBundle = buildHoverStateBundle();
  stateBundle.state.catalog.layers = stateBundle.state.catalog.layers.map((layer) =>
    layer.layerId === "regions" || layer.layerId === "region_groups"
      ? { ...layer, visible: false }
      : layer,
  );

  const panes = buildPointDetailPanes(
    {
      layerSamples: [
        {
          layerId: "region_groups",
          layerName: "Region Groups",
          detailPane: { id: "territory", label: "Territory", icon: "hover-resources", order: 200 },
          detailSections: [
            {
              id: "resource-bar",
              kind: "facts",
              title: "Resources",
              facts: [{ key: "resource_region", label: "Containing region", value: "Tarif", icon: "hover-zone" }],
            },
          ],
        },
        {
          layerId: "regions",
          layerName: "Regions",
          detailPane: { id: "territory", label: "Territory", icon: "hover-origin", order: 200 },
          detailSections: [
            {
              id: "trade-origin",
              kind: "facts",
              title: "Trade Origin",
              facts: [{ key: "origin_region", label: "Region", value: "Tarif", icon: "hover-origin" }],
            },
          ],
        },
      ],
    },
    stateBundle,
  );

  assert.deepEqual(
    panes.map((pane) => [pane.id, pane.sections.map((section) => section.id)]),
    [["territory", ["resource-bar", "trade-origin"]]],
  );
});

test("buildPointDetailPanes keeps zone evidence as a zone-only section", () => {
  const stateBundle = buildHoverStateBundle();
  stateBundle.state.selection = {
    zoneStats: {
      zoneRgb: 0xc17f7f,
      distribution: [],
    },
  };
  stateBundle.state.statuses = {
    zoneStatsStatus: "zone evidence: ready",
  };
  const panes = buildPointDetailPanes(
    {
      layerSamples: [
        {
          layerId: "zone_mask",
          layerName: "Zone Mask",
          detailPane: { id: "zone_mask", label: "Zone", icon: "hover-zone", order: 100 },
          detailSections: [
            {
              id: "zone",
              kind: "facts",
              title: "Zone",
              facts: [{ key: "zone", label: "Zone", value: "Demi River", icon: "hover-zone" }],
              targets: [{ label: "Tarif", worldX: 10, worldZ: 20 }],
            },
          ],
        },
        {
          layerId: "region_groups",
          layerName: "Region Groups",
          detailPane: { id: "territory", label: "Territory", icon: "hover-resources", order: 200 },
          detailSections: [
            {
              id: "resource-bar",
              kind: "facts",
              title: "Resources",
              facts: [{ key: "resource_region", label: "Containing region", value: "Tarif", icon: "hover-zone" }],
              targets: [{ label: "Resource node", worldX: 30, worldZ: 40 }],
            },
          ],
        },
      ],
    },
    stateBundle,
  );

  assert.deepEqual(
    panes.map((pane) => [pane.id, pane.sections.map((section) => section.id)]),
    [
      ["zone_mask", ["zone", "zone-evidence"]],
      ["territory", ["resource-bar"]],
    ],
  );
});

test("buildPointDetailViewModel resolves the requested active pane when available", () => {
  const viewModel = buildPointDetailViewModel(
    {
      layerSamples: [
        {
          layerId: "zone_mask",
          layerName: "Zone Mask",
          detailPane: { id: "zone_mask", label: "Zone", icon: "hover-zone", order: 100 },
          detailSections: [
            {
              id: "zone",
              kind: "facts",
              title: "Zone",
              facts: [{ key: "zone", label: "Zone", value: "Demi River", icon: "hover-zone" }],
            },
          ],
        },
        {
          layerId: "region_groups",
          layerName: "Region Groups",
          detailPane: { id: "territory", label: "Territory", icon: "hover-resources", order: 200 },
          detailSections: [
            {
              id: "resource-bar",
              kind: "facts",
              title: "Resources",
              facts: [{ key: "resource_region", label: "Containing region", value: "Tarif", icon: "hover-zone" }],
            },
          ],
        },
      ],
    },
    buildHoverStateBundle(),
    {
      zoneInfo: { tab: "territory" },
    },
  );

  assert.equal(viewModel.activePaneId, "territory");
  assert.equal(viewModel.activePane?.id, "territory");
  assert.equal(viewModel.descriptor.title, "Demi River");
  assert.equal(viewModel.activePane?.summary, "Tarif");
});

test("buildPointDetailViewModel uses bookmark titles over semantic summaries", () => {
  const stateBundle = buildHoverStateBundle();
  stateBundle.state.ui = {
    bookmarks: [
      {
        id: "bookmark-a",
        label: "Olvia Academy",
        worldX: 10,
        worldZ: 20,
      },
    ],
  };
  stateBundle.inputState.ui = {
    bookmarkSelectedIds: ["bookmark-a"],
  };

  const viewModel = buildPointDetailViewModel(
    {
      worldX: 10,
      worldZ: 20,
      pointKind: "bookmark",
      pointLabel: "Olvia Academy",
      layerSamples: [
        {
          layerId: "zone_mask",
          layerName: "Zone Mask",
          detailPane: { id: "zone_mask", label: "Zone", icon: "hover-zone", order: 100 },
          detailSections: [
            {
              id: "zone",
              kind: "facts",
              title: "Zone",
              facts: [{ key: "zone", label: "Zone", value: "Demi River", icon: "hover-zone" }],
            },
          ],
        },
      ],
    },
    stateBundle,
    {
      zoneInfo: { tab: "zone_mask" },
    },
  );

  assert.equal(viewModel.descriptor.title, "Olvia Academy");
  assert.equal(viewModel.descriptor.pointKind, "bookmark");
  assert.equal(viewModel.activePaneId, "zone_mask");
});

test("territoryPointDetailPaneMarkup keeps waypoint buttons in the compact pane", () => {
  const markup = territoryPointDetailPaneMarkup({
    id: "territory",
    sections: [
      {
        id: "resource-bar",
        kind: "facts",
        title: "Resources",
        facts: [
          { key: "resource_group", label: "Region Group", value: "Tarif (RG58)", icon: "hover-resources" },
          { key: "resource_waypoint", label: "Waypoint", value: "Hasrah Cliff", icon: "map-pin" },
          { key: "resource_region", label: "Region", value: "Tarif (R216)", icon: "hover-zone" },
        ],
        targets: [
          { key: "resource_node", label: "Resource bar: Tarif (RG58)", worldX: 189607, worldZ: -160661 },
        ],
      },
      {
        id: "trade-origin",
        kind: "facts",
        title: "Trade Origin",
        facts: [
          { key: "origin_region", label: "Region", value: "Tarif (R216)", icon: "hover-origin" },
        ],
        targets: [
          { key: "origin_node", label: "Origin: Tarif (R216)", worldX: 226814, worldZ: -73831.4 },
        ],
      },
    ],
  });

  assert.match(markup, /Resources/);
  assert.match(markup, /Origin/);
  assert.match(markup, /Waypoint/);
  assert.match(markup, /Hasrah Cliff/);
  assert.match(markup, /fishymap-semantic-chip-code">RG58</);
  assert.match(markup, /fishymap-semantic-chip-name">Tarif</);
  assert.match(markup, /fishymap-semantic-chip-code">R216</);
  assert.match(markup, /data-zone-info-target-world-x="189607"/);
  assert.match(markup, /Resource bar: Tarif \(RG58\)/);
  assert.match(markup, /data-zone-info-target-world-x="226814"/);
  assert.match(markup, /Origin: Tarif \(R216\)/);
});

test("pointDetailPaneMarkupKey changes when territory facts change with same section ids", () => {
  const firstKey = pointDetailPaneMarkupKey({
    id: "territory",
    summary: "Tarif",
    sections: [
      {
        id: "resource-bar",
        kind: "facts",
        title: "Resources",
        facts: [
          { key: "resource_group", label: "Region Group", value: "Tarif (RG54)", icon: "hover-resources" },
          { key: "resource_region", label: "Region", value: "Tarif (R213)", icon: "hover-zone" },
        ],
        targets: [
          { key: "resource_node", label: "Resources: Tarif (RG54)", worldX: 100, worldZ: 200 },
        ],
      },
      {
        id: "trade-origin",
        kind: "facts",
        title: "Trade Origin",
        facts: [{ key: "origin_region", label: "Region", value: "Tarif (R213)", icon: "trade-origin" }],
        targets: [{ key: "origin_node", label: "Origin: Tarif (R213)", worldX: 300, worldZ: 400 }],
      },
    ],
  });
  const secondKey = pointDetailPaneMarkupKey({
    id: "territory",
    summary: "Tarif",
    sections: [
      {
        id: "resource-bar",
        kind: "facts",
        title: "Resources",
        facts: [
          { key: "resource_group", label: "Region Group", value: "Tarif (RG54)", icon: "hover-resources" },
          { key: "resource_region", label: "Region", value: "Tarif (R209)", icon: "hover-zone" },
        ],
        targets: [
          { key: "resource_node", label: "Resources: Tarif (RG54)", worldX: 100, worldZ: 200 },
        ],
      },
      {
        id: "trade-origin",
        kind: "facts",
        title: "Trade Origin",
        facts: [{ key: "origin_region", label: "Region", value: "Tarif (R209)", icon: "trade-origin" }],
        targets: [{ key: "origin_node", label: "Origin: Tarif (R209)", worldX: 300, worldZ: 400 }],
      },
    ],
  });

  assert.notEqual(firstKey, secondKey);
});

test("buildBookmarkDeletionPrompt uses the bookmark label for single deletions", () => {
  assert.equal(
    buildBookmarkDeletionPrompt([
      {
        id: "bookmark-a",
        label: "Tarif route",
        worldX: 10,
        worldZ: 20,
      },
    ]),
    'Delete bookmark "Tarif route"?',
  );
});

test("buildBookmarkDeletionPrompt summarizes multi-delete confirmations", () => {
  assert.equal(
    buildBookmarkDeletionPrompt(
      [
        {
          id: "bookmark-a",
          label: "Tarif route",
          worldX: 10,
          worldZ: 20,
        },
        {
          id: "bookmark-b",
          label: "Velia route",
          worldX: 30,
          worldZ: 40,
        },
        {
          id: "bookmark-c",
          label: "Hasrah route",
          worldX: 50,
          worldZ: 60,
        },
        {
          id: "bookmark-d",
          label: "Ancado route",
          worldX: 70,
          worldZ: 80,
        },
      ],
      { selection: true },
    ),
    [
      "Delete 4 selected bookmarks?",
      "",
      "1. Tarif route",
      "2. Velia route",
      "3. Hasrah route",
      "...and 1 more.",
    ].join("\n"),
  );
});

test("resolveHoveredBookmark matches the nearest bookmark under the cursor", () => {
  const hoveredBookmark = resolveHoveredBookmark(
    {
      worldX: 100,
      worldZ: 100,
    },
    buildHoverStateBundle(),
    [
      {
        id: "bookmark-a",
        label: "Velia route",
        worldX: 104,
        worldZ: 103,
      },
      {
        id: "bookmark-b",
        label: "Tarif route",
        worldX: 112,
        worldZ: 112,
      },
    ],
  );
  assert.equal(hoveredBookmark?.bookmark?.id, "bookmark-a");
  assert.equal(hoveredBookmark?.bookmark?.label, "Velia route");
  assert.equal(hoveredBookmark?.index, 0);

  assert.equal(
    resolveHoveredBookmark(
      {
        worldX: 100,
        worldZ: 100,
      },
      buildHoverStateBundle(),
      [
        {
          id: "bookmark-a",
          label: "Velia route",
          worldX: 150,
          worldZ: 150,
        },
      ],
    ),
    null,
  );
});

test("buildHoverOverviewRows keeps bookmark info out of the regular hover box", () => {
  assert.deepEqual(
    buildHoverOverviewRows(
      {
        worldX: 100,
        worldZ: 100,
        layerSamples: [
          zoneLayerSample("Demi River"),
          regionGroupLayerSample({ containingRegion: "Tarif" }),
          regionLayerSample({ tradeOrigin: "Tarif" }),
        ],
      },
      buildHoverStateBundle(),
      {
        bookmarks: [
          {
            id: "bookmark-a",
            label: "Velia route",
            worldX: 102,
            worldZ: 101,
          },
          {
            id: "bookmark-b",
            label: "Tarif route",
            worldX: 300,
            worldZ: 300,
          },
        ],
        selectedIds: ["bookmark-a", "bookmark-b"],
      },
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
        icon: "trade-origin",
        label: "Origin",
        value: "Tarif",
      },
    ],
  );
});

test("buildHoverOverviewRows falls back to region ids when assignments are missing", () => {
  assert.deepEqual(
    buildHoverOverviewRows(
      {
        layerSamples: [
          zoneLayerSample("Demi River"),
          regionGroupLayerSample({
            containingRegion: "RG16",
            containingRegionStatusIcon: "question-mark",
          }),
          regionLayerSample({
            tradeOrigin: "R76",
            tradeOriginStatusIcon: "question-mark",
          }),
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
        value: "RG16",
        statusIcon: "question-mark",
      },
      {
        layerId: "regions",
        icon: "trade-origin",
        label: "Origin",
        value: "R76",
        statusIcon: "question-mark",
      },
    ],
  );
});

test("buildHoverOverviewRows keeps a soft unknown marker when resource coordinates exist without a name", () => {
  assert.deepEqual(
    buildHoverOverviewRows(
      {
        layerSamples: [
          zoneLayerSample("Demi River"),
          regionGroupLayerSample({
            containingRegion: "R76",
            containingRegionStatusIcon: "question-mark",
            containingRegionStatusIconTone: "subtle",
          }),
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
        value: "R76",
        statusIcon: "question-mark",
        statusIconTone: "subtle",
      },
    ],
  );
});

test("buildHoverOverviewRows keeps a soft unknown marker when origin coordinates exist without a name", () => {
  assert.deepEqual(
    buildHoverOverviewRows(
      {
        layerSamples: [
          zoneLayerSample("Demi River"),
          regionLayerSample({
            tradeOrigin: "R76",
            tradeOriginStatusIcon: "question-mark",
            tradeOriginStatusIconTone: "subtle",
          }),
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
        layerId: "regions",
        icon: "trade-origin",
        label: "Origin",
        value: "R76",
        statusIcon: "question-mark",
        statusIconTone: "subtle",
      },
    ],
  );
});

test("buildBookmarkOverviewRows mirrors the hover row style without duplicating the zone", () => {
  assert.deepEqual(
    buildBookmarkOverviewRows(
      {
        label: "Tarif hotspot",
        layerSamples: [
          zoneLayerSample("Tarif"),
          regionGroupLayerSample({ containingRegion: "Tarif" }),
          regionLayerSample({ tradeOrigin: "Tarif" }),
        ],
      },
      0,
    ),
    [
      {
        icon: "bookmark",
        label: "Bookmark",
        value: "Tarif hotspot",
        hideLabel: true,
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
        icon: "trade-origin",
        label: "Origin",
        value: "Tarif",
      },
    ],
  );

  assert.deepEqual(
    buildBookmarkOverviewRows(
      {
        label: "Tarif",
        layerSamples: [
          zoneLayerSample("Tarif"),
          regionGroupLayerSample({ containingRegion: "Tarif" }),
          regionLayerSample({ tradeOrigin: "Tarif" }),
        ],
      },
      0,
    ),
    [
      {
        icon: "bookmark",
        label: "Bookmark",
        value: "Tarif",
        hideLabel: true,
      },
      {
        icon: "hover-resources",
        label: "Resources",
        value: "Tarif",
      },
      {
        icon: "trade-origin",
        label: "Origin",
        value: "Tarif",
      },
    ],
  );

  assert.deepEqual(
    buildBookmarkOverviewRows(
      {
        label: "Unknown route",
        layerSamples: [
          regionGroupLayerSample({
            containingRegion: "RG16",
            containingRegionStatusIcon: "question-mark",
          }),
          regionLayerSample({
            tradeOrigin: "R76",
            tradeOriginStatusIcon: "question-mark",
          }),
        ],
      },
      0,
    ),
    [
      {
        icon: "bookmark",
        label: "Bookmark",
        value: "Unknown route",
        hideLabel: true,
      },
      {
        icon: "hover-resources",
        label: "Resources",
        value: "RG16",
        statusIcon: "question-mark",
      },
      {
        icon: "trade-origin",
        label: "Origin",
        value: "R76",
        statusIcon: "question-mark",
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

test("buildZoneEvidenceListMarkup hides stability percentages while keeping the detail tooltip", () => {
  const originalWindow = globalThis.window;
  const originalLocation = globalThis.location;
  globalThis.window = {
    location: {
      href: "https://fishystuff.fish/map/",
      hostname: "fishystuff.fish",
    },
  };
  globalThis.location = globalThis.window.location;

  const fishLookup = new Map([
    [
      912,
      {
        fishId: 912,
        itemId: 3012,
        encyclopediaId: 4012,
        name: "Cron Dart",
        grade: "Rare",
        isPrize: false,
      },
    ],
  ]);

  try {
    const markup = buildZoneEvidenceListMarkup(
      [
        {
          fishId: 912,
          fishName: "Cron Dart",
          pMean: 0.027,
          evidenceWeight: 0.031,
          ciLow: 0.0,
          ciHigh: 0.184,
        },
      ],
      fishLookup,
    );

    assert.equal(markup.includes('data-zone-evidence-fish-id="912"'), true);
    assert.equal(markup.includes('title="p 0.027 · weight 0.031 · CI 0.000-0.184"'), true);
    assert.equal(
      markup.includes('aria-description="p 0.027 · weight 0.031 · CI 0.000-0.184"'),
      true,
    );
    assert.equal(markup.includes("fishymap-item-icon-frame grade-rare"), true);
    assert.equal(markup.includes("2.7%"), false);
    assert.equal(markup.includes("badge badge-outline badge-sm cursor-help"), false);
  } finally {
    globalThis.window = originalWindow;
    globalThis.location = originalLocation;
  }
});

test("parseWindowUiState falls back to defaults for invalid persisted state", () => {
  assert.deepEqual(parseWindowUiState("not json"), normalizeWindowUiState(null));
  assert.deepEqual(parseWindowUiState(""), normalizeWindowUiState(null));
});

test("serializeWindowUiState normalizes persisted window geometry and flags", () => {
  const serialized = serializeWindowUiState({
    search: { open: false, collapsed: "yes", x: 42.8, y: "13" },
    settings: { open: true, collapsed: false, x: null, y: null, autoAdjustView: false },
    zoneInfo: { open: true, collapsed: false, x: undefined, y: 5.2, tab: "zoneEvidence" },
    layers: { open: false, collapsed: 0, x: "bad", y: 99.9 },
    bookmarks: { open: true, collapsed: true, x: "14", y: 7.8 },
  });

  assert.deepEqual(JSON.parse(serialized), {
    search: { open: false, collapsed: false, x: 43, y: 13 },
    settings: { open: true, collapsed: false, x: null, y: null, autoAdjustView: false },
    zoneInfo: { open: true, collapsed: false, x: null, y: 5, tab: "zoneEvidence" },
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

test("territoryPointDetailPaneMarkup renders semantic focus buttons for region, region group, and node facts", () => {
  const markup = territoryPointDetailPaneMarkup({
    id: "territory",
    sections: [
      {
        id: "resource-bar",
        kind: "facts",
        title: "Resources",
        facts: [
          { key: "resource_group", label: "Region Group", value: "Velia (RG1)", icon: "hover-resources" },
          { key: "resource_waypoint", label: "Waypoint", value: "Velia (N1)", icon: "map-pin" },
        ],
        targets: [
          { key: "resource_node", label: "Resources: Velia (RG1)", worldX: 100, worldZ: 120 },
        ],
      },
      {
        id: "trade-origin",
        kind: "facts",
        title: "Trade Origin",
        facts: [
          { key: "origin_region", label: "Origin", value: "Velia (R5)", icon: "trade-origin" },
          { key: "origin_node", label: "Node", value: "Velia (N1)", icon: "map-pin" },
        ],
      },
    ],
  });

  assert.match(markup, /data-semantic-focus-code="RG1"/);
  assert.match(markup, /data-semantic-focus-code="R5"/);
  assert.match(markup, /data-semantic-focus-code="N1"/);
});

test("buildWaypointFocusIndex and semantic commands resolve focus bounds from waypoint data", () => {
  const focusIndex = buildWaypointFocusIndex({
    regionNodes: {
      type: "FeatureCollection",
      features: [
        {
          type: "Feature",
          properties: { r: 5, wp: 1 },
          geometry: { type: "Point", coordinates: [100, 100] },
        },
        {
          type: "Feature",
          properties: { r: 7, wp: 7 },
          geometry: { type: "Point", coordinates: [260, 110] },
        },
        {
          type: "Feature",
          properties: { r: 9, wp: 9 },
          geometry: { type: "Point", coordinates: [180, 260] },
        },
      ],
    },
    regions: {
      type: "FeatureCollection",
      features: [
        {
          type: "Feature",
          properties: { r: 5, rg: 1, o: 9, owp: 9, ox: 180, oz: 260, rgwp: 11, rgx: 220, rgz: 210 },
          geometry: { type: "Polygon", coordinates: [] },
        },
      ],
    },
    regionGroups: {
      type: "FeatureCollection",
      features: [
        {
          type: "Feature",
          properties: { rg: 1, rgwp: 11, rgx: 220, rgz: 210, rs: [5, 7, 9] },
          geometry: { type: "Polygon", coordinates: [] },
        },
      ],
    },
  });

  const regionGroupCommand = buildSemanticIdentityCommand(
    "Velia (RG1)",
    focusIndex,
    { state: { view: { viewMode: "2d" } } },
    { width: 1000, height: 600 },
    { autoAdjustView: true },
  );
  assert.deepEqual(regionGroupCommand.selectSemanticField, {
    layerId: "region_groups",
    fieldId: 1,
  });
  assert.equal(regionGroupCommand.restoreView.viewMode, "2d");
  assert.equal(regionGroupCommand.restoreView.camera.centerWorldX, 180);
  assert.equal(regionGroupCommand.restoreView.camera.centerWorldZ, 180);
  assert.ok(regionGroupCommand.restoreView.camera.zoom > 0);

  const nodeCommand = buildSemanticIdentityCommand(
    "Velia (N1)",
    focusIndex,
    { state: { view: { viewMode: "2d" } } },
    { width: 1000, height: 600 },
    { autoAdjustView: false },
  );
  assert.deepEqual(nodeCommand, {
    selectWorldPoint: {
      worldX: 100,
      worldZ: 100,
      pointKind: "waypoint",
      pointLabel: "Velia (N1)",
    },
  });
});

test("buildFocusWorldRect keeps waypoint focus padded away from full-screen fit", () => {
  const rect = buildFocusWorldRect(
    [
      { worldX: 100, worldZ: 100 },
      { worldX: 300, worldZ: 180 },
    ],
    { width: 1000, height: 600 },
  );
  assert.ok(rect.spanX > 200);
  assert.ok(rect.spanZ > 80);

  const restoreView = buildRestoreViewForWorldRect(rect, { width: 1000, height: 600 }, {
    state: { view: { viewMode: "2d" } },
  });
  assert.equal(restoreView.viewMode, "2d");
  assert.ok(restoreView.camera.zoom > 0);
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

test("buildSelectWorldPointCommand normalizes bookmark coordinates for commands", () => {
  assert.deepEqual(buildSelectWorldPointCommand("123.45678", "-987.65432"), {
    selectWorldPoint: {
      worldX: 123.457,
      worldZ: -987.654,
    },
  });
  assert.equal(buildSelectWorldPointCommand("bad", 12), null);
});

test("normalizeBookmarks filters invalid entries and keeps bookmark metadata", () => {
  assert.deepEqual(
    normalizeBookmarks({
      bookmarks: [
        {
          id: "a",
          label: "",
          layerSamples: [zoneLayerSample("Velia Coast")],
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
        layerSamples: [zoneLayerSample("Velia Coast")],
        zoneRgb: null,
        worldX: 123.457,
        worldZ: -45.679,
        createdAt: null,
      },
      {
        id: "c",
        label: "Manual",
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
            layerSamples: [zoneLayerSample("Mediah"), regionGroupLayerSample({ resourceBarNode: "Tarif" })],
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
      layerSamples: [zoneLayerSample("Mediah"), regionGroupLayerSample({ resourceBarNode: "Tarif" })],
      zoneRgb: null,
      worldX: 12.5,
      worldZ: 8.25,
      createdAt: null,
    },
  ]);
});

test("createBookmarkFromPlacement uses semantic rows as the default label", () => {
  assert.deepEqual(
    createBookmarkFromPlacement(
      {
        worldX: 123.4567,
        worldZ: -45.6789,
        layerSamples: [zoneLayerSample("Cron Islands - Depth 2")],
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
      layerSamples: [zoneLayerSample("Cron Islands - Depth 2")],
      zoneRgb: 12345,
      worldX: 123.457,
      worldZ: -45.679,
      createdAt: "2026-03-20T12:00:00.000Z",
    },
  );
});

test("renameBookmark updates the label and falls back to semantic rows when cleared", () => {
  const bookmarks = normalizeBookmarks([
    {
      id: "bookmark-1",
      label: "Cron Islands - Depth 2",
      layerSamples: [zoneLayerSample("Cron Islands - Depth 2")],
      worldX: 123.4567,
      worldZ: -45.6789,
    },
  ]);

  assert.deepEqual(renameBookmark(bookmarks, "bookmark-1", "Shipwreck Route"), [
    {
      id: "bookmark-1",
      label: "Shipwreck Route",
      layerSamples: [zoneLayerSample("Cron Islands - Depth 2")],
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
      layerSamples: [zoneLayerSample("Cron Islands - Depth 2")],
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
        layerSamples: [zoneLayerSample("Cron Islands - Depth 2")],
        worldX: 123.4567,
        worldZ: -45.6789,
      },
      {
        id: "bookmark-2",
        label: "Harbor Loop",
        layerSamples: [zoneLayerSample("Cron Islands - Depth 2")],
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
      zoneRgb: null,
      worldX: 123.457,
      worldZ: -45.679,
      createdAt: null,
    },
    {
      id: "bookmark-2",
      label: "Harbor Loop",
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
          layerSamples: [zoneLayerSample("Cron Islands - Depth 2")],
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
        layerSamples: [zoneLayerSample("Cron Islands - Depth 2")],
        zoneRgb: null,
        worldX: 123.457,
        worldZ: -45.679,
        createdAt: null,
      },
      {
        id: "bookmark-12",
        label: "Harbor Loop",
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
