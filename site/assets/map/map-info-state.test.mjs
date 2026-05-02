import { test } from "bun:test";
import assert from "node:assert/strict";
import { installMapTestI18n } from "./test-i18n.js";

import { buildInfoViewModel, patchTouchesInfoSignals } from "./map-info-state.js";

function detailSectionFact(key, label, value, icon) {
  return {
    id: key,
    kind: "facts",
    title: label,
    facts: [
      {
        key,
        label,
        value,
        icon,
      },
    ],
    targets: [],
  };
}

test("buildInfoViewModel groups selection data into zone, territory, and trade panes", () => {
  const viewModel = buildInfoViewModel(
    {
      _map_ui: {
        windowUi: {
          zoneInfo: { tab: "territory" },
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
              detailSections: [detailSectionFact("zone", "Zone", "Valencia Sea - Depth 5", "hover-zone")],
            },
            {
              layerId: "region_groups",
              detailSections: [detailSectionFact("resource_group", "Resources", "(RG212|Arehaza)", "hover-resources")],
            },
            {
              layerId: "regions",
              detailSections: [detailSectionFact("origin_region", "Origin", "(R430|Hakoven Islands)", "trade-origin")],
            },
          ],
        },
        catalog: {
          layers: [
            { layerId: "zone_mask", displayOrder: 20 },
            { layerId: "region_groups", displayOrder: 30 },
            { layerId: "regions", displayOrder: 40 },
          ],
        },
      },
    },
    {
      zoneCatalog: [{ zoneRgb: 0x39e58d, name: "Valencia Sea - Depth 5", biteTimeMin: 5, biteTimeMax: 7 }],
      zoneLootStatus: "loaded",
      zoneLootSummary: {
        available: true,
        profileLabel: "Calculator defaults",
        dataQualityNote:
          "Expected loot uses average session casts, the current Fish multiplier, normalized group shares, and actual source-backed item prices.",
        note: "Zone loot uses calculator default session settings.",
        groups: [
          {
            slotIdx: 4,
            label: "General",
            fillColor: "#eef6ff",
            strokeColor: "#89a8d8",
            textColor: "#1f2937",
            dropRateText: "80%",
            dropRateSourceKind: "database",
            dropRateTooltip: "DB General group share",
            rawDropRateText: "80%",
            rawDropRateTooltip: "DB General group share",
            normalizedDropRateText: "80%",
            normalizedDropRateTooltip: "DB General group share",
            conditionText: "Zone base rate 80%",
            conditionTooltip: "Zone base rate: 80%",
            catchMethods: ["rod"],
            conditionOptions: [
              {
                conditionText: "Default",
                dropRateText: "80%",
                dropRateSourceKind: "database",
                dropRateTooltip: "Default General group lineage",
                presenceText: "Community confirmed×1 · General subgroup",
                presenceSourceKind: "community",
                presenceTooltip: "Community confirmed×1 · General subgroup 11054",
                rawDropRateText: "80%",
                rawDropRateTooltip: "Default General group lineage",
                normalizedDropRateText: "80%",
                normalizedDropRateTooltip: "Default General group lineage",
                active: true,
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
                    rawDropRateText: "80%",
                    rawDropRateTooltip: "DB-backed drop rate",
                    normalizedDropRateText: "80%",
                    normalizedDropRateTooltip: "DB-backed drop rate",
                    catchMethods: ["rod"],
                  },
                ],
              },
              {
                conditionText: "Fishing Level Guru 1+",
                dropRateText: "80%",
                dropRateSourceKind: "database",
                dropRateTooltip: "Guru General group lineage",
                rawDropRateText: "80%",
                rawDropRateTooltip: "Guru General group lineage",
                normalizedDropRateText: "80%",
                normalizedDropRateTooltip: "Guru General group lineage",
                active: false,
                speciesRows: [
                  {
                    slotIdx: 4,
                    groupLabel: "General",
                    label: "Mystical Fish",
                    iconUrl: "/i/mystical-fish.png",
                    iconGradeTone: "rare",
                    fillColor: "#eef6ff",
                    strokeColor: "#89a8d8",
                    textColor: "#1f2937",
                    dropRateText: "0.005%",
                    dropRateSourceKind: "database",
                    dropRateTooltip: "DB-backed drop rate",
                    rawDropRateText: "0.005%",
                    rawDropRateTooltip: "DB-backed drop rate",
                    normalizedDropRateText: "0.005%",
                    normalizedDropRateTooltip: "DB-backed drop rate",
                    catchMethods: ["rod"],
                  },
                ],
              },
            ],
          },
          {
            slotIdx: 6,
            label: "Harpoon",
            fillColor: "#c7f9f1",
            strokeColor: "#2dd4bf",
            textColor: "#083344",
            dropRateText: "100%",
            dropRateSourceKind: "database",
            dropRateTooltip: "DB Harpoon group share",
            presenceText: "DB presence · Harpoon subgroup",
            presenceSourceKind: "database",
            presenceTooltip: "DB presence · Harpoon subgroup 10901 · Source: item_sub_group_table",
            rawDropRateText: "100%",
            rawDropRateTooltip: "DB Harpoon group share",
            normalizedDropRateText: "100%",
            normalizedDropRateTooltip: "DB Harpoon group share",
            conditionText: "Mastery 200-699 · Mastery 700-1199 · Mastery 1200+ · Fishing Level Guru 1+",
            conditionTooltip:
              "Mastery 200-699 | Mastery 700-1199 | Mastery 1200+ | Fishing Level Guru 1+",
            catchMethods: ["harpoon"],
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
            rawDropRateText: "80%",
            rawDropRateTooltip: "DB-backed drop rate",
            normalizedDropRateText: "80%",
            normalizedDropRateTooltip: "DB-backed drop rate",
            catchMethods: ["rod"],
          },
          {
            slotIdx: 6,
            groupLabel: "Harpoon",
            label: "Mako Shark",
            iconUrl: "/i/mako-shark.png",
            iconGradeTone: "rare",
            fillColor: "#c7f9f1",
            strokeColor: "#2dd4bf",
            textColor: "#083344",
            dropRateText: "27.5%",
            dropRateSourceKind: "database",
            dropRateTooltip: "Harpoon in-group rate",
            rawDropRateText: "27.5%",
            rawDropRateTooltip: "Harpoon in-group rate",
            normalizedDropRateText: "27.5%",
            normalizedDropRateTooltip: "Harpoon in-group rate",
            presenceText: "Community confirmed×1 · General group",
            presenceTooltip: "Community confirmed×1 · General group 9001 · source community_zone_fish_support",
            catchMethods: ["harpoon"],
          },
        ],
      },
    },
  );

  assert.equal(viewModel.descriptor.title, "Valencia Sea - Depth 5");
  assert.equal(viewModel.descriptor.titleIcon, "inspect-fill");
  assert.equal(viewModel.descriptor.statusIcon, "information-circle");
  assert.deepEqual(viewModel.panes.map((pane) => pane.id), ["zone", "territory", "trade"]);
  assert.equal(viewModel.activePaneId, "territory");
  assert.deepEqual(
    viewModel.panes.find((pane) => pane.id === "zone")?.sections.map((section) => section.kind),
    ["facts", "zone-loot"],
  );
  assert.equal(
    viewModel.panes.find((pane) => pane.id === "zone")?.sections[1]?.title,
    "Catch Profile",
  );
  assert.match(
    viewModel.panes.find((pane) => pane.id === "zone")?.sections[1]?.dataQualityNote || "",
    /Expected loot uses average session casts/,
  );
  assert.equal(
    viewModel.panes.find((pane) => pane.id === "zone")?.sections[1]?.profiles?.[0]?.groups?.[0]?.rows?.[0]?.label,
    "Sea Eel",
  );
  assert.equal(
    viewModel.panes.find((pane) => pane.id === "zone")?.sections[1]?.profiles?.[0]?.groups?.[0]?.conditionText,
    "Default",
  );
  assert.equal(
    viewModel.panes.find((pane) => pane.id === "zone")?.sections[1]?.profiles?.[0]?.groups?.[0]?.presenceSourceKind,
    "community",
  );
  assert.equal(
    viewModel.panes.find((pane) => pane.id === "zone")?.sections[1]?.profiles?.[0]?.groups?.[0]?.dropRateText,
    "80%",
  );
  assert.deepEqual(
    viewModel.panes.find((pane) => pane.id === "zone")?.sections[1]?.profiles?.map((profile) => profile.method),
    ["rod", "harpoon"],
  );
  assert.equal(
    viewModel.panes.find((pane) => pane.id === "zone")?.sections[1]?.profiles?.[1]?.groups?.[0]?.rows?.[0]?.dropRateText,
    "27.5%",
  );
  assert.equal(
    viewModel.panes.find((pane) => pane.id === "zone")?.sections[1]?.profiles?.[1]?.groups?.[0]?.presenceSourceKind,
    "database",
  );
  assert.deepEqual(
    viewModel.panes.find((pane) => pane.id === "territory")?.sections[0].facts,
    [
      {
        key: "resources",
        icon: "hover-resources",
        label: "Resources",
        value: "(RG212|Arehaza)",
      },
    ],
  );
  assert.deepEqual(
    viewModel.panes.find((pane) => pane.id === "trade")?.sections[0].facts,
    [
      {
        key: "origin",
        icon: "trade-origin",
        label: "Origin",
        value: "(R430|Hakoven Islands)",
      },
    ],
  );
});

test("buildInfoViewModel switches zone loot rates from the normalize rates signal", () => {
  const zoneLootSummary = {
    available: true,
    groups: [
      {
        slotIdx: 2,
        label: "Rare",
        dropRateText: "selected group",
        rawDropRateText: "12%",
        rawDropRateTooltip: "raw group",
        normalizedDropRateText: "50%",
        normalizedDropRateTooltip: "normalized group",
        catchMethods: ["rod"],
        conditionOptions: [
          {
            conditionText: "Default",
            dropRateText: "selected condition",
            rawDropRateText: "12%",
            rawDropRateTooltip: "raw condition",
            normalizedDropRateText: "50%",
            normalizedDropRateTooltip: "normalized condition",
            active: true,
            speciesRows: [
              {
                slotIdx: 2,
                groupLabel: "Rare",
                label: "Grunt",
                dropRateText: "selected species",
                rawDropRateText: "40%",
                rawDropRateTooltip: "raw species",
                normalizedDropRateText: "100%",
                normalizedDropRateTooltip: "normalized species",
                catchMethods: ["rod"],
              },
            ],
          },
        ],
      },
    ],
    speciesRows: [],
  };
  const viewModelFor = (normalizeRates) => buildInfoViewModel(
    {
      _map_ui: { windowUi: { settings: { normalizeRates } } },
      _map_runtime: {
        selection: { pointKind: "clicked", layerSamples: [] },
        catalog: { layers: [] },
      },
    },
    {
      zoneLootStatus: "loaded",
      zoneLootSummary,
    },
  );
  const zoneLootGroup = (viewModel) => viewModel.panes
    .find((pane) => pane.id === "zone")
    ?.sections.find((section) => section.kind === "zone-loot")
    ?.profiles[0]
    ?.groups[0];

  const normalizedGroup = zoneLootGroup(viewModelFor(true));
  const rawGroup = zoneLootGroup(viewModelFor(false));

  assert.equal(normalizedGroup.dropRateText, "50%");
  assert.equal(normalizedGroup.dropRateTooltip, "normalized condition");
  assert.equal(normalizedGroup.rows[0].dropRateText, "100%");
  assert.equal(normalizedGroup.rows[0].dropRateTooltip, "normalized species");
  assert.equal(rawGroup.dropRateText, "12%");
  assert.equal(rawGroup.dropRateTooltip, "raw condition");
  assert.equal(rawGroup.rows[0].dropRateText, "40%");
  assert.equal(rawGroup.rows[0].dropRateTooltip, "raw species");
});

test("buildInfoViewModel exposes clicked ranking samples as the first pane", () => {
  globalThis.window = globalThis.window || {};
  globalThis.window.__fishystuffResolveFishItemIconUrl = (itemId) => `/items/${itemId}.webp`;
  const viewModel = buildInfoViewModel(
    {
      _map_runtime: {
        selection: {
          pointKind: "clicked",
          pointLabel: "",
          layerSamples: [],
          pointSamples: [
            {
              fishId: 20,
              sampleCount: 1,
              lastTsUtc: 1_700_200_000,
              zoneRgbs: [0x123456, 0x654321],
              fullZoneRgbs: [],
            },
            {
              fishId: 10,
              sampleCount: 4,
              lastTsUtc: 1_700_000_000,
              zoneRgbs: [0x39e58d],
              fullZoneRgbs: [0x39e58d],
            },
          ],
        },
        catalog: {
          fish: [
            { fishId: 10, itemId: 900010, name: "Sea Eel", grade: "general" },
            { fishId: 20, itemId: 900020, name: "Mako Shark", grade: "rare" },
          ],
          layers: [],
        },
      },
    },
    {
      zoneCatalog: [
        { zoneRgb: 0x39e58d, name: "Velia Coast" },
        { zoneRgb: 0x123456, name: "Demi River" },
        { zoneRgb: 0x654321, name: "Balenos River" },
      ],
    },
  );

  assert.deepEqual(viewModel.panes.map((pane) => pane.id), ["samples"]);
  assert.equal(viewModel.activePaneId, "samples");
  assert.equal(viewModel.activePane.sections[0].kind, "point-samples");
  assert.deepEqual(
    viewModel.activePane.sections[0].rows.map((row) => [row.fishName, row.sampleCount, row.zoneKind]),
    [
      ["Sea Eel", 4, "full"],
      ["Mako Shark", 1, "partial"],
    ],
  );
  assert.deepEqual(
    viewModel.activePane.sections[0].rows[1].zones.map((zone) => zone.name),
    ["Demi River", "Balenos River"],
  );
});

test("buildInfoViewModel lets zone loot condition selection switch branch rows", () => {
  const viewModel = buildInfoViewModel(
    {
      _map_runtime: {
        selection: {
          pointKind: "clicked",
          pointLabel: "Velia Event",
          layerSamples: [],
        },
      },
    },
    {
      zoneLootStatus: "loaded",
      zoneLootConditionSelection: {
        "2:Rare": 1,
      },
      zoneLootSummary: {
        available: true,
        profileLabel: "Calculator defaults",
        groups: [
          {
            slotIdx: 2,
            label: "Rare",
            dropRateText: "1%",
            rawDropRateText: "1%",
            rawDropRateTooltip: "stale parent lineage",
            normalizedDropRateText: "1%",
            normalizedDropRateTooltip: "stale parent lineage",
            catchMethods: ["rod"],
            conditionText: "Default",
            conditionOptions: [
              {
                conditionText: "Default",
                dropRateText: "1%",
                dropRateSourceKind: "database",
                dropRateTooltip: "stale parent lineage",
                rawDropRateText: "1%",
                rawDropRateTooltip: "stale parent lineage",
                normalizedDropRateText: "1%",
                normalizedDropRateTooltip: "stale parent lineage",
                active: true,
                speciesRows: [
                  {
                    slotIdx: 2,
                    groupLabel: "Rare",
                    label: "Grunt",
                    dropRateText: "100%",
                    dropRateTooltip: "DB 100% · main group 10990 -> subgroup 10990 · option 1",
                    rawDropRateText: "100%",
                    rawDropRateTooltip: "DB 100% · main group 10990 -> subgroup 10990 · option 1",
                    normalizedDropRateText: "100%",
                    normalizedDropRateTooltip: "DB 100% · main group 10990 -> subgroup 10990 · option 1",
                    catchMethods: ["rod"],
                  },
                ],
              },
              {
                conditionText: "Fishing Level Guru 1+",
                dropRateText: "1%",
                dropRateSourceKind: "database",
                dropRateTooltip: "stale parent lineage",
                rawDropRateText: "1%",
                rawDropRateTooltip: "stale parent lineage",
                normalizedDropRateText: "1%",
                normalizedDropRateTooltip: "stale parent lineage",
                active: false,
                speciesRows: [
                  {
                    slotIdx: 2,
                    groupLabel: "Rare",
                    label: "Mystical Fish",
                    dropRateText: "0.005%",
                    dropRateTooltip: "DB 0.005% · main group 10990 -> subgroup 11152 · option 0",
                    rawDropRateText: "0.005%",
                    rawDropRateTooltip: "DB 0.005% · main group 10990 -> subgroup 11152 · option 0",
                    normalizedDropRateText: "0.005%",
                    normalizedDropRateTooltip: "DB 0.005% · main group 10990 -> subgroup 11152 · option 0",
                    catchMethods: ["rod"],
                  },
                ],
              },
            ],
          },
        ],
        speciesRows: [],
      },
    },
  );
  const group = viewModel.panes.find((pane) => pane.id === "zone")?.sections[0]?.profiles?.[0]?.groups?.[0];

  assert.equal(group.conditionText, "Fishing Level Guru 1+");
  assert.equal(group.dropRateTooltip, "main group 10990 -> subgroup 11152 · option 0");
  assert.equal(group.rows[0].label, "Mystical Fish");
  assert.equal(group.conditionOptionIndex, 1);
  assert.equal(group.conditionOptionKey, "2:Rare");
});

test("buildInfoViewModel exposes selected waypoint detail sections as a landmark pane", () => {
  const viewModel = buildInfoViewModel({
    _map_runtime: {
      selection: {
        pointKind: "waypoint",
        pointLabel: "Chunsu",
        layerSamples: [
          {
            layerId: "trade_npcs",
            layerName: "Trade NPCs",
            kind: "waypoint",
            detailSections: [
              {
                id: "trade-npc",
                kind: "facts",
                title: "Trade NPC",
                facts: [
                  { key: "trade_npc", label: "NPC", value: "Chunsu", icon: "trade-origin" },
                  { key: "npc_key", label: "NPC Key", value: "1", icon: "information-circle" },
                ],
              },
            ],
          },
          {
            layerId: "zone_mask",
            rgbU32: 0x39e58d,
            rgb: [57, 229, 141],
            detailSections: [detailSectionFact("zone", "Zone", "Velia Coast", "hover-zone")],
          },
        ],
      },
      catalog: {
        layers: [
          { layerId: "zone_mask", displayOrder: 20 },
          { layerId: "trade_npcs", displayOrder: 42 },
        ],
      },
    },
  });

  assert.equal(viewModel.descriptor.title, "Chunsu");
  assert.deepEqual(viewModel.panes.map((pane) => pane.id), ["landmark", "zone"]);
  assert.equal(viewModel.activePaneId, "landmark");
  assert.equal(viewModel.activePane.sections[0].title, "Landmark");
  assert.deepEqual(
    viewModel.activePane.sections[0].facts.map((fact) => [fact.key, fact.label, fact.value, fact.icon]),
    [
      ["landmark:trade_npcs:trade_npc", "NPC", "Chunsu", "trade-origin"],
      ["landmark:trade_npcs:npc_key", "NPC Key", "1", "information-circle"],
    ],
  );
});

test("buildInfoViewModel titles selected bookmarks from the details target identity", () => {
  const viewModel = buildInfoViewModel({
    _map_runtime: {
      selection: {
        detailsTarget: {
          elementKind: "bookmark",
          worldX: 1234,
          worldZ: 5678,
          pointKind: "bookmark",
          pointLabel: "Saved Hotspot",
        },
        pointKind: "bookmark",
        pointLabel: "Serendia - Terrain",
        layerSamples: [
          {
            layerId: "zone_mask",
            rgbU32: 0x39e58d,
            rgb: [57, 229, 141],
            detailSections: [detailSectionFact("zone", "Zone", "Serendia - Terrain", "hover-zone")],
          },
        ],
      },
      catalog: {
        layers: [{ layerId: "zone_mask", displayOrder: 20 }],
      },
    },
  });

  assert.equal(viewModel.descriptor.title, "Saved Hotspot");
  assert.equal(viewModel.descriptor.statusText, "Saved Hotspot");
});

test("patchTouchesInfoSignals stays narrow to selection, pane tab, rate display, and runtime layer inputs", () => {
  assert.equal(
    patchTouchesInfoSignals({
      _map_runtime: { selection: {} },
    }),
    true,
  );
  assert.equal(
    patchTouchesInfoSignals({
      _map_runtime: { catalog: { layers: [] } },
    }),
    true,
  );
  assert.equal(
    patchTouchesInfoSignals({
      _map_runtime: { catalog: { fish: [] } },
    }),
    true,
  );
  assert.equal(
    patchTouchesInfoSignals({
      _map_ui: { windowUi: { zoneInfo: { tab: "trade" } } },
    }),
    true,
  );
  assert.equal(
    patchTouchesInfoSignals({
      _map_ui: { windowUi: { settings: { normalizeRates: false } } },
    }),
    true,
  );
  assert.equal(
    patchTouchesInfoSignals({
      _map_ui: { search: { open: true } },
    }),
    false,
  );
});

test("buildInfoViewModel falls back to Details when no layer label is available", () => {
  const viewModel = buildInfoViewModel({
    _map_runtime: {
      selection: {
        pointKind: "clicked",
        worldX: 0,
        worldZ: 0,
        layerSamples: [],
      },
    },
  });

  assert.equal(viewModel.descriptor.title, "Details");
  assert.equal(viewModel.descriptor.titleIcon, "inspect-fill");
  assert.equal(viewModel.descriptor.statusIcon, "information-circle");
});
installMapTestI18n();
