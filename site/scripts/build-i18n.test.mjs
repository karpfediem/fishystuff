import test from "node:test";
import assert from "node:assert/strict";

import {
  buildPageManifest,
  LANGUAGE_CONFIG,
  parseFluentMessages,
  resolveLocaleCatalogs,
} from "./build-i18n.mjs";

test("parseFluentMessages reads simple Fluent entries", () => {
  const catalog = parseFluentMessages(`
# comment
nav.guides = Guides
language.menu.label = Language
fishydex.details.spots.empty =
  No zone evidence is currently attached to this fish.
`);

  assert.deepEqual(catalog, {
    "nav.guides": "Guides",
    "language.menu.label": "Language",
    "fishydex.details.spots.empty": "No zone evidence is currently attached to this fish.",
  });
});

test("buildPageManifest maps locale variants and keeps english root paths", () => {
  const manifest = buildPageManifest(LANGUAGE_CONFIG);

  assert.deepEqual(manifest["/"], {
    "en-US": "/",
    "de-DE": "/de-DE/",
  });
  assert.deepEqual(manifest["/log/"], {
    "en-US": "/log/",
    "de-DE": "/de-DE/log/",
  });
  assert.deepEqual(manifest["/map/"], {
    "en-US": "/map/",
  });
});

test("resolveLocaleCatalogs fills missing locale keys from the default locale", () => {
  const catalogs = resolveLocaleCatalogs({
    "en-US": {
      "nav.guides": "Guides",
      "nav.map": "Map",
    },
    "de-DE": {
      "nav.guides": "Anleitungen",
    },
  }, "en-US");

  assert.deepEqual(catalogs, {
    "en-US": {
      "nav.guides": "Guides",
      "nav.map": "Map",
    },
    "de-DE": {
      "nav.guides": "Anleitungen",
      "nav.map": "Map",
    },
  });
});
