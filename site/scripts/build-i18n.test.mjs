import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

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
  assert.deepEqual(manifest["/community/"], {
    "en-US": "/community/",
    "de-DE": "/de-DE/community/",
  });
  assert.deepEqual(manifest["/guides/money/"], {
    "en-US": "/guides/money/",
    "de-DE": "/de-DE/guides/money/",
  });
  assert.deepEqual(manifest["/map/"], {
    "en-US": "/map/",
    "de-DE": "/de-DE/karte/",
  });
  assert.deepEqual(manifest["/karte/"], {
    "en-US": "/map/",
    "de-DE": "/de-DE/karte/",
  });
  assert.deepEqual(manifest["/calculator/"], {
    "en-US": "/calculator/",
    "de-DE": "/de-DE/rechner/",
  });
  assert.deepEqual(manifest["/rechner/"], {
    "en-US": "/calculator/",
    "de-DE": "/de-DE/rechner/",
  });
  assert.deepEqual(manifest["/dex/"], {
    "en-US": "/dex/",
    "de-DE": "/de-DE/dex/",
  });
  assert.deepEqual(manifest["/profile/"], {
    "en-US": "/profile/",
    "de-DE": "/de-DE/profil/",
  });
  assert.deepEqual(manifest["/profil/"], {
    "en-US": "/profile/",
    "de-DE": "/de-DE/profil/",
  });
});

test("buildPageManifest groups localized slug variants by translation_key", () => {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), "fishystuff-build-i18n-"));
  try {
    fs.mkdirSync(path.join(rootDir, "content", "en-US"), { recursive: true });
    fs.mkdirSync(path.join(rootDir, "content", "de-DE"), { recursive: true });
    fs.writeFileSync(path.join(rootDir, "content", "en-US", "profile.smd"), `---
.title = "Profile",
.translation_key = "profile",
---
`);
    fs.writeFileSync(path.join(rootDir, "content", "de-DE", "profil.smd"), `---
.title = "Profil",
.translation_key = "profile",
---
`);

    const manifest = buildPageManifest(LANGUAGE_CONFIG, rootDir);

    assert.deepEqual(manifest["/profile/"], {
      "en-US": "/profile/",
      "de-DE": "/de-DE/profil/",
    });
    assert.deepEqual(manifest["/profil/"], {
      "en-US": "/profile/",
      "de-DE": "/de-DE/profil/",
    });
  } finally {
    fs.rmSync(rootDir, { recursive: true, force: true });
  }
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
