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
    fs.writeFileSync(path.join(rootDir, "content", "en-US", "profile.smd"), "tracked profile");
    fs.writeFileSync(path.join(rootDir, "content", "de-DE", "log.smd"), "---\n.title = \"Fanglog\",\n---\n");

    buildShellContentTree({ config: LANGUAGE_CONFIG, rootDir, outRoot });

    assert.equal(fs.readFileSync(path.join(outRoot, "en-US", "community.smd"), "utf8"), "---\n.title = \"Community\",\n---\n");
    assert.equal(fs.readFileSync(path.join(outRoot, "de-DE", "log.smd"), "utf8"), "---\n.title = \"Fanglog\",\n---\n");
    assert.match(fs.readFileSync(path.join(outRoot, "en-US", "index.smd"), "utf8"), /\.layout = "frontpage\.shtml"/);
    assert.match(fs.readFileSync(path.join(outRoot, "de-DE", "profil.smd"), "utf8"), /\.translation_key = "profile"/);
    assert.ok(!fs.existsSync(path.join(outRoot, "en-US", "profile.smd", "index.smd")));
    assert.notEqual(fs.readFileSync(path.join(outRoot, "en-US", "profile.smd"), "utf8"), "tracked profile");
  } finally {
    fs.rmSync(rootDir, { recursive: true, force: true });
  }
});
