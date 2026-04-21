import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { buildPageManifest } from "./build-i18n.mjs";
import { LANGUAGE_CONFIG } from "./language-config.mjs";
import { buildShellPageEntries, buildShellPagePathSet, renderShellPageSource } from "./shell-pages.mjs";

const scriptPath = fileURLToPath(import.meta.url);
const siteDir = path.resolve(path.dirname(scriptPath), "..");
const TRANSLATION_HELP_ROUTE_KEY = "/community/";
const GITHUB_REPOSITORY_URL = "https://github.com/karpfediem/fishystuff";
const GITHUB_DEFAULT_BRANCH = "main";

function parseArgs(argv) {
  const args = {
    outRoot: path.join(".generated", "content"),
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--out-root" && index + 1 < argv.length) {
      args.outRoot = argv[++index];
      continue;
    }
    throw new Error(`unknown arg: ${arg}`);
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

function copyTrackedContentTree(sourceDir, targetDir, excludedPaths) {
  if (!fs.existsSync(sourceDir)) {
    fs.mkdirSync(targetDir, { recursive: true });
    return;
  }
  fs.cpSync(sourceDir, targetDir, {
    recursive: true,
    filter(sourcePath) {
      const relativePath = path.relative(sourceDir, sourcePath).replace(/\\/g, "/");
      if (!relativePath) {
        return true;
      }
      return !excludedPaths.has(relativePath);
    },
  });
}

function copyNonSmdTree(sourceDir, targetDir, excludedPaths) {
  if (!fs.existsSync(sourceDir)) {
    fs.mkdirSync(targetDir, { recursive: true });
    return;
  }
  fs.cpSync(sourceDir, targetDir, {
    recursive: true,
    filter(sourcePath) {
      const relativePath = path.relative(sourceDir, sourcePath).replace(/\\/g, "/");
      if (!relativePath) {
        return true;
      }
      if (excludedPaths.has(relativePath)) {
        return false;
      }
      return fs.statSync(sourcePath).isDirectory() || !relativePath.endsWith(".smd");
    },
  });
}

function trimString(value) {
  return String(value ?? "").trim();
}

function renderQuoted(value) {
  return `"${String(value ?? "").replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
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

function parseFrontmatterParts(source) {
  const match = String(source ?? "").match(/^---\s*\r?\n([\s\S]*?)\r?\n---([ \t]*\r?\n)?([\s\S]*)$/);
  if (!match) {
    throw new Error("expected page source with frontmatter");
  }
  return {
    frontmatter: match[1],
    body: match[3] ?? "",
  };
}

function pageSourceHasFrontmatterField(source, fieldName) {
  const { frontmatter } = parseFrontmatterParts(source);
  return new RegExp(`(^|\\n)\\s*\\.${fieldName}\\s*=`, "m").test(frontmatter);
}

function parseTranslationKey(source) {
  const { frontmatter } = parseFrontmatterParts(source);
  const match = frontmatter.match(/^\s*\.translation_key\s*=\s*"([^"]+)"\s*,?\s*$/m);
  return match ? match[1].trim() : "";
}

function renderCustomField(field) {
  if (typeof field.value === "boolean") {
    return `${field.key} = ${field.value ? "true" : "false"},`;
  }
  return `${field.key} = ${renderQuoted(field.value)},`;
}

function mergeCustomFields(frontmatter, fields) {
  const customBlockPattern = /^(\s*)\.custom\s*=\s*\{([\s\S]*?)^\1\},?\s*$/m;
  const customInlinePattern = /^(\s*)\.custom\s*=\s*\{(.*)\}\s*,?\s*$/m;
  const nextFields = fields.map((field) => `  ${renderCustomField(field)}`);

  if (customBlockPattern.test(frontmatter)) {
    return frontmatter.replace(customBlockPattern, (_match, indent, inner) => {
      const existingLines = String(inner)
        .split(/\r?\n/)
        .filter((line) => line.length > 0);
      const mergedLines = [...existingLines, ...nextFields.map((line) => `${indent}${line}`)];
      return `${indent}.custom = {\n${mergedLines.join("\n")}\n${indent}},`;
    });
  }

  if (customInlinePattern.test(frontmatter)) {
    return frontmatter.replace(customInlinePattern, (_match, indent, inner) => {
      const existing = trimString(inner);
      const existingLines = existing ? [`${indent}  ${existing.endsWith(",") ? existing : `${existing},`}`] : [];
      const mergedLines = [...existingLines, ...nextFields.map((line) => `${indent}${line}`)];
      return `${indent}.custom = {\n${mergedLines.join("\n")}\n${indent}},`;
    });
  }

  const lines = frontmatter.split(/\r?\n/);
  const draftIndex = lines.findIndex((line) => /^\s*\.draft\s*=/.test(line));
  const insertAt = draftIndex >= 0 ? draftIndex : lines.length;
  const customLines = [
    ".custom = {",
    ...nextFields,
    "},",
  ];
  lines.splice(insertAt, 0, ...customLines);
  return lines.join("\n");
}

function applyDefaultOgImage(source, ogImageHref) {
  if (pageSourceHasFrontmatterField(source, "og_image_asset") || pageSourceHasFrontmatterField(source, "og_image")) {
    return source;
  }
  const { frontmatter, body } = parseFrontmatterParts(source);
  return `---\n${mergeCustomFields(frontmatter, [
    { key: ".og_image", value: ogImageHref },
  ])}\n---\n${body}`;
}

function renderFallbackPageSource(source, metadata) {
  const { frontmatter, body } = parseFrontmatterParts(source);
  const nextFrontmatter = mergeCustomFields(frontmatter, [
    { key: ".translation_fallback", value: true },
    { key: ".translation_source_locale", value: metadata.sourceLocale },
    { key: ".translation_target_locale", value: metadata.targetLocale },
    { key: ".translation_help_url", value: metadata.helpUrl },
    { key: ".translation_source_file_url", value: metadata.sourceFileUrl },
    { key: ".translation_create_file_url", value: metadata.createFileUrl },
    { key: ".translation_target_path", value: metadata.targetPath },
    { key: ".translation_source_path", value: metadata.sourcePath },
    { key: ".canonical", value: metadata.sourcePath },
    { key: ".noindex", value: true },
  ]);
  return `---\n${nextFrontmatter}\n---\n${rewriteInternalPageLinks(body, metadata.targetLocale, metadata.targetPathPrefix, metadata.pageManifest)}`;
}

function joinPath(prefix, routeKey) {
  const normalizedPrefix = String(prefix || "/").replace(/\/+$/, "");
  const normalizedRoute = routeKey === "/" ? "/" : routeKey.replace(/^\/+/, "/");
  if (!normalizedPrefix || normalizedPrefix === "/") {
    return normalizedRoute;
  }
  return `${normalizedPrefix}${normalizedRoute}`.replace(/\/{2,}/g, "/");
}

function encodeGitHubPath(filePath) {
  return String(filePath ?? "")
    .split("/")
    .map((segment) => encodeURIComponent(segment))
    .join("/");
}

function buildGitHubBlobUrl(repositoryUrl, branch, filePath) {
  return `${repositoryUrl}/blob/${encodeURIComponent(branch)}/${encodeGitHubPath(filePath)}`;
}

function buildGitHubNewFileUrl(repositoryUrl, branch, filePath) {
  return `${repositoryUrl}/new/${encodeURIComponent(branch)}?filename=${encodeURIComponent(filePath)}`;
}

function splitUrlParts(href) {
  const match = String(href ?? "").match(/^([^?#]*)(\?[^#]*)?(#.*)?$/);
  return {
    pathname: match?.[1] ?? "",
    search: match?.[2] ?? "",
    hash: match?.[3] ?? "",
  };
}

function normalizeManifestRouteKey(pathname) {
  const trimmed = String(pathname ?? "").trim();
  if (!trimmed || trimmed === "/") {
    return "/";
  }
  if (!trimmed.startsWith("/") || trimmed.startsWith("//")) {
    return "";
  }
  const normalized = trimmed.replace(/\/+$/, "");
  if (/\.[a-z0-9]+$/i.test(normalized)) {
    return "";
  }
  return `${normalized}/`;
}

function resolveLocalizedPageHref(href, locale, pageManifest) {
  const { pathname, search, hash } = splitUrlParts(href);
  const routeKey = normalizeManifestRouteKey(pathname);
  if (!routeKey) {
    return href;
  }
  const localizedPath = pageManifest[routeKey]?.[locale];
  if (!localizedPath) {
    return href;
  }
  return `${localizedPath}${search}${hash}`;
}

function rewriteInternalPageLinks(source, locale, pathPrefix, pageManifest) {
  let nextSource = String(source ?? "");
  nextSource = nextSource.replace(/\]\((\/[^)\s]+)\)/g, (_match, href) => {
    return `](${toContentPageHref(resolveLocalizedPageHref(href, locale, pageManifest), pathPrefix)})`;
  });
  nextSource = nextSource.replace(/\bhref=(["'])(\/[^"']+)\1/g, (_match, quote, href) => {
    return `href=${quote}${toContentPageHref(resolveLocalizedPageHref(href, locale, pageManifest), pathPrefix)}${quote}`;
  });
  return nextSource;
}

function toContentPageHref(href, pathPrefix) {
  const { pathname, search, hash } = splitUrlParts(href);
  const normalizedPrefix = String(pathPrefix || "/").replace(/\/+$/, "");
  if (!normalizedPrefix || normalizedPrefix === "/") {
    return `${pathname || "/"}${search}${hash}`;
  }
  const prefixWithSlash = `${normalizedPrefix}/`;
  if (pathname === normalizedPrefix || pathname === prefixWithSlash) {
    return `/${search}${hash}`;
  }
  if (!pathname.startsWith(prefixWithSlash)) {
    return `${pathname}${search}${hash}`;
  }
  const contentPath = pathname.slice(normalizedPrefix.length) || "/";
  return `${contentPath}${search}${hash}`;
}

function buildTrackedContentGroups(config, rootDir, shellPathsByLocale) {
  const groups = new Map();
  for (const contentLanguage of config.contentLanguages) {
    const locale = contentLanguage.code;
    const contentDir = path.join(rootDir, "content", locale);
    if (!fs.existsSync(contentDir)) {
      continue;
    }
    for (const filePath of listFiles(contentDir)) {
      if (!filePath.endsWith(".smd")) {
        continue;
      }
      const relativePath = path.relative(contentDir, filePath).replace(/\\/g, "/");
      if (shellPathsByLocale.get(locale)?.has(relativePath)) {
        continue;
      }
      const source = fs.readFileSync(filePath, "utf8");
      const translationKey = parseTranslationKey(source);
      const groupKey = translationKey
        ? `translation:${translationKey}`
        : `path:${relativePath}`;
      let group = groups.get(groupKey);
      if (!group) {
        group = {
          variants: {},
        };
        groups.set(groupKey, group);
      }
      group.variants[locale] = {
        locale,
        relativePath,
        routeKey: normalizeRouteKey(relativePath),
        source,
      };
    }
  }
  return groups;
}

export function buildShellContentTree({
  config = LANGUAGE_CONFIG,
  rootDir = siteDir,
  outRoot = path.join(rootDir, ".generated", "content"),
} = {}) {
  const shellEntries = buildShellPageEntries({ config, rootDir });
  const shellPathsByLocale = buildShellPagePathSet({ config, rootDir });
  const pageManifest = buildPageManifest(config, rootDir);
  const canonicalLanguage = config.defaultContentLang;
  const canonicalContentDir = path.join(rootDir, "content", canonicalLanguage);
  fs.rmSync(outRoot, { recursive: true, force: true });
  for (const contentLanguage of config.contentLanguages) {
    const locale = contentLanguage.code;
    const targetDir = path.join(outRoot, locale);
    if (locale !== canonicalLanguage) {
      copyNonSmdTree(canonicalContentDir, targetDir, shellPathsByLocale.get(canonicalLanguage) ?? new Set());
    }
    copyTrackedContentTree(path.join(rootDir, "content", locale), targetDir, shellPathsByLocale.get(locale) ?? new Set());
  }
  for (const entry of shellEntries) {
    const targetPath = path.join(outRoot, entry.locale, entry.relativePath);
    fs.mkdirSync(path.dirname(targetPath), { recursive: true });
    fs.writeFileSync(targetPath, renderShellPageSource(entry), "utf8");
  }
  const contentGroups = buildTrackedContentGroups(config, rootDir, shellPathsByLocale);
  for (const group of contentGroups.values()) {
    const sourceEntry = group.variants[canonicalLanguage];
    if (!sourceEntry) {
      continue;
    }
    const sourceLanguage = config.contentLanguages.find((language) => language.code === canonicalLanguage);
    if (!sourceLanguage) {
      continue;
    }
    for (const contentLanguage of config.contentLanguages) {
      const locale = contentLanguage.code;
      if (locale === canonicalLanguage || group.variants[locale]) {
        continue;
      }
      const targetPath = path.join(outRoot, locale, sourceEntry.relativePath);
      fs.mkdirSync(path.dirname(targetPath), { recursive: true });
      const sourceFilePath = `site/content/${canonicalLanguage}/${sourceEntry.relativePath}`;
      const targetFilePath = `site/content/${locale}/${sourceEntry.relativePath}`;
      fs.writeFileSync(targetPath, renderFallbackPageSource(sourceEntry.source, {
        sourceLocale: canonicalLanguage,
        targetLocale: locale,
        helpUrl: pageManifest[TRANSLATION_HELP_ROUTE_KEY]?.[locale]
          ?? pageManifest[TRANSLATION_HELP_ROUTE_KEY]?.[canonicalLanguage]
          ?? joinPath(contentLanguage.pathPrefix, TRANSLATION_HELP_ROUTE_KEY),
        sourceFileUrl: buildGitHubBlobUrl(GITHUB_REPOSITORY_URL, GITHUB_DEFAULT_BRANCH, sourceFilePath),
        createFileUrl: buildGitHubNewFileUrl(GITHUB_REPOSITORY_URL, GITHUB_DEFAULT_BRANCH, targetFilePath),
        targetPath: targetFilePath,
        targetPathPrefix: contentLanguage.pathPrefix,
        sourcePath: pageManifest[sourceEntry.routeKey]?.[canonicalLanguage]
          ?? joinPath(sourceLanguage.pathPrefix, sourceEntry.routeKey),
        pageManifest,
      }), "utf8");
    }
  }
  for (const contentLanguage of config.contentLanguages) {
    const localeDir = path.join(outRoot, contentLanguage.code);
    const ogImageHref = contentLanguage.code === config.defaultContentLang
      ? "/img/embed.png"
      : joinPath(contentLanguage.pathPrefix, "/embed.png");
    for (const filePath of listFiles(localeDir)) {
      if (!filePath.endsWith(".smd")) {
        continue;
      }
      const source = fs.readFileSync(filePath, "utf8");
      fs.writeFileSync(filePath, applyDefaultOgImage(source, ogImageHref), "utf8");
    }
  }
  return { outRoot, entries: shellEntries, fallbackGroups: contentGroups.size };
}

const isMainModule = process.argv[1] && path.resolve(process.argv[1]) === scriptPath;

if (isMainModule) {
  try {
    const args = parseArgs(process.argv.slice(2));
    buildShellContentTree({
      outRoot: path.resolve(siteDir, args.outRoot),
    });
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
