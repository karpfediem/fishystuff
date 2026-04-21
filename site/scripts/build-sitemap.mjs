import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { LANGUAGE_CONFIG } from "./language-config.mjs";
import { buildShellPageEntries, buildShellPagePathSet } from "./shell-pages.mjs";

const scriptPath = fileURLToPath(import.meta.url);
const siteDir = path.resolve(path.dirname(scriptPath), "..");

function parseArgs(argv) {
  const args = {
    out: "",
    rootDir: siteDir,
    hostUrl: "",
    zineConfigPath: "",
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--out" && index + 1 < argv.length) {
      args.out = argv[++index];
      continue;
    }
    if (arg === "--root-dir" && index + 1 < argv.length) {
      args.rootDir = argv[++index];
      continue;
    }
    if (arg === "--host-url" && index + 1 < argv.length) {
      args.hostUrl = argv[++index];
      continue;
    }
    if (arg === "--zine-config" && index + 1 < argv.length) {
      args.zineConfigPath = argv[++index];
      continue;
    }
    throw new Error(`unknown arg: ${arg}`);
  }

  if (!args.out) {
    throw new Error("missing required --out");
  }

  return args;
}

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

function parseFrontmatter(source) {
  const match = String(source ?? "").match(/^---\s*\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/);
  return match ? match[1] : "";
}

function parseTranslationKey(frontmatter) {
  const match = String(frontmatter ?? "").match(/^\s*\.translation_key\s*=\s*"([^"]+)"\s*,?\s*$/m);
  return match ? match[1].trim() : "";
}

function parseBooleanField(frontmatter, fieldName) {
  return new RegExp(`\\.${fieldName}\\s*=\\s*true\\b`).test(String(frontmatter ?? ""));
}

function parseDateField(frontmatter, fieldName) {
  const match = String(frontmatter ?? "").match(new RegExp(`\\.${fieldName}\\s*=\\s*@date\\("([^"]+)"\\)`));
  return match ? match[1] : "";
}

export function extractDateStamp(value) {
  const match = String(value ?? "").match(/(\d{4}-\d{2}-\d{2})/);
  return match ? match[1] : "";
}

function escapeXml(value) {
  return String(value ?? "")
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function ensureTrailingSlash(value) {
  return String(value ?? "").endsWith("/") ? String(value ?? "") : `${String(value ?? "")}/`;
}

function buildAbsoluteUrl(hostUrl, pagePath) {
  return new URL(pagePath, ensureTrailingSlash(hostUrl)).toString();
}

export function readHostUrlFromZineConfig(configPath) {
  const source = fs.readFileSync(configPath, "utf8");
  const match = source.match(/\.host_url\s*=\s*"([^"]+)"/);
  if (!match?.[1]) {
    throw new Error(`Unable to find .host_url in ${configPath}`);
  }
  return match[1];
}

function addGroupEntry(groups, entry) {
  if (entry.draft || entry.noindex) {
    return;
  }
  let group = groups.get(entry.groupKey);
  if (!group) {
    group = [];
    groups.set(entry.groupKey, group);
  }
  group.push(entry);
}

function sortAlternates(left, right) {
  const localeCompare = left.locale.localeCompare(right.locale);
  if (localeCompare !== 0) {
    return localeCompare;
  }
  return left.path.localeCompare(right.path);
}

function collectTrackedContentEntries(config, rootDir) {
  const shellPathsByLocale = buildShellPagePathSet({ config, rootDir });
  const entries = [];

  for (const contentLanguage of config.contentLanguages) {
    const locale = contentLanguage.code;
    const contentDir = path.join(rootDir, "content", locale);
    for (const filePath of listFiles(contentDir)) {
      if (!filePath.endsWith(".smd")) {
        continue;
      }
      const relativePath = path.relative(contentDir, filePath).replace(/\\/g, "/");
      if (shellPathsByLocale.get(locale)?.has(relativePath)) {
        continue;
      }
      const source = fs.readFileSync(filePath, "utf8");
      const frontmatter = parseFrontmatter(source);
      const translationKey = parseTranslationKey(frontmatter);
      entries.push({
        groupKey: translationKey
          ? `translation:${translationKey}`
          : `path:${relativePath}`,
        locale,
        path: joinPath(contentLanguage.pathPrefix, normalizeRouteKey(relativePath)),
        lastmod: extractDateStamp(parseDateField(frontmatter, "updated") || parseDateField(frontmatter, "date")),
        draft: parseBooleanField(frontmatter, "draft"),
        noindex: parseBooleanField(frontmatter, "noindex"),
      });
    }
  }

  return entries;
}

function collectShellEntries(config, rootDir) {
  const prefixByLocale = new Map(
    config.contentLanguages.map((language) => [language.code, language.pathPrefix]),
  );

  return buildShellPageEntries({ config, rootDir }).map((entry) => ({
    groupKey: entry.translationKey
      ? `translation:${entry.translationKey}`
      : `path:${entry.relativePath}`,
    locale: entry.locale,
    path: joinPath(prefixByLocale.get(entry.locale), entry.routeKey),
    lastmod: extractDateStamp(entry.updated || entry.date),
    draft: Boolean(entry.draft),
    noindex: false,
  }));
}

export function buildSitemapRecords({
  config = LANGUAGE_CONFIG,
  rootDir = siteDir,
  hostUrl,
} = {}) {
  const resolvedHostUrl = hostUrl || readHostUrlFromZineConfig(path.join(rootDir, "zine.ziggy"));
  const groups = new Map();

  for (const entry of [
    ...collectTrackedContentEntries(config, rootDir),
    ...collectShellEntries(config, rootDir),
  ]) {
    addGroupEntry(groups, entry);
  }

  const records = [];
  for (const groupEntries of groups.values()) {
    const sortedAlternates = [...groupEntries].sort(sortAlternates);
    const xDefault = sortedAlternates.find((entry) => entry.locale === config.defaultContentLang)?.path ?? "";
    for (const entry of sortedAlternates) {
      records.push({
        locale: entry.locale,
        path: entry.path,
        loc: buildAbsoluteUrl(resolvedHostUrl, entry.path),
        lastmod: entry.lastmod,
        alternates: sortedAlternates.map((alternate) => ({
          hreflang: alternate.locale,
          href: buildAbsoluteUrl(resolvedHostUrl, alternate.path),
        })),
        xDefaultHref: xDefault ? buildAbsoluteUrl(resolvedHostUrl, xDefault) : "",
      });
    }
  }

  return records.sort((left, right) => left.path.localeCompare(right.path));
}

export function renderSitemapXml(records) {
  const lines = [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">',
  ];

  for (const record of records) {
    lines.push("  <url>");
    lines.push(`    <loc>${escapeXml(record.loc)}</loc>`);
    if (record.lastmod) {
      lines.push(`    <lastmod>${escapeXml(record.lastmod)}</lastmod>`);
    }
    for (const alternate of record.alternates) {
      lines.push(
        `    <xhtml:link rel="alternate" hreflang="${escapeXml(alternate.hreflang)}" href="${escapeXml(alternate.href)}" />`,
      );
    }
    if (record.xDefaultHref) {
      lines.push(
        `    <xhtml:link rel="alternate" hreflang="x-default" href="${escapeXml(record.xDefaultHref)}" />`,
      );
    }
    lines.push("  </url>");
  }

  lines.push("</urlset>", "");
  return lines.join("\n");
}

export function buildSitemap({
  config = LANGUAGE_CONFIG,
  rootDir = siteDir,
  hostUrl = "",
  outPath,
} = {}) {
  const records = buildSitemapRecords({
    config,
    rootDir,
    hostUrl,
  });
  const xml = renderSitemapXml(records);
  if (outPath) {
    fs.mkdirSync(path.dirname(outPath), { recursive: true });
    fs.writeFileSync(outPath, xml, "utf8");
  }
  return { records, xml };
}

const isMainModule = process.argv[1] && path.resolve(process.argv[1]) === scriptPath;

if (isMainModule) {
  try {
    const args = parseArgs(process.argv.slice(2));
    const rootDir = path.resolve(args.rootDir);
    const resolvedHostUrl = args.hostUrl
      || readHostUrlFromZineConfig(
        path.resolve(args.zineConfigPath || path.join(rootDir, "zine.ziggy")),
      );
    buildSitemap({
      rootDir,
      hostUrl: resolvedHostUrl,
      outPath: path.resolve(args.out),
    });
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
