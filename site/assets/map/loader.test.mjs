import test from "node:test";
import assert from "node:assert/strict";

globalThis.__fishystuffLoaderAutoStart = false;
const {
  buildSearchMatches,
  normalizeZoneCatalog,
  normalizeWindowUiState,
  parseZoneRgbSearch,
  parseWindowUiState,
  renderSearchSelection,
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
    search: { open: 0, collapsed: "yes", x: 42.8, y: "13" },
    settings: { open: true, collapsed: false, x: null, y: null },
    zoneInfo: { open: true, collapsed: false, x: undefined, y: 5.2 },
    layers: { open: false, collapsed: 0, x: "bad", y: 99.9 },
  });

  assert.deepEqual(JSON.parse(serialized), {
    search: { open: true, collapsed: true, x: 43, y: 13 },
    settings: { open: true, collapsed: false, x: null, y: null },
    zoneInfo: { open: true, collapsed: false, x: null, y: 5 },
    layers: { open: false, collapsed: false, x: null, y: 100 },
  });
});
