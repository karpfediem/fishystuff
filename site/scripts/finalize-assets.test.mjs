import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import assert from "node:assert/strict";

import {
  buildCspPolicy,
  discoverAssetReferences,
  finalizeAssets,
  getAttribute,
  hashedAssetPath,
  rewriteHtml,
} from "./finalize-assets.mjs";

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

test("hashedAssetPath inserts content hash before extension", () => {
  assert.equal(
    hashedAssetPath("map/map-app-live-entry.js", "abc123"),
    "map/map-app-live-entry.abc123.js",
  );
  assert.equal(hashedAssetPath("css/site.css", "ff00"), "css/site.ff00.css");
});

test("discoverAssetReferences finds buildable script and stylesheet assets", () => {
  const refs = discoverAssetReferences(`
    <link rel="preload" href="/css/ignored.css">
    <link rel="stylesheet" href="/css/site.css">
    <script src="/runtime-config.js"></script>
    <script type="module" src="/map/map-app-live-entry.js"></script>
    <script src="https://example.test/external.js"></script>
  `);

  assert.deepEqual([...refs.entries()], [
    ["/map/map-app-live-entry.js", { kind: "script", module: true }],
    ["/css/site.css", { kind: "stylesheet" }],
  ]);
});

test("rewriteHtml adds SRI and a CSP meta tag", () => {
  const assetMap = new Map([
    [
      "/js/app.js",
      {
        kind: "script",
        url: "/js/app.abc123.js",
        integrity: "sha384-script",
      },
    ],
    [
      "/css/app.css",
      {
        kind: "stylesheet",
        url: "/css/app.abc123.css",
        integrity: "sha384-style",
      },
    ],
  ]);
  const html = rewriteHtml(`
    <html><head>
      <link rel="stylesheet" href="/css/app.css">
      <script src="/runtime-config.js"></script>
      <script>window.__inline = true;</script>
      <script type="module" src="/js/app.js"></script>
    </head><body></body></html>
  `, assetMap);

  assert.match(html, /data-fishystuff-generated-csp/);
  assert.match(html, /script-src[^"]*'sha256-/);
  assert.match(html, /href="\/css\/app\.abc123\.css"[^>]*integrity="sha384-style"/);
  assert.match(html, /src="\/js\/app\.abc123\.js"[^>]*integrity="sha384-script"/);
  assert.match(html, /<script src="\/runtime-config\.js"><\/script>/);
});

test("buildCspPolicy keeps script inline execution hash-based", () => {
  const policy = buildCspPolicy({ scriptHashes: ["sha256-test"] });

  assert.match(policy, /script-src 'self'/);
  assert.match(policy, /'sha256-test'/);
  assert.match(policy, /'unsafe-eval'/);
  assert.doesNotMatch(policy, /script-src[^;]*'unsafe-inline'/);
  assert.match(policy, /script-src-attr 'none'/);
});

test("getAttribute reads quoted and unquoted attributes", () => {
  assert.equal(getAttribute('<script src="/x.js" defer>', "src"), "/x.js");
  assert.equal(getAttribute("<script src='/x.js'>", "src"), "/x.js");
  assert.equal(getAttribute("<script src=/x.js>", "src"), "/x.js");
  assert.equal(getAttribute("<script>", "src"), null);
});

test("finalizeAssets minifies, hashes, writes source maps, and rewrites HTML", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "fishystuff-finalize-assets-"));
  try {
    await writeFile(
      path.join(root, "index.html"),
      `<!doctype html>
<html>
<head>
  <link rel="stylesheet" href="/css/app.css">
  <script src="/runtime-config.js"></script>
  <script>window.__inline = true;</script>
  <script type="module" src="/js/app.js"></script>
</head>
<body></body>
</html>
`,
      "utf8",
    );
    await writeFile(path.join(root, "runtime-config.js"), "window.__runtime = {};\n", "utf8");
    await mkdir(path.join(root, "css"), { recursive: true });
    await mkdir(path.join(root, "js"), { recursive: true });
    await writeFile(path.join(root, "css", "app.css"), ".example { color: red; }\n", "utf8");
    await writeFile(
      path.join(root, "js", "dep.js"),
      "export const message = 'hello from dependency';\n",
      "utf8",
    );
    await writeFile(
      path.join(root, "js", "app.js"),
      "import { message } from './dep.js';\nwindow.__message = message;\n",
      "utf8",
    );

    const result = await finalizeAssets({ rootDir: root });

    assert.equal(result.assetCount, 2);
    const manifest = JSON.parse(await readFile(path.join(root, "asset-manifest.json"), "utf8"));
    const scriptAsset = manifest.assets["/js/app.js"];
    const styleAsset = manifest.assets["/css/app.css"];
    assert.match(scriptAsset.url, /^\/js\/app\.[0-9a-f]{16}\.js$/);
    assert.match(scriptAsset.sourceMapUrl, /^\/js\/app\.[0-9a-f]{16}\.js\.map$/);
    assert.match(scriptAsset.integrity, /^sha384-/);
    assert.equal(scriptAsset.bundled, true);
    assert.match(styleAsset.url, /^\/css\/app\.[0-9a-f]{16}\.css$/);
    assert.match(styleAsset.sourceMapUrl, /^\/css\/app\.[0-9a-f]{16}\.css\.map$/);

    const html = await readFile(path.join(root, "index.html"), "utf8");
    assert.match(html, new RegExp(`src="${scriptAsset.url}"`));
    assert.match(html, new RegExp(`href="${styleAsset.url}"`));
    assert.ok(html.includes(`integrity="${scriptAsset.integrity}"`));
    assert.ok(html.includes(`integrity="${styleAsset.integrity}"`));
    assert.match(html, /src="\/runtime-config\.js"/);
    assert.match(html, /data-fishystuff-generated-csp/);
    assert.doesNotMatch(html, /\.map\b/);

    const bundled = await readFile(path.join(root, scriptAsset.url.slice(1)), "utf8");
    assert.match(bundled, /hello from dependency/);
    assert.match(
      bundled,
      new RegExp(`sourceMappingURL=${escapeRegExp(path.posix.basename(scriptAsset.sourceMapUrl))}`),
    );
    const css = await readFile(path.join(root, styleAsset.url.slice(1)), "utf8");
    assert.match(
      css,
      new RegExp(`sourceMappingURL=${escapeRegExp(path.posix.basename(styleAsset.sourceMapUrl))}`),
    );
    await readFile(path.join(root, scriptAsset.sourceMapUrl.slice(1)), "utf8");
    await readFile(path.join(root, styleAsset.sourceMapUrl.slice(1)), "utf8");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
