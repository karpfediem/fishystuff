import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  buildSitemap,
  buildSitemapRecords,
  extractDateStamp,
  readHostUrlFromZineConfig,
} from "./build-sitemap.mjs";
import { LANGUAGE_CONFIG } from "./language-config.mjs";

const repoSiteDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function writePage(filePath, frontmatterLines) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `---\n${frontmatterLines.join("\n")}\n---\n`, "utf8");
}

test("extractDateStamp normalizes full datetime strings to sitemap-safe dates", () => {
  assert.equal(extractDateStamp("2026-04-21T00:00:00"), "2026-04-21");
  assert.equal(extractDateStamp("2025-12-23:00:00"), "2025-12-23");
  assert.equal(extractDateStamp(""), "");
});

test("readHostUrlFromZineConfig reads the configured site host", () => {
  assert.equal(
    readHostUrlFromZineConfig(path.join(repoSiteDir, "zine.ziggy")),
    "https://fishystuff.fish",
  );
});

test("buildSitemapRecords includes real localized pages and excludes untranslated fallbacks", () => {
  const records = buildSitemapRecords({
    config: LANGUAGE_CONFIG,
    rootDir: repoSiteDir,
    hostUrl: "https://fishystuff.fish",
  });
  const paths = new Set(records.map((record) => record.path));

  assert.equal(paths.has("/"), true);
  assert.equal(paths.has("/map/"), true);
  assert.equal(paths.has("/de-DE/karte/"), true);
  assert.equal(paths.has("/log/"), true);
  assert.equal(paths.has("/de-DE/log/"), true);

  assert.equal(paths.has("/guides/active/"), false);
  assert.equal(paths.has("/de-DE/guides/money/"), false);

  const mapRecord = records.find((record) => record.path === "/map/");
  assert.ok(mapRecord);
  assert.deepEqual(
    mapRecord.alternates.map((alternate) => [alternate.hreflang, alternate.href]),
    [
      ["de-DE", "https://fishystuff.fish/de-DE/karte/"],
      ["en-US", "https://fishystuff.fish/map/"],
    ],
  );
  assert.equal(mapRecord.xDefaultHref, "https://fishystuff.fish/map/");
});

test("buildSitemap renders only indexable source and shell pages", () => {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), "fishystuff-build-sitemap-"));
  try {
    fs.mkdirSync(path.join(rootDir, "content", "en-US"), { recursive: true });
    fs.mkdirSync(path.join(rootDir, "content", "de-DE"), { recursive: true });
    fs.mkdirSync(path.join(rootDir, "i18n"), { recursive: true });
    fs.writeFileSync(path.join(rootDir, "zine.ziggy"), '.host_url = "https://example.test",\n', "utf8");
    fs.writeFileSync(path.join(rootDir, "i18n", "shell-pages.json"), JSON.stringify({
      pages: [
        {
          id: "home",
          layout: "frontpage.shtml",
          author: "Karpfen",
          date: "2025-03-23T00:00:00",
          updated: "2025-10-01T12:34:56",
          locales: {
            "en-US": { slug: "", title: "Home" },
            "de-DE": { slug: "", title: "Startseite" },
          },
        },
        {
          id: "map",
          layout: "map.shtml",
          author: "Karpfen",
          date: "2025-03-23T00:00:00",
          translationKey: "map",
          locales: {
            "en-US": { slug: "map", title: "Map" },
            "de-DE": { slug: "karte", title: "Karte" },
          },
        },
        {
          id: "draft-shell",
          layout: "page.shtml",
          author: "Karpfen",
          date: "2025-03-23T00:00:00",
          draft: true,
          locales: {
            "en-US": { slug: "draft-shell", title: "Draft Shell" },
          },
        },
      ],
    }, null, 2));

    writePage(path.join(rootDir, "content", "en-US", "log.smd"), [
      '.title = "Log",',
      '.date = @date("2025-03-23T00:00:00"),',
      '.draft = false,',
    ]);
    writePage(path.join(rootDir, "content", "de-DE", "log.smd"), [
      '.title = "Log",',
      '.date = @date("2025-03-24T00:00:00"),',
      '.draft = false,',
    ]);
    writePage(path.join(rootDir, "content", "en-US", "hidden.smd"), [
      '.title = "Hidden",',
      '.date = @date("2025-03-23T00:00:00"),',
      '.custom = { .noindex = true },',
      '.draft = false,',
    ]);
    writePage(path.join(rootDir, "content", "en-US", "draft-page.smd"), [
      '.title = "Draft",',
      '.date = @date("2025-03-23T00:00:00"),',
      '.draft = true,',
    ]);

    const outPath = path.join(rootDir, "out", "sitemap.xml");
    const { records, xml } = buildSitemap({
      config: LANGUAGE_CONFIG,
      rootDir,
      outPath,
    });
    const paths = new Set(records.map((record) => record.path));

    assert.equal(paths.has("/"), true);
    assert.equal(paths.has("/de-DE/"), true);
    assert.equal(paths.has("/map/"), true);
    assert.equal(paths.has("/de-DE/karte/"), true);
    assert.equal(paths.has("/log/"), true);
    assert.equal(paths.has("/de-DE/log/"), true);

    assert.equal(paths.has("/hidden/"), false);
    assert.equal(paths.has("/draft-page/"), false);
    assert.equal(paths.has("/draft-shell/"), false);

    assert.match(xml, /<loc>https:\/\/example\.test\/map\/<\/loc>/);
    assert.match(xml, /<lastmod>2025-10-01<\/lastmod>/);
    assert.match(xml, /hreflang="x-default" href="https:\/\/example\.test\/map\/"/);
    assert.equal(fs.existsSync(outPath), true);
  } finally {
    fs.rmSync(rootDir, { recursive: true, force: true });
  }
});
