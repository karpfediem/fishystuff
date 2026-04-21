import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import { buildRobots, buildSitemapUrl, renderRobotsTxt } from "./build-robots.mjs";

test("buildSitemapUrl resolves the sitemap against the public site host", () => {
  assert.equal(buildSitemapUrl("https://fishystuff.fish"), "https://fishystuff.fish/sitemap.xml");
  assert.equal(buildSitemapUrl("https://beta.fishystuff.fish/"), "https://beta.fishystuff.fish/sitemap.xml");
});

test("renderRobotsTxt allows crawling and advertises the sitemap", () => {
  assert.equal(
    renderRobotsTxt({ hostUrl: "https://fishystuff.fish" }),
    [
      "User-agent: *",
      "Allow: /",
      "Sitemap: https://fishystuff.fish/sitemap.xml",
      "",
    ].join("\n"),
  );
});

test("buildRobots can derive the host from zine config and write the output", () => {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), "fishystuff-build-robots-"));
  try {
    const zineConfigPath = path.join(rootDir, "zine.ziggy");
    const outPath = path.join(rootDir, "out", "robots.txt");
    fs.writeFileSync(zineConfigPath, '.host_url = "https://example.test",\n', "utf8");

    const robotsTxt = buildRobots({
      zineConfigPath,
      outPath,
    });

    assert.equal(
      robotsTxt,
      [
        "User-agent: *",
        "Allow: /",
        "Sitemap: https://example.test/sitemap.xml",
        "",
      ].join("\n"),
    );
    assert.equal(fs.readFileSync(outPath, "utf8"), robotsTxt);
  } finally {
    fs.rmSync(rootDir, { recursive: true, force: true });
  }
});
