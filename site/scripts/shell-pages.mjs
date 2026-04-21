import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { LANGUAGE_CONFIG } from "./language-config.mjs";

const scriptPath = fileURLToPath(import.meta.url);
const siteDir = path.resolve(path.dirname(scriptPath), "..");
const shellPageConfigPath = path.join(siteDir, "i18n", "shell-pages.json");

function trimString(value) {
  return String(value ?? "").trim();
}

function normalizeSlug(slug) {
  return trimString(slug).replace(/^\/+|\/+$/g, "");
}

function renderQuoted(value) {
  return `"${String(value ?? "").replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
}

function relativePathForSlug(slug) {
  const normalizedSlug = normalizeSlug(slug);
  return normalizedSlug ? `${normalizedSlug}.smd` : "index.smd";
}

function routeKeyForSlug(slug) {
  const normalizedSlug = normalizeSlug(slug);
  return normalizedSlug ? `/${normalizedSlug}/` : "/";
}

function assertArray(value, label) {
  if (!Array.isArray(value)) {
    throw new Error(`${label} must be an array`);
  }
  return value;
}

function assertObject(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} must be an object`);
  }
  return value;
}

function loadShellPageSource(rootDir) {
  const configPath = path.join(rootDir, "i18n", "shell-pages.json");
  if (!fs.existsSync(configPath)) {
    return { pages: [] };
  }
  return JSON.parse(fs.readFileSync(configPath, "utf8"));
}

export function loadShellPages(rootDir = siteDir) {
  const source = loadShellPageSource(rootDir);
  const pages = assertArray(source.pages ?? [], "shell page config pages");
  return Object.freeze({
    pages: pages.map((page, index) => {
      const record = assertObject(page, `shell page ${index}`);
      const pageId = trimString(record.id);
      if (!pageId) {
        throw new Error(`shell page ${index} is missing id`);
      }
      const layout = trimString(record.layout);
      if (!layout) {
        throw new Error(`shell page ${pageId} is missing layout`);
      }
      const author = trimString(record.author);
      if (!author) {
        throw new Error(`shell page ${pageId} is missing author`);
      }
      const date = trimString(record.date);
      if (!date) {
        throw new Error(`shell page ${pageId} is missing date`);
      }
      const locales = assertObject(record.locales ?? {}, `shell page ${pageId} locales`);
      const tags = record.tags === undefined ? [] : assertArray(record.tags, `shell page ${pageId} tags`);
      return Object.freeze({
        id: pageId,
        layout,
        author,
        date,
        draft: Boolean(record.draft),
        translationKey: trimString(record.translationKey),
        tags: Object.freeze(tags.map((tag) => trimString(tag)).filter(Boolean)),
        updated: trimString(record.updated),
        locales: Object.freeze(
          Object.fromEntries(
            Object.entries(locales).map(([locale, localeRecord]) => {
              const normalizedLocale = trimString(locale);
              const normalizedRecord = assertObject(localeRecord, `shell page ${pageId} locale ${locale}`);
              const title = trimString(normalizedRecord.title);
              if (!title) {
                throw new Error(`shell page ${pageId} locale ${normalizedLocale} is missing title`);
              }
              return [normalizedLocale, Object.freeze({
                slug: normalizeSlug(normalizedRecord.slug),
                title,
                description: trimString(normalizedRecord.description),
              })];
            }),
          ),
        ),
      });
    }),
  });
}

export function buildShellPageEntries({
  config = LANGUAGE_CONFIG,
  rootDir = siteDir,
} = {}) {
  const localeCodes = new Set(config.contentLanguages.map((language) => trimString(language.code)));
  const entries = [];
  for (const page of loadShellPages(rootDir).pages) {
    for (const [locale, localeRecord] of Object.entries(page.locales)) {
      if (!localeCodes.has(locale)) {
        continue;
      }
      entries.push(Object.freeze({
        pageId: page.id,
        locale,
        relativePath: relativePathForSlug(localeRecord.slug),
        routeKey: routeKeyForSlug(localeRecord.slug),
        title: localeRecord.title,
        description: localeRecord.description,
        layout: page.layout,
        author: page.author,
        date: page.date,
        draft: page.draft,
        translationKey: page.translationKey,
        tags: page.tags,
        updated: page.updated,
      }));
    }
  }
  return entries.sort((left, right) => {
    if (left.locale !== right.locale) {
      return left.locale.localeCompare(right.locale);
    }
    return left.relativePath.localeCompare(right.relativePath);
  });
}

export function buildShellPagePathSet({
  config = LANGUAGE_CONFIG,
  rootDir = siteDir,
} = {}) {
  const paths = new Map();
  for (const entry of buildShellPageEntries({ config, rootDir })) {
    if (!paths.has(entry.locale)) {
      paths.set(entry.locale, new Set());
    }
    paths.get(entry.locale).add(entry.relativePath);
  }
  return paths;
}

export function renderShellPageSource(entry) {
  const lines = [
    "---",
    `.title = ${renderQuoted(entry.title)},`,
  ];
  if (entry.description) {
    lines.push(`.description = ${renderQuoted(entry.description)},`);
  }
  lines.push(`.date = @date(${renderQuoted(entry.date)}),`);
  lines.push(`.author = ${renderQuoted(entry.author)},`);
  lines.push(`.layout = ${renderQuoted(entry.layout)},`);
  if (entry.translationKey) {
    lines.push(`.translation_key = ${renderQuoted(entry.translationKey)},`);
  }
  lines.push(`.draft = ${entry.draft ? "true" : "false"},`);
  if (entry.tags.length) {
    lines.push(`.tags = [${entry.tags.map(renderQuoted).join(", ")}],`);
  }
  if (entry.updated) {
    lines.push(`.custom = { .updated = @date(${renderQuoted(entry.updated)}) },`);
  }
  lines.push("---", "");
  return lines.join("\n");
}

export { normalizeSlug, relativePathForSlug, routeKeyForSlug, shellPageConfigPath };
