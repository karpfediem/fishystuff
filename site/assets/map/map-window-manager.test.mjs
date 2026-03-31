import test from "node:test";
import assert from "node:assert/strict";

import {
  buildWindowUiEntryPatch,
  clampManagedWindowPosition,
} from "./map-window-manager.js";

test("clampManagedWindowPosition keeps windows within the shell bounds", () => {
  assert.deepEqual(
    clampManagedWindowPosition(
      { width: 640, height: 480 },
      { width: 240, height: 320 },
      56,
      900,
      900,
    ),
    { x: 400, y: 424 },
  );

  assert.deepEqual(
    clampManagedWindowPosition(
      { width: 640, height: 480 },
      { width: 240, height: 320 },
      56,
      -50,
      -30,
    ),
    { x: 0, y: 0 },
  );
});

test("buildWindowUiEntryPatch normalizes search collapse and coordinates", () => {
  const patch = buildWindowUiEntryPatch(
    {
      search: { open: true, collapsed: false, x: null, y: null },
      settings: { open: true, collapsed: false, x: null, y: null, autoAdjustView: true },
      zoneInfo: { open: true, collapsed: false, x: null, y: null, tab: "" },
      layers: { open: true, collapsed: false, x: null, y: null },
      bookmarks: { open: false, collapsed: false, x: null, y: null },
    },
    "search",
    { collapsed: true, x: 12.8, y: "33" },
  );

  assert.deepEqual(patch, {
    _map_ui: {
      windowUi: {
        search: {
          open: true,
          collapsed: false,
          x: 13,
          y: 33,
        },
      },
    },
  });
});
