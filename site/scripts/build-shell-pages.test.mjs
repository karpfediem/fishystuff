import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import { buildShellContentTree } from "./build-shell-pages.mjs";
import { LANGUAGE_CONFIG } from "./language-config.mjs";

function writePngStub(filePath, width, height) {
  const buffer = Buffer.alloc(24);
  buffer.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a], 0);
  buffer.writeUInt32BE(13, 8);
  buffer.write("IHDR", 12, "ascii");
  buffer.writeUInt32BE(width, 16);
  buffer.writeUInt32BE(height, 20);
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, buffer);
}

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
    writePngStub(path.join(rootDir, "assets", "img", "embed.png"), 900, 300);
    writePngStub(path.join(rootDir, "assets", "de-DE", "embed.png"), 900, 300);
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

    buildShellContentTree({ config: LANGUAGE_CONFIG, rootDir, outRoot, env: {} });

    const communitySource = fs.readFileSync(path.join(outRoot, "en-US", "community.smd"), "utf8");
    const deLogSource = fs.readFileSync(path.join(outRoot, "de-DE", "log.smd"), "utf8");
    const enIndexSource = fs.readFileSync(path.join(outRoot, "en-US", "index.smd"), "utf8");
    const deProfileSource = fs.readFileSync(path.join(outRoot, "de-DE", "profil.smd"), "utf8");

    assert.match(communitySource, /\.title = "Community",/);
    assert.match(communitySource, /\.document_title = "Community \| Fishy Stuff",/);
    assert.match(communitySource, /\.resolved_description = "Fishy Stuff: Fishing Guides and Tools for Black Desert",/);
    assert.match(communitySource, /\.hreflang_links_html = "<link rel=\\"alternate\\" hreflang=\\"en-US\\" href=\\"https:\/\/fishystuff\.fish\/community\/\\">\\n<link rel=\\"alternate\\" hreflang=\\"de-DE\\" href=\\"https:\/\/fishystuff\.fish\/de-DE\/community\/\\">\\n<link rel=\\"alternate\\" hreflang=\\"x-default\\" href=\\"https:\/\/fishystuff\.fish\/community\/\\">",/);
    assert.match(communitySource, /\.og_locale = "en_US",/);
    assert.match(communitySource, /\.og_locale_alternate_html = "<meta property=\\"og:locale:alternate\\" content=\\"de_DE\\">",/);
    assert.match(communitySource, /\.og_image_alt = "Community",/);
    assert.match(communitySource, /\.og_image_type = "image\/png",/);
    assert.match(communitySource, /\.og_image_width = "900",/);
    assert.match(communitySource, /\.og_image_height = "300",/);
    assert.match(communitySource, /\.og_image = "\/img\/embed\.png",/);
    assert.match(deLogSource, /\.title = "Fanglog",/);
    assert.match(deLogSource, /\.document_title = "Fanglog \| Fishy Stuff",/);
    assert.match(deLogSource, /\.og_locale = "de_DE",/);
    assert.match(deLogSource, /\.og_locale_alternate_html = "",/);
    assert.match(deLogSource, /\.og_image_type = "image\/png",/);
    assert.match(deLogSource, /\.og_image = "\/de-DE\/embed\.png",/);
    assert.match(enIndexSource, /\.layout = "frontpage\.shtml"/);
    assert.match(enIndexSource, /\.document_title = "Home \| Fishy Stuff",/);
    assert.match(enIndexSource, /\.og_image = "\/img\/embed\.png",/);
    assert.match(deProfileSource, /\.translation_key = "profile"/);
    assert.match(deProfileSource, /\.document_title = "Profil \| Fishy Stuff",/);
    assert.match(deProfileSource, /\.og_locale = "de_DE",/);
    assert.match(deProfileSource, /\.og_locale_alternate_html = "<meta property=\\"og:locale:alternate\\" content=\\"en_US\\">",/);
    assert.match(deProfileSource, /\.og_image_type = "image\/png",/);
    assert.match(deProfileSource, /\.og_image = "\/de-DE\/embed\.png",/);
    const generatedFallback = fs.readFileSync(path.join(outRoot, "de-DE", "guides", "money", "index.smd"), "utf8");
    assert.match(generatedFallback, /\.translation_fallback = true,/);
    assert.match(generatedFallback, /\.canonical = "\/guides\/money\/",/);
    assert.match(generatedFallback, /\.translation_target_path = "site\/content\/de-DE\/guides\/money\/index\.smd",/);
    assert.match(generatedFallback, /\.translation_help_url = "\/de-DE\/community\/",/);
    assert.match(generatedFallback, /\.translation_source_file_url = "https:\/\/github\.com\/karpfediem\/fishystuff\/blob\/main\/site\/content\/en-US\/guides\/money\/index\.smd",/);
    assert.match(generatedFallback, /\.translation_create_file_url = "https:\/\/github\.com\/karpfediem\/fishystuff\/new\/main\?filename=site%2Fcontent%2Fde-DE%2Fguides%2Fmoney%2Findex\.smd",/);
    assert.doesNotMatch(generatedFallback, /\.og_image =/);
    assert.match(generatedFallback, /\.og_image_type = "image\/png",/);
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
        FISHYSTUFF_DEPLOYMENT_ENVIRONMENT: "beta",
        FISHYSTUFF_PUBLIC_SITE_BASE_URL: "https://beta.fishystuff.fish",
      },
    });

    const enIndexSource = fs.readFileSync(path.join(outRoot, "en-US", "index.smd"), "utf8");
    assert.match(
      enIndexSource,
      /\.brand_logo_url = "https:\/\/cdn\.beta\.fishystuff\.fish\/images\/items\/00820996\.webp",/,
    );
    assert.match(enIndexSource, /\.document_title = "Home \| Fishy Stuff \(Beta\)",/);
    assert.match(enIndexSource, /\.hreflang_links_html = "<link rel=\\"alternate\\" hreflang=\\"en-US\\" href=\\"https:\/\/beta\.fishystuff\.fish\/\\">\\n<link rel=\\"alternate\\" hreflang=\\"de-DE\\" href=\\"https:\/\/beta\.fishystuff\.fish\/de-DE\/\\">\\n<link rel=\\"alternate\\" hreflang=\\"x-default\\" href=\\"https:\/\/beta\.fishystuff\.fish\/\\">",/);
    assert.match(
      enIndexSource,
      /\.brand_logo_nav_url = "https:\/\/cdn\.beta\.fishystuff\.fish\/images\/items\/00820996\.webp",/,
    );
    assert.match(enIndexSource, /\.brand_logo_nav_srcset = "",/);
  } finally {
    fs.rmSync(rootDir, { recursive: true, force: true });
  }
});

test("buildShellContentTree derives document titles from the deployment name", () => {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), "fishystuff-shell-pages-preview-"));
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
        FISHYSTUFF_DEPLOYMENT_ENVIRONMENT: "preview-east",
      },
    });

    const enIndexSource = fs.readFileSync(path.join(outRoot, "en-US", "index.smd"), "utf8");
    assert.match(enIndexSource, /\.document_title = "Home \| Fishy Stuff \(Preview East\)",/);
  } finally {
    fs.rmSync(rootDir, { recursive: true, force: true });
  }
});
