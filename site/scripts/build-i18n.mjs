import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const siteDir = path.resolve(path.dirname(scriptPath), "..");

export const LANGUAGE_CONFIG = Object.freeze({
  defaultContentLang: "en-US",
  defaultLocale: "en-US",
  defaultApiLang: "en",
  contentLanguages: Object.freeze([
    Object.freeze({ code: "en-US", pathPrefix: "/" }),
    Object.freeze({ code: "de-DE", pathPrefix: "/de-DE/" }),
  ]),
  localeLanguages: Object.freeze(["en-US", "de-DE", "ko-KR"]),
  apiLanguages: Object.freeze(["en", "ko"]),
});

function listFiles(rootDir) {
  if (!fs.existsSync(rootDir)) {
    return [];
  }
  const pending = [rootDir];
  const files = [];
  while (pending.length) {
    const current = pending.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        pending.push(fullPath);
        continue;
      }
      files.push(fullPath);
    }
  }
  return files.sort();
}

export function parseFluentMessages(source) {
  const messages = {};
  let currentKey = "";
  for (const rawLine of String(source || "").split(/\r?\n/)) {
    const line = rawLine.replace(/\r$/, "");
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) {
      continue;
    }
    if (/^\s/.test(line) && currentKey) {
      messages[currentKey] = messages[currentKey]
        ? `${messages[currentKey]}\n${trimmed}`
        : trimmed;
      continue;
    }
    const separator = line.indexOf("=");
    if (separator <= 0) {
      throw new Error(`Unsupported Fluent entry: ${line}`);
    }
    const key = line.slice(0, separator).trim();
    const value = line.slice(separator + 1).replace(/^\s*/, "");
    if (!key || key.startsWith("-") || key.startsWith(".")) {
      throw new Error(`Unsupported Fluent key: ${line}`);
    }
    messages[key] = value;
    currentKey = key;
  }
  return messages;
}

export function loadLocaleCatalogs(rootDir = fluentDir) {
  const locales = {};
  for (const localeEntry of fs.readdirSync(rootDir, { withFileTypes: true })) {
    if (!localeEntry.isDirectory()) {
      continue;
    }
    const locale = localeEntry.name;
    const localeDir = path.join(rootDir, locale);
    const catalog = {};
    for (const filePath of listFiles(localeDir)) {
      if (!filePath.endsWith(".ftl")) {
        continue;
      }
      Object.assign(catalog, parseFluentMessages(fs.readFileSync(filePath, "utf8")));
    }
    locales[locale] = catalog;
  }
  return locales;
}

export function resolveLocaleCatalogs(catalogs, defaultLocale = LANGUAGE_CONFIG.defaultLocale) {
  const defaultCatalog = catalogs[defaultLocale];
  if (!defaultCatalog) {
    throw new Error(`Missing default locale catalog: ${defaultLocale}`);
  }
  const resolved = {};
  for (const [locale, catalog] of Object.entries(catalogs)) {
    resolved[locale] = {
      ...defaultCatalog,
      ...catalog,
    };
  }
  return resolved;
}

function normalizeRouteKey(relativePath) {
  const normalized = relativePath.replace(/\\/g, "/");
  if (normalized === "index.smd") {
    return "/";
  }
  if (normalized.endsWith("/index.smd")) {
    return `/${normalized.slice(0, -"index.smd".length)}`;
  }
  return `/${normalized.slice(0, -".smd".length)}/`;
}

function joinPath(prefix, routeKey) {
  const normalizedPrefix = String(prefix || "/").replace(/\/+$/, "");
  const normalizedRoute = routeKey === "/" ? "/" : routeKey.replace(/^\/+/, "/");
  if (!normalizedPrefix || normalizedPrefix === "/") {
    return normalizedRoute;
  }
  return `${normalizedPrefix}${normalizedRoute}`.replace(/\/{2,}/g, "/");
}

export function buildPageManifest(config = LANGUAGE_CONFIG, rootDir = siteDir) {
  const manifest = {};
  for (const contentLanguage of config.contentLanguages) {
    const contentDir = path.join(rootDir, "content", contentLanguage.code);
    for (const filePath of listFiles(contentDir)) {
      if (!filePath.endsWith(".smd")) {
        continue;
      }
      const relativePath = path.relative(contentDir, filePath);
      const routeKey = normalizeRouteKey(relativePath);
      if (!manifest[routeKey]) {
        manifest[routeKey] = {};
      }
      manifest[routeKey][contentLanguage.code] = joinPath(contentLanguage.pathPrefix, routeKey);
    }
  }
  return manifest;
}

function ensureDirectory(targetPath) {
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
}

function writeJson(targetPath, value) {
  ensureDirectory(targetPath);
  fs.writeFileSync(targetPath, `${JSON.stringify(value, null, 2)}\n`);
}

function writeGeneratedScript(targetPath, payload) {
  ensureDirectory(targetPath);
  fs.writeFileSync(
    targetPath,
    [
      "(function () {",
      `  window.__fishystuffGeneratedI18n = Object.freeze(${JSON.stringify(payload, null, 2)});`,
      "})();",
      "",
    ].join("\n"),
  );
}

export function buildI18nArtifacts({
  config = LANGUAGE_CONFIG,
  rootDir = siteDir,
} = {}) {
  const catalogs = resolveLocaleCatalogs(
    loadLocaleCatalogs(path.join(rootDir, "i18n", "fluent")),
    config.defaultLocale,
  );
  const pageManifest = buildPageManifest(config, rootDir);
  for (const locale of Object.keys(catalogs)) {
    writeJson(path.join(rootDir, "i18n", `${locale}.ziggy`), catalogs[locale]);
  }
  writeGeneratedScript(path.join(rootDir, "assets", "js", "generated", "site-i18n.js"), {
    config,
    catalogs,
    pageManifest,
  });
  return { catalogs, pageManifest };
}

if (process.argv[1] === scriptPath) {
  buildI18nArtifacts();
}
