import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";

import { resolveBrandAssets } from "./brand-assets.mjs";
import { loadLocaleCatalogs, resolveLocaleCatalogs } from "./build-i18n.mjs";
import { LANGUAGE_CONFIG } from "./language-config.mjs";

const scriptPath = fileURLToPath(import.meta.url);
const siteDir = path.resolve(path.dirname(scriptPath), "..");

const EMBED_WIDTH = 900;
const EMBED_HEIGHT = 300;
const BRAND_TEXT = "FishyStuff";
const DEFAULT_TAGLINE = "Everything you need to get fishing";

const assetsDir = path.join(siteDir, "assets");
const fontsCssPath = path.join(assetsDir, "css", "fonts.css");
const siteCssPath = path.join(assetsDir, "css", "site.css");
const templatePath = path.join(siteDir, "scripts", "embed-image-template.html");

function runChromium(args, label) {
  const result = spawnSync("chromium", args, {
    cwd: siteDir,
    stdio: "inherit",
  });
  if (result.error?.code === "ENOENT") {
    throw new Error("Chromium was not found on PATH; install chromium before running site embed builds.");
  }
  if (result.status !== 0) {
    throw new Error(`chromium failed while ${label}`);
  }
}

function ensureInputs(logoPath) {
  for (const inputPath of [fontsCssPath, siteCssPath, logoPath, templatePath]) {
    if (!fs.existsSync(inputPath)) {
      throw new Error(`Missing embed build input: ${path.relative(siteDir, inputPath)}`);
    }
  }
}

function loadCatalogs(rootDir = siteDir) {
  return resolveLocaleCatalogs(
    loadLocaleCatalogs(path.join(rootDir, "i18n", "fluent")),
    LANGUAGE_CONFIG.defaultLocale,
  );
}

function normalizeTagline(value) {
  const normalized = String(value ?? "").trim();
  return normalized || DEFAULT_TAGLINE;
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function resolveTitleFontSize(tagline) {
  if (tagline.length > 34) {
    return "1.9rem";
  }
  if (tagline.length > 26) {
    return "2.05rem";
  }
  return "2.2rem";
}

function resolveOutputPaths(locale) {
  if (locale === LANGUAGE_CONFIG.defaultContentLang) {
    return [path.join(assetsDir, "img", "embed.png")];
  }
  return [
    path.join(assetsDir, "img", `embed.${locale}.png`),
    path.join(assetsDir, locale, "embed.png"),
  ];
}

function cleanupObsoleteFiles() {
  fs.rmSync(path.join(assetsDir, "embed.png"), { force: true });
  for (const entry of fs.readdirSync(path.join(assetsDir, "img"), { withFileTypes: true })) {
    if (!entry.isFile()) {
      continue;
    }
    if (/^embed(?:\.[A-Za-z-]+)?_\(Original\)\.png$/.test(entry.name)) {
      fs.rmSync(path.join(assetsDir, "img", entry.name), { force: true });
    }
  }
}

function buildHtmlDocument({ locale, tagline, logoPath }) {
  const templateSource = fs.readFileSync(templatePath, "utf8");
  const fontsCssHref = pathToFileURL(fontsCssPath).href;
  const siteCssHref = pathToFileURL(siteCssPath).href;
  const logoHref = pathToFileURL(logoPath).href;
  const homeHref = locale === LANGUAGE_CONFIG.defaultContentLang ? "/" : `/${locale}/`;
  return templateSource
    .replace("__LOCALE__", escapeHtml(locale))
    .replace("__FONTS_CSS_HREF__", fontsCssHref)
    .replace("__SITE_CSS_HREF__", siteCssHref)
    .replace("__HOME_HREF__", homeHref)
    .replace("__LOGO_HREF__", logoHref)
    .replace("__TAGLINE__", escapeHtml(tagline))
    .replace("__TITLE_FONT_SIZE__", resolveTitleFontSize(tagline))
    .replace("FishyStuff", BRAND_TEXT);
}

function renderLocaleEmbed({ locale, tagline, outputPaths, logoPath }) {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), `fishystuff-embed-${locale}-`));
  try {
    const htmlPath = path.join(tempDir, "embed.html");
    const screenshotPath = path.join(tempDir, "embed.png");
    fs.writeFileSync(htmlPath, buildHtmlDocument({ locale, tagline, logoPath }), "utf8");
    runChromium([
      "--headless=new",
      "--disable-gpu",
      "--hide-scrollbars",
      "--force-device-scale-factor=1",
      "--run-all-compositor-stages-before-draw",
      "--virtual-time-budget=2000",
      `--window-size=${EMBED_WIDTH},${EMBED_HEIGHT}`,
      `--screenshot=${screenshotPath}`,
      pathToFileURL(htmlPath).href,
    ], `rendering ${locale} embed`);
    for (const outputPath of outputPaths) {
      fs.mkdirSync(path.dirname(outputPath), { recursive: true });
      fs.copyFileSync(screenshotPath, outputPath);
    }
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
}

export function buildEmbedImages({
  rootDir = siteDir,
  config = LANGUAGE_CONFIG,
  env = process.env,
} = {}) {
  const brandAssets = resolveBrandAssets(env);
  ensureInputs(brandAssets.embedLogoPath);
  cleanupObsoleteFiles();
  const catalogs = loadCatalogs(rootDir);
  for (const language of config.contentLanguages) {
    renderLocaleEmbed({
      locale: language.code,
      tagline: normalizeTagline(catalogs[language.code]?.["frontpage.hero.title"]),
      outputPaths: resolveOutputPaths(language.code),
      logoPath: brandAssets.embedLogoPath,
    });
  }
}

const isMainModule = process.argv[1] && path.resolve(process.argv[1]) === scriptPath;

if (isMainModule) {
  try {
    buildEmbedImages();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
