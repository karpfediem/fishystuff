import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { resolveBrandAssets } from "./brand-assets.mjs";
import { buildPageManifest } from "./build-i18n.mjs";
import { buildDocumentTitle, resolveDocumentTitleSuffix } from "./document-title.mjs";
import { LANGUAGE_CONFIG } from "./language-config.mjs";
import { joinUrl, resolvePublicBaseUrls } from "./write-runtime-config.mjs";
import { buildShellPageEntries, buildShellPagePathSet, renderShellPageSource } from "./shell-pages.mjs";

const scriptPath = fileURLToPath(import.meta.url);
const siteDir = path.resolve(path.dirname(scriptPath), "..");
const TRANSLATION_HELP_ROUTE_KEY = "/community/";
const GITHUB_REPOSITORY_URL = "https://github.com/karpfediem/fishystuff";
const GITHUB_DEFAULT_BRANCH = "main";
const DEFAULT_PAGE_DESCRIPTION = "Fishy Stuff: Fishing Guides and Tools for Black Desert";

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
  return JSON.stringify(String(value ?? ""));
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

function parseTitle(source) {
  const { frontmatter } = parseFrontmatterParts(source);
  const match = frontmatter.match(/^\s*\.title\s*=\s*("(?:\\.|[^"])*")\s*,?\s*$/m);
  if (!match) {
    return "";
  }
  try {
    return JSON.parse(match[1]);
  } catch {
    return trimString(match[1].slice(1, -1));
  }
}

function parseStringField(source, fieldName) {
  const { frontmatter } = parseFrontmatterParts(source);
  const match = frontmatter.match(
    new RegExp(`^\\s*\\.${fieldName}\\s*=\\s*("(?:\\\\.|[^"])*")\\s*,?\\s*$`, "m"),
  );
  if (!match) {
    return "";
  }
  try {
    return JSON.parse(match[1]);
  } catch {
    return trimString(match[1].slice(1, -1));
  }
}

function resolvePageDescription(source) {
  return parseStringField(source, "description") || DEFAULT_PAGE_DESCRIPTION;
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

function applyDefaultBrandAssets(source, brandAssets) {
  if (!brandAssets || brandAssets.variant === "default") {
    return source;
  }

  const fields = [];
  if (!pageSourceHasFrontmatterField(source, "brand_logo_url")) {
    fields.push({ key: ".brand_logo_url", value: brandAssets.heroLogoUrl });
  }
  if (!pageSourceHasFrontmatterField(source, "brand_logo_nav_url")) {
    fields.push({ key: ".brand_logo_nav_url", value: brandAssets.navLogoUrl });
  }
  if (!pageSourceHasFrontmatterField(source, "brand_logo_nav_srcset")) {
    fields.push({ key: ".brand_logo_nav_srcset", value: brandAssets.navLogoSrcset });
  }
  if (fields.length === 0) {
    return source;
  }

  const { frontmatter, body } = parseFrontmatterParts(source);
  return `---\n${mergeCustomFields(frontmatter, fields)}\n---\n${body}`;
}

function applyDefaultDocumentTitle(source, documentTitleSuffix) {
  if (pageSourceHasFrontmatterField(source, "document_title")) {
    return source;
  }

  const pageTitle = parseTitle(source);
  if (!pageTitle) {
    return source;
  }

  const { frontmatter, body } = parseFrontmatterParts(source);
  return `---\n${mergeCustomFields(frontmatter, [
    { key: ".document_title", value: buildDocumentTitle(pageTitle, documentTitleSuffix) },
  ])}\n---\n${body}`;
}

function localeToOgLocale(locale) {
  const normalized = trimString(locale);
  if (!normalized) {
    return "";
  }
  return normalized.replace(/-/g, "_");
}

function buildHreflangLinksHtml(routeKey, pageManifest, config, publicSiteBaseUrl) {
  const localizedPaths = pageManifest[routeKey];
  if (!localizedPaths) {
    return "";
  }

  const links = [];
  for (const contentLanguage of config.contentLanguages) {
    const href = localizedPaths[contentLanguage.code];
    if (!href) {
      continue;
    }
    links.push(
      `<link rel="alternate" hreflang="${contentLanguage.code}" href="${joinUrl(publicSiteBaseUrl, href)}">`,
    );
  }

  const defaultHref = localizedPaths[config.defaultContentLang];
  if (defaultHref) {
    links.push(
      `<link rel="alternate" hreflang="x-default" href="${joinUrl(publicSiteBaseUrl, defaultHref)}">`,
    );
  }

  return links.join("\n");
}

function buildOgLocaleAlternateHtml(currentLocale, routeKey, pageManifest, config) {
  const localizedPaths = pageManifest[routeKey];
  if (!localizedPaths) {
    return "";
  }

  return config.contentLanguages
    .map((contentLanguage) => contentLanguage.code)
    .filter((locale) => locale !== currentLocale && localizedPaths[locale])
    .map((locale) => `<meta property="og:locale:alternate" content="${localeToOgLocale(locale)}">`)
    .join("\n");
}

function isAbsoluteUrl(value) {
  return /^https?:\/\//i.test(trimString(value));
}

function resolveSiteRelativeAssetPath(rootDir, href) {
  const normalized = trimString(href).replace(/^\/+/, "");
  if (!normalized || isAbsoluteUrl(normalized) || normalized.startsWith("data:")) {
    return "";
  }
  return path.join(rootDir, "assets", normalized);
}

function readPngDimensions(buffer) {
  if (buffer.length < 24 || buffer.toString("ascii", 1, 4) !== "PNG") {
    return null;
  }
  return {
    width: buffer.readUInt32BE(16),
    height: buffer.readUInt32BE(20),
  };
}

function readWebpDimensions(buffer) {
  if (buffer.length < 30 || buffer.toString("ascii", 0, 4) !== "RIFF" || buffer.toString("ascii", 8, 12) !== "WEBP") {
    return null;
  }

  const chunkType = buffer.toString("ascii", 12, 16);
  if (chunkType === "VP8X" && buffer.length >= 30) {
    return {
      width: 1 + buffer.readUIntLE(24, 3),
      height: 1 + buffer.readUIntLE(27, 3),
    };
  }

  if (chunkType === "VP8 " && buffer.length >= 30) {
    return {
      width: buffer.readUInt16LE(26) & 0x3fff,
      height: buffer.readUInt16LE(28) & 0x3fff,
    };
  }

  if (chunkType === "VP8L" && buffer.length >= 25) {
    const bits = buffer.readUInt32LE(21);
    return {
      width: (bits & 0x3fff) + 1,
      height: ((bits >> 14) & 0x3fff) + 1,
    };
  }

  return null;
}

function readJpegDimensions(buffer) {
  if (buffer.length < 4 || buffer[0] !== 0xff || buffer[1] !== 0xd8) {
    return null;
  }

  let offset = 2;
  while (offset + 8 < buffer.length) {
    if (buffer[offset] !== 0xff) {
      offset += 1;
      continue;
    }
    const marker = buffer[offset + 1];
    const segmentLength = buffer.readUInt16BE(offset + 2);
    if (
      (marker >= 0xc0 && marker <= 0xc3)
      || (marker >= 0xc5 && marker <= 0xc7)
      || (marker >= 0xc9 && marker <= 0xcb)
      || (marker >= 0xcd && marker <= 0xcf)
    ) {
      return {
        height: buffer.readUInt16BE(offset + 5),
        width: buffer.readUInt16BE(offset + 7),
      };
    }
    if (segmentLength < 2) {
      break;
    }
    offset += 2 + segmentLength;
  }

  return null;
}

function readImageDimensions(filePath) {
  if (!filePath || !fs.existsSync(filePath)) {
    return null;
  }

  const buffer = fs.readFileSync(filePath);
  return readPngDimensions(buffer) || readWebpDimensions(buffer) || readJpegDimensions(buffer);
}

function resolveImageMimeType(...candidates) {
  const mimeTypesByExtension = new Map([
    [".png", "image/png"],
    [".webp", "image/webp"],
    [".jpg", "image/jpeg"],
    [".jpeg", "image/jpeg"],
    [".gif", "image/gif"],
    [".svg", "image/svg+xml"],
  ]);

  for (const candidate of candidates) {
    const normalized = trimString(candidate).split(/[?#]/, 1)[0];
    if (!normalized) {
      continue;
    }
    const mimeType = mimeTypesByExtension.get(path.extname(normalized).toLowerCase());
    if (mimeType) {
      return mimeType;
    }
  }

  return "";
}

function resolveOgImageInfo({
  source,
  filePath,
  rootDir,
  fallbackHref,
  pageTitle,
  resolvedDescription,
}) {
  const ogImageAsset = parseStringField(source, "og_image_asset");
  if (ogImageAsset) {
    return {
      filePath: path.join(path.dirname(filePath), ogImageAsset),
      href: ogImageAsset,
      alt: pageTitle || resolvedDescription,
    };
  }

  const ogImageHref = parseStringField(source, "og_image");
  if (ogImageHref) {
    return {
      filePath: ogImageHref.startsWith("/")
        ? resolveSiteRelativeAssetPath(rootDir, ogImageHref)
        : path.join(path.dirname(filePath), ogImageHref),
      href: ogImageHref,
      alt: pageTitle || resolvedDescription,
    };
  }

  return {
    filePath: resolveSiteRelativeAssetPath(rootDir, fallbackHref),
    href: fallbackHref,
    alt: resolvedDescription,
  };
}

function applyDefaultSeoMetadata({
  source,
  filePath,
  routeKey,
  locale,
  config,
  pageManifest,
  rootDir,
  publicSiteBaseUrl,
  ogImageHref,
}) {
  const pageTitle = parseTitle(source);
  const resolvedDescription = resolvePageDescription(source);
  const ogLocale = localeToOgLocale(locale);
  const hreflangLinksHtml = buildHreflangLinksHtml(routeKey, pageManifest, config, publicSiteBaseUrl);
  const ogLocaleAlternateHtml = buildOgLocaleAlternateHtml(locale, routeKey, pageManifest, config);
  const ogImageInfo = resolveOgImageInfo({
    source,
    filePath,
    rootDir,
    fallbackHref: ogImageHref,
    pageTitle,
    resolvedDescription,
  });
  const ogImageDimensions = readImageDimensions(ogImageInfo.filePath);
  const ogImageType = resolveImageMimeType(ogImageInfo.filePath, ogImageInfo.href);

  const fields = [];
  if (!pageSourceHasFrontmatterField(source, "resolved_description")) {
    fields.push({ key: ".resolved_description", value: resolvedDescription });
  }
  if (!pageSourceHasFrontmatterField(source, "hreflang_links_html")) {
    fields.push({ key: ".hreflang_links_html", value: hreflangLinksHtml });
  }
  if (!pageSourceHasFrontmatterField(source, "og_locale")) {
    fields.push({ key: ".og_locale", value: ogLocale });
  }
  if (!pageSourceHasFrontmatterField(source, "og_locale_alternate_html")) {
    fields.push({ key: ".og_locale_alternate_html", value: ogLocaleAlternateHtml });
  }
  if (!pageSourceHasFrontmatterField(source, "og_image_alt")) {
    fields.push({ key: ".og_image_alt", value: ogImageInfo.alt });
  }
  if (ogImageType && !pageSourceHasFrontmatterField(source, "og_image_type")) {
    fields.push({ key: ".og_image_type", value: ogImageType });
  }
  if (ogImageDimensions && !pageSourceHasFrontmatterField(source, "og_image_width")) {
    fields.push({ key: ".og_image_width", value: String(ogImageDimensions.width) });
  }
  if (ogImageDimensions && !pageSourceHasFrontmatterField(source, "og_image_height")) {
    fields.push({ key: ".og_image_height", value: String(ogImageDimensions.height) });
  }
  if (fields.length === 0) {
    return source;
  }

  const { frontmatter, body } = parseFrontmatterParts(source);
  return `---\n${mergeCustomFields(frontmatter, fields)}\n---\n${body}`;
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
  env = process.env,
} = {}) {
  const shellEntries = buildShellPageEntries({ config, rootDir });
  const shellPathsByLocale = buildShellPagePathSet({ config, rootDir });
  const pageManifest = buildPageManifest(config, rootDir);
  const brandAssets = resolveBrandAssets(env);
  const documentTitleSuffix = resolveDocumentTitleSuffix(env);
  const { publicSiteBaseUrl } = resolvePublicBaseUrls(env);
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
      const relativePath = path.relative(localeDir, filePath);
      const routeKey = normalizeRouteKey(relativePath);
      const source = fs.readFileSync(filePath, "utf8");
      const branded = applyDefaultBrandAssets(source, brandAssets);
      const titled = applyDefaultDocumentTitle(branded, documentTitleSuffix);
      const withOgImage = applyDefaultOgImage(titled, ogImageHref);
      const withSeoMetadata = applyDefaultSeoMetadata({
        source: withOgImage,
        filePath,
        routeKey,
        locale: contentLanguage.code,
        config,
        pageManifest,
        rootDir,
        publicSiteBaseUrl,
        ogImageHref,
      });
      fs.writeFileSync(filePath, withSeoMetadata, "utf8");
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
