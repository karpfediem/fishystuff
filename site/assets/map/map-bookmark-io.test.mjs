import test from "node:test";
import assert from "node:assert/strict";
import { installMapTestI18n } from "./test-i18n.js";

import {
  buildBookmarkExportMessage,
  buildBookmarkImportMessage,
  buildBookmarkSelectionCopyMessage,
  mergeImportedBookmarks,
  parseImportedBookmarks,
  serializeBookmarksForExport,
} from "./map-bookmark-io.js";

installMapTestI18n();

test("bookmark io serializes bookmarks to WorldmapBookMark XML", () => {
  const xml = serializeBookmarksForExport([
    { id: "a", label: "Alpha", worldX: 100, worldZ: 200 },
  ]);
  assert.match(xml, /<WorldmapBookMark>/);
  assert.match(xml, /BookMarkName="1: Alpha"/);
  assert.match(xml, /PosX="100.0"/);
  assert.match(xml, /PosZ="200.0"/);
});

test("bookmark io parses exported XML back into bookmarks", () => {
  const parsed = parseImportedBookmarks(
    '<WorldmapBookMark><BookMark BookMarkName="1: Alpha" PosX="100.0" PosY="0.0" PosZ="200.0" /></WorldmapBookMark>',
    { idFactory: () => "bookmark-imported" },
  );
  assert.deepEqual(JSON.parse(JSON.stringify(parsed)), [
    {
      id: "bookmark-imported",
      label: "Alpha",
      worldX: 100,
      worldZ: 200,
    },
  ]);
});

test("bookmark io merges imports while skipping duplicates by label and coordinates", () => {
  const merged = mergeImportedBookmarks(
    [{ id: "a", label: "Alpha", worldX: 100, worldZ: 200 }],
    [
      { id: "b", label: "Alpha", worldX: 100, worldZ: 200 },
      { id: "c", label: "Bravo", worldX: 300, worldZ: 400 },
    ],
  );
  assert.deepEqual(
    JSON.parse(JSON.stringify(merged.map((bookmark) => bookmark.id))),
    ["a", "c"],
  );
});

test("bookmark io status messages stay user-facing", () => {
  assert.equal(buildBookmarkSelectionCopyMessage(2), "Copied XML for 2 bookmarks.");
  assert.equal(buildBookmarkExportMessage(1, 1), "Exported 1 selected bookmark.");
  assert.equal(
    buildBookmarkImportMessage(2, 1),
    "Imported 2 bookmarks. Skipped 1 duplicate.",
  );
});
