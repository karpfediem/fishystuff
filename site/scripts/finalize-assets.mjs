#!/usr/bin/env node
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import {
  mkdir,
  readdir,
  readFile,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HASH_LENGTH = 16;
const GENERATED_CSP_ATTRIBUTE = "data-fishystuff-generated-csp";
const DEFAULT_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", ".out");

const SCRIPT_TAG_RE = /<script\b[^>]*>/gi;
const LINK_TAG_RE = /<link\b[^>]*>/gi;
const INLINE_SCRIPT_RE = /<script\b([^>]*)>([\s\S]*?)<\/script>/gi;
const GENERATED_CSP_RE =
  /<meta\s+http-equiv=(["'])Content-Security-Policy\1\s+data-fishystuff-generated-csp\b[^>]*>\s*/gi;

function usage() {
  console.log(`finalize-assets.mjs

Usage:
  node site/scripts/finalize-assets.mjs --root <site-output-dir>

Minifies JS/CSS assets referenced by generated HTML, writes content-hashed
copies plus public source maps, rewrites HTML with SRI, and injects a per-page
CSP meta tag with hashes for inline scripts.`);
}

function parseArgs(argv) {
  let rootDir = DEFAULT_ROOT;
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") {
      usage();
      process.exit(0);
    }
    if (arg === "--root" || arg === "--out") {
      rootDir = path.resolve(argv[index + 1] || "");
      index += 1;
      continue;
    }
    if (arg.startsWith("--root=")) {
      rootDir = path.resolve(arg.slice("--root=".length));
      continue;
    }
    if (arg.startsWith("--out=")) {
      rootDir = path.resolve(arg.slice("--out=".length));
      continue;
    }
    throw new Error(`Unknown argument: ${arg}`);
  }
  return { rootDir };
}

async function walkFiles(rootDir) {
  const files = [];
  async function walk(currentDir) {
    const entries = await readdir(currentDir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(currentDir, entry.name);
      if (entry.isDirectory()) {
        if (entry.name === ".asset-finalize-tmp") {
          continue;
        }
        await walk(fullPath);
      } else if (entry.isFile()) {
        files.push(fullPath);
      }
    }
  }
  await walk(rootDir);
  return files;
}

function htmlEscapeAttribute(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll('"', "&quot;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function attrValuePattern(name) {
  return new RegExp(`\\s${name}\\s*=\\s*("([^"]*)"|'([^']*)'|([^\\s>]+))`, "i");
}

export function getAttribute(tag, name) {
  const match = tag.match(attrValuePattern(name));
  if (!match) {
    return null;
  }
  return match[2] ?? match[3] ?? match[4] ?? "";
}

function removeAttribute(tag, name) {
  return tag.replace(attrValuePattern(name), "");
}

function setAttribute(tag, name, value) {
  const escapedValue = htmlEscapeAttribute(value);
  const pattern = attrValuePattern(name);
  if (pattern.test(tag)) {
    return tag.replace(pattern, ` ${name}="${escapedValue}"`);
  }
  return tag.replace(/\s*\/?>$/, (suffix) => ` ${name}="${escapedValue}"${suffix}`);
}

function hasStylesheetRel(tag) {
  const rel = getAttribute(tag, "rel");
  return rel?.split(/\s+/).some((part) => part.toLowerCase() === "stylesheet") ?? false;
}

function scriptIsModule(tag) {
  return getAttribute(tag, "type")?.toLowerCase() === "module";
}

function cleanAssetUrl(rawUrl) {
  if (!rawUrl || rawUrl.startsWith("//")) {
    return null;
  }
  try {
    const url = new URL(rawUrl, "https://site.invalid");
    if (url.origin !== "https://site.invalid") {
      return null;
    }
    return url.pathname;
  } catch {
    return null;
  }
}

function isBuildableAssetUrl(pathname, extension) {
  if (!pathname?.startsWith("/")) {
    return false;
  }
  if (pathname === "/runtime-config.js") {
    return false;
  }
  if (pathname.includes("..")) {
    return false;
  }
  return pathname.endsWith(extension);
}

function assetUrlToRelativePath(assetUrl) {
  return decodeURIComponent(assetUrl.slice(1));
}

function toAssetUrl(relativePath) {
  return `/${relativePath.split(path.sep).join("/")}`;
}

export function hashedAssetPath(relativePath, hash) {
  const parsed = path.posix.parse(relativePath);
  return path.posix.join(parsed.dir, `${parsed.name}.${hash}${parsed.ext}`);
}

function contentHash(buffer) {
  return createHash("sha256").update(buffer).digest("hex").slice(0, HASH_LENGTH);
}

function sriHash(buffer) {
  return `sha384-${createHash("sha384").update(buffer).digest("base64")}`;
}

function cspHash(text) {
  return `sha256-${createHash("sha256").update(text).digest("base64")}`;
}

async function assertFileExists(filePath, label) {
  const fileStat = await stat(filePath).catch(() => null);
  if (!fileStat?.isFile()) {
    throw new Error(`${label} does not exist: ${filePath}`);
  }
}

async function assertDirectoryExists(dirPath, label) {
  const dirStat = await stat(dirPath).catch(() => null);
  if (!dirStat?.isDirectory()) {
    throw new Error(`${label} does not exist: ${dirPath}`);
  }
}

async function runCommand(command, args, options = {}) {
  const child = spawn(command, args, {
    cwd: options.cwd,
    env: process.env,
    stdio: ["ignore", "pipe", "pipe"],
  });
  const stdout = [];
  const stderr = [];
  child.stdout.on("data", (chunk) => stdout.push(chunk));
  child.stderr.on("data", (chunk) => stderr.push(chunk));
  const code = await new Promise((resolve, reject) => {
    child.on("error", reject);
    child.on("close", resolve);
  });
  if (code !== 0) {
    const output = Buffer.concat([...stdout, ...stderr]).toString("utf8").trim();
    throw new Error(`${command} failed with exit code ${code}\n${output}`);
  }
}

function esbuildArgs(relativePath, outRelativePath, { bundle, sourcemap }) {
  const args = [
    relativePath,
    "--platform=browser",
    "--target=es2020",
    "--minify",
    "--legal-comments=none",
    `--outfile=${outRelativePath}`,
  ];
  if (sourcemap) {
    args.splice(args.length - 1, 0, `--sourcemap=${sourcemap}`);
  }
  if (bundle) {
    args.splice(1, 0, "--bundle", "--format=esm");
  }
  return args;
}

function linkedJsSourceMap(buffer, sourceMapRelativePath) {
  const mapBasename = path.posix.basename(sourceMapRelativePath);
  const body = buffer.toString("utf8").replace(/\n?\/\/# sourceMappingURL=.*$/s, "");
  return `${body.endsWith("\n") ? body : `${body}\n`}//# sourceMappingURL=${mapBasename}\n`;
}

function linkedCssSourceMap(buffer, sourceMapRelativePath) {
  const mapBasename = path.posix.basename(sourceMapRelativePath);
  const body = buffer.toString("utf8").replace(/\n?\/\*# sourceMappingURL=.*?\*\/\s*$/s, "");
  return `${body.endsWith("\n") ? body : `${body}\n`}/*# sourceMappingURL=${mapBasename} */\n`;
}

async function buildJsAsset(rootDir, _tempDir, relativePath, { bundle }) {
  const tempRelativePath = path.posix.join(
    ".asset-finalize-tmp",
    "nomap",
    relativePath,
  );
  await mkdir(path.dirname(path.join(rootDir, tempRelativePath)), { recursive: true });
  await runCommand("esbuild", esbuildArgs(relativePath, tempRelativePath, {
    bundle,
    sourcemap: false,
  }), { cwd: rootDir });
  const minifiedBuffer = await readFile(path.join(rootDir, tempRelativePath));
  const tempMapRelativePath = path.posix.join(
    ".asset-finalize-tmp",
    "map",
    relativePath,
  );
  await mkdir(path.dirname(path.join(rootDir, tempMapRelativePath)), { recursive: true });
  await runCommand("esbuild", esbuildArgs(relativePath, tempMapRelativePath, {
    bundle,
    sourcemap: "external",
  }), { cwd: rootDir });
  const sourceMapBuffer = await readFile(path.join(rootDir, `${tempMapRelativePath}.map`));
  const sourceMapHash = contentHash(sourceMapBuffer);
  const sourceMapRelativePath = `${hashedAssetPath(relativePath, sourceMapHash)}.map`;
  await mkdir(path.dirname(path.join(rootDir, sourceMapRelativePath)), { recursive: true });
  await writeFile(path.join(rootDir, sourceMapRelativePath), sourceMapBuffer);

  const finalContent = linkedJsSourceMap(minifiedBuffer, sourceMapRelativePath);
  const finalBuffer = Buffer.from(finalContent, "utf8");
  const hash = contentHash(finalBuffer);
  const hashedRelativePath = hashedAssetPath(relativePath, hash);
  await mkdir(path.dirname(path.join(rootDir, hashedRelativePath)), { recursive: true });
  await writeFile(path.join(rootDir, hashedRelativePath), finalBuffer);
  await assertFileExists(path.join(rootDir, sourceMapRelativePath), "JS source map");
  return {
    kind: "script",
    original: toAssetUrl(relativePath),
    url: toAssetUrl(hashedRelativePath),
    sourceMapUrl: toAssetUrl(sourceMapRelativePath),
    integrity: sriHash(finalBuffer),
    bundled: bundle,
  };
}

async function buildCssAsset(rootDir, _tempDir, relativePath) {
  const tempRelativePath = path.posix.join(
    ".asset-finalize-tmp",
    "nomap",
    relativePath,
  );
  await mkdir(path.dirname(path.join(rootDir, tempRelativePath)), { recursive: true });
  await runCommand("lightningcss", [
    relativePath,
    "--minify",
    "--output-file",
    tempRelativePath,
  ], { cwd: rootDir });
  const minifiedBuffer = await readFile(path.join(rootDir, tempRelativePath));
  const tempMapRelativePath = path.posix.join(
    ".asset-finalize-tmp",
    "map",
    relativePath,
  );
  await mkdir(path.dirname(path.join(rootDir, tempMapRelativePath)), { recursive: true });
  await runCommand("lightningcss", [
    relativePath,
    "--minify",
    "--sourcemap",
    "--output-file",
    tempMapRelativePath,
  ], { cwd: rootDir });
  const sourceMapBuffer = await readFile(path.join(rootDir, `${tempMapRelativePath}.map`));
  const sourceMapHash = contentHash(sourceMapBuffer);
  const sourceMapRelativePath = `${hashedAssetPath(relativePath, sourceMapHash)}.map`;
  await mkdir(path.dirname(path.join(rootDir, sourceMapRelativePath)), { recursive: true });
  await writeFile(path.join(rootDir, sourceMapRelativePath), sourceMapBuffer);

  const cssWithLinkedMap = linkedCssSourceMap(minifiedBuffer, sourceMapRelativePath);
  const finalBuffer = Buffer.from(cssWithLinkedMap, "utf8");
  const hash = contentHash(finalBuffer);
  const hashedRelativePath = hashedAssetPath(relativePath, hash);
  const finalPath = path.join(rootDir, hashedRelativePath);
  await mkdir(path.dirname(finalPath), { recursive: true });
  await writeFile(finalPath, finalBuffer);
  await assertFileExists(path.join(rootDir, sourceMapRelativePath), "CSS source map");
  return {
    kind: "stylesheet",
    original: toAssetUrl(relativePath),
    url: toAssetUrl(hashedRelativePath),
    sourceMapUrl: toAssetUrl(sourceMapRelativePath),
    integrity: sriHash(finalBuffer),
  };
}

export function discoverAssetReferences(html) {
  const references = new Map();
  for (const match of html.matchAll(SCRIPT_TAG_RE)) {
    const tag = match[0];
    const pathname = cleanAssetUrl(getAttribute(tag, "src"));
    if (!isBuildableAssetUrl(pathname, ".js")) {
      continue;
    }
    const existing = references.get(pathname);
    references.set(pathname, {
      kind: "script",
      module: scriptIsModule(tag) || existing?.module === true,
    });
  }
  for (const match of html.matchAll(LINK_TAG_RE)) {
    const tag = match[0];
    if (!hasStylesheetRel(tag)) {
      continue;
    }
    const pathname = cleanAssetUrl(getAttribute(tag, "href"));
    if (!isBuildableAssetUrl(pathname, ".css")) {
      continue;
    }
    references.set(pathname, { kind: "stylesheet" });
  }
  return references;
}

function rewriteExternalScriptTags(html, assetMap) {
  return html.replace(SCRIPT_TAG_RE, (tag) => {
    const pathname = cleanAssetUrl(getAttribute(tag, "src"));
    const asset = pathname ? assetMap.get(pathname) : null;
    if (!asset || asset.kind !== "script") {
      return tag;
    }
    let next = setAttribute(tag, "src", asset.url);
    next = removeAttribute(next, "integrity");
    next = setAttribute(next, "integrity", asset.integrity);
    return next;
  });
}

function rewriteStylesheetTags(html, assetMap) {
  return html.replace(LINK_TAG_RE, (tag) => {
    if (!hasStylesheetRel(tag)) {
      return tag;
    }
    const pathname = cleanAssetUrl(getAttribute(tag, "href"));
    const asset = pathname ? assetMap.get(pathname) : null;
    if (!asset || asset.kind !== "stylesheet") {
      return tag;
    }
    let next = setAttribute(tag, "href", asset.url);
    next = removeAttribute(next, "integrity");
    next = setAttribute(next, "integrity", asset.integrity);
    return next;
  });
}

function inlineScriptHashes(html) {
  const hashes = [];
  for (const match of html.matchAll(INLINE_SCRIPT_RE)) {
    const attrs = match[1] || "";
    if (/\ssrc\s*=/i.test(attrs)) {
      continue;
    }
    const content = match[2] || "";
    if (content.trim().length === 0) {
      continue;
    }
    hashes.push(cspHash(content));
  }
  return [...new Set(hashes)].sort();
}

export function buildCspPolicy({ scriptHashes = [] } = {}) {
  const scriptSources = [
    "'self'",
    "'unsafe-eval'",
    "'wasm-unsafe-eval'",
    "https://cdn.fishystuff.fish",
    "https://cdn.beta.fishystuff.fish",
    "http://127.0.0.1:*",
    "http://localhost:*",
    ...scriptHashes.map((hash) => `'${hash}'`),
  ];
  const directives = [
    ["default-src", "'self'"],
    ["base-uri", "'self'"],
    ["object-src", "'none'"],
    ["script-src", ...scriptSources],
    ["script-src-attr", "'none'"],
    ["style-src", "'self'", "'unsafe-inline'"],
    ["img-src", "'self'", "data:", "https:", "http:"],
    ["font-src", "'self'"],
    [
      "connect-src",
      "'self'",
      "https://api.fishystuff.fish",
      "https://api.beta.fishystuff.fish",
      "https://cdn.fishystuff.fish",
      "https://cdn.beta.fishystuff.fish",
      "https://telemetry.fishystuff.fish",
      "https://telemetry.beta.fishystuff.fish",
      "http://127.0.0.1:*",
      "http://localhost:*",
      "http://telemetry.localhost:*",
    ],
    ["frame-src", "https://www.youtube.com", "https://www.youtube-nocookie.com"],
    ["worker-src", "'self'", "blob:"],
    ["manifest-src", "'self'"],
    ["form-action", "'self'"],
  ];
  return directives.map((parts) => parts.join(" ")).join("; ");
}

function injectCspMeta(html, policy) {
  const meta = `<meta http-equiv="Content-Security-Policy" ${GENERATED_CSP_ATTRIBUTE} content="${htmlEscapeAttribute(policy)}">\n`;
  const withoutGeneratedCsp = html.replace(GENERATED_CSP_RE, "");
  if (/<head\b[^>]*>/i.test(withoutGeneratedCsp)) {
    return withoutGeneratedCsp.replace(/<head\b[^>]*>/i, (tag) => `${tag}\n  ${meta}`);
  }
  return withoutGeneratedCsp;
}

export function rewriteHtml(html, assetMap) {
  const withAssets = rewriteStylesheetTags(rewriteExternalScriptTags(html, assetMap), assetMap);
  const policy = buildCspPolicy({ scriptHashes: inlineScriptHashes(withAssets) });
  return injectCspMeta(withAssets, policy);
}

async function readHtmlFiles(rootDir) {
  const files = await walkFiles(rootDir);
  return files
    .filter((filePath) => filePath.endsWith(".html"))
    .sort((a, b) => a.localeCompare(b));
}

async function buildAssets(rootDir, references) {
  const tempDir = path.join(rootDir, ".asset-finalize-tmp");
  await rm(tempDir, { recursive: true, force: true });
  await mkdir(tempDir, { recursive: true });
  try {
    const assetMap = new Map();
    for (const [assetUrl, reference] of [...references.entries()].sort()) {
      const relativePath = assetUrlToRelativePath(assetUrl);
      await assertFileExists(path.join(rootDir, relativePath), "Referenced asset");
      const asset = reference.kind === "script"
        ? await buildJsAsset(rootDir, tempDir, relativePath, {
            bundle: reference.module === true,
          })
        : await buildCssAsset(rootDir, tempDir, relativePath);
      assetMap.set(assetUrl, asset);
    }
    return assetMap;
  } finally {
    await rm(tempDir, { recursive: true, force: true });
  }
}

async function writeManifest(rootDir, assetMap) {
  const assets = {};
  for (const [original, asset] of [...assetMap.entries()].sort()) {
    assets[original] = asset;
  }
  const manifest = {
    version: 1,
    hashAlgorithm: "sha256",
    integrityAlgorithm: "sha384",
    assets,
  };
  await writeFile(
    path.join(rootDir, "asset-manifest.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
    "utf8",
  );
}

export async function finalizeAssets({ rootDir = DEFAULT_ROOT } = {}) {
  rootDir = path.resolve(rootDir);
  await assertDirectoryExists(rootDir, "Site output directory");
  const htmlFiles = await readHtmlFiles(rootDir);
  const htmlByFile = new Map();
  const references = new Map();
  for (const htmlFile of htmlFiles) {
    const html = await readFile(htmlFile, "utf8");
    htmlByFile.set(htmlFile, html);
    for (const [assetUrl, reference] of discoverAssetReferences(html)) {
      const existing = references.get(assetUrl);
      references.set(assetUrl, {
        ...reference,
        module: existing?.module === true || reference.module === true,
      });
    }
  }
  const assetMap = await buildAssets(rootDir, references);
  for (const [htmlFile, html] of htmlByFile) {
    await writeFile(htmlFile, rewriteHtml(html, assetMap), "utf8");
  }
  await writeManifest(rootDir, assetMap);
  return {
    htmlCount: htmlFiles.length,
    assetCount: assetMap.size,
    manifestPath: path.join(rootDir, "asset-manifest.json"),
  };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  try {
    const result = await finalizeAssets(parseArgs(process.argv.slice(2)));
    console.log(
      `Finalized ${result.assetCount} JS/CSS assets across ${result.htmlCount} HTML files.`,
    );
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
