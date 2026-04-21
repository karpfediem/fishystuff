import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import { buildShellContentTree } from "./build-shell-pages.mjs";
import { LANGUAGE_CONFIG } from "./language-config.mjs";

test("buildShellContentTree copies tracked pages and generates shell pages", () => {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), "fishystuff-shell-pages-"));
  const outRoot = path.join(rootDir, ".generated", "content");
  try {
    fs.mkdirSync(path.join(rootDir, "i18n"), { recursive: true });
    fs.writeFileSync(path.join(rootDir, "i18n", "shell-pages.json"), JSON.stringify({
      pages: [
        {
          id: "home",
          layout: "frontpage.shtml",
          author: "Karpfen",
          date: "2025-03-23T00:00:00",
          locales: {
            "en-US": { slug: "", title: "Home" },
            "de-DE": { slug: "", title: "Startseite" },
          },
        },
        {
          id: "profile",
          layout: "profile.shtml",
          author: "Karpfen",
          date: "2026-04-19T00:00:00",
          translationKey: "profile",
          locales: {
            "en-US": { slug: "profile", title: "Profile" },
            "de-DE": { slug: "profil", title: "Profil" },
          },
        },
      ],
    }, null, 2));
    fs.mkdirSync(path.join(rootDir, "content", "en-US"), { recursive: true });
    fs.mkdirSync(path.join(rootDir, "content", "de-DE"), { recursive: true });
    fs.writeFileSync(path.join(rootDir, "content", "en-US", "community.smd"), "---\n.title = \"Community\",\n---\n");
    fs.mkdirSync(path.join(rootDir, "content", "en-US", "guides", "money"), { recursive: true });
    fs.writeFileSync(path.join(rootDir, "content", "en-US", "guides", "money", "index.smd"), [
      "---",
      '.title = "Fishing for Money",',
      '.layout = "guide_page.shtml",',
      '.custom = {',
      '  .og_image_asset = "money.png",',
      '},',
      "---",
      "",
      "Read the [Profile](/profile) page or join the [Community](/community).",
      "",
    ].join("\n"));
    fs.writeFileSync(path.join(rootDir, "content", "en-US", "guides", "money", "money.png"), "png");
    fs.writeFileSync(path.join(rootDir, "content", "en-US", "profile.smd"), "tracked profile");
    fs.writeFileSync(path.join(rootDir, "content", "de-DE", "log.smd"), "---\n.title = \"Fanglog\",\n---\n");

    buildShellContentTree({ config: LANGUAGE_CONFIG, rootDir, outRoot });

    const communitySource = fs.readFileSync(path.join(outRoot, "en-US", "community.smd"), "utf8");
    const deLogSource = fs.readFileSync(path.join(outRoot, "de-DE", "log.smd"), "utf8");
    const enIndexSource = fs.readFileSync(path.join(outRoot, "en-US", "index.smd"), "utf8");
    const deProfileSource = fs.readFileSync(path.join(outRoot, "de-DE", "profil.smd"), "utf8");

    assert.match(communitySource, /\.title = "Community",/);
    assert.match(communitySource, /\.og_image = "\/img\/embed\.png",/);
    assert.match(deLogSource, /\.title = "Fanglog",/);
    assert.match(deLogSource, /\.og_image = "\/de-DE\/embed\.png",/);
    assert.match(enIndexSource, /\.layout = "frontpage\.shtml"/);
    assert.match(enIndexSource, /\.og_image = "\/img\/embed\.png",/);
    assert.match(deProfileSource, /\.translation_key = "profile"/);
    assert.match(deProfileSource, /\.og_image = "\/de-DE\/embed\.png",/);
    const generatedFallback = fs.readFileSync(path.join(outRoot, "de-DE", "guides", "money", "index.smd"), "utf8");
    assert.match(generatedFallback, /\.translation_fallback = true,/);
    assert.match(generatedFallback, /\.canonical = "\/guides\/money\/",/);
    assert.match(generatedFallback, /\.translation_target_path = "site\/content\/de-DE\/guides\/money\/index\.smd",/);
    assert.match(generatedFallback, /\.translation_help_url = "\/de-DE\/community\/",/);
    assert.match(generatedFallback, /\.translation_source_file_url = "https:\/\/github\.com\/karpfediem\/fishystuff\/blob\/main\/site\/content\/en-US\/guides\/money\/index\.smd",/);
    assert.match(generatedFallback, /\.translation_create_file_url = "https:\/\/github\.com\/karpfediem\/fishystuff\/new\/main\?filename=site%2Fcontent%2Fde-DE%2Fguides%2Fmoney%2Findex\.smd",/);
    assert.doesNotMatch(generatedFallback, /\.og_image =/);
    assert.match(generatedFallback, /\[Profile\]\(\/profil\/\)/);
    assert.match(generatedFallback, /\[Community\]\(\/community\/\)/);
    assert.equal(fs.readFileSync(path.join(outRoot, "de-DE", "guides", "money", "money.png"), "utf8"), "png");
    assert.ok(!fs.existsSync(path.join(outRoot, "en-US", "profile.smd", "index.smd")));
    assert.notEqual(fs.readFileSync(path.join(outRoot, "en-US", "profile.smd"), "utf8"), "tracked profile");
  } finally {
    fs.rmSync(rootDir, { recursive: true, force: true });
  }
});

test("buildShellContentTree injects the sourced betta icon for beta builds", () => {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), "fishystuff-shell-pages-beta-"));
  const outRoot = path.join(rootDir, ".generated", "content");
  try {
    fs.mkdirSync(path.join(rootDir, "i18n"), { recursive: true });
    fs.writeFileSync(path.join(rootDir, "i18n", "shell-pages.json"), JSON.stringify({
      pages: [
        {
          id: "home",
          layout: "frontpage.shtml",
          author: "Karpfen",
          date: "2025-03-23T00:00:00",
          locales: {
            "en-US": { slug: "", title: "Home" },
            "de-DE": { slug: "", title: "Startseite" },
          },
        },
      ],
    }, null, 2));
    fs.mkdirSync(path.join(rootDir, "content", "en-US"), { recursive: true });
    fs.mkdirSync(path.join(rootDir, "content", "de-DE"), { recursive: true });

    buildShellContentTree({
      config: LANGUAGE_CONFIG,
      rootDir,
      outRoot,
      env: {
        FISHYSTUFF_PUBLIC_SITE_BASE_URL: "https://beta.fishystuff.fish",
      },
    });

    const enIndexSource = fs.readFileSync(path.join(outRoot, "en-US", "index.smd"), "utf8");
    assert.match(
      enIndexSource,
      /\.brand_logo_url = "https:\/\/cdn\.beta\.fishystuff\.fish\/images\/items\/00820996\.webp",/,
    );
    assert.match(
      enIndexSource,
      /\.brand_logo_nav_url = "https:\/\/cdn\.beta\.fishystuff\.fish\/images\/items\/00820996\.webp",/,
    );
    assert.match(enIndexSource, /\.brand_logo_nav_srcset = "",/);
  } finally {
    fs.rmSync(rootDir, { recursive: true, force: true });
  }
});
