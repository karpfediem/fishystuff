import { bookmarkDisplayLabel, normalizeBookmarks } from "./map-bookmark-state.js";

function escapeXml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

function unescapeXml(value) {
  return String(value ?? "")
    .replaceAll("&quot;", '"')
    .replaceAll("&apos;", "'")
    .replaceAll("&gt;", ">")
    .replaceAll("&lt;", "<")
    .replaceAll("&amp;", "&");
}

function formatBookmarkCoordinate(value) {
  const normalized = Number(value);
  if (!Number.isFinite(normalized)) {
    return "";
  }
  return Number.isInteger(normalized) ? normalized.toFixed(1) : String(normalized);
}

function pluralizeBookmarks(count) {
  return count === 1 ? "bookmark" : "bookmarks";
}

function parseBookmarkXmlAttributes(nodeText) {
  const attributes = {};
  const attributePattern = /([A-Za-z_:][A-Za-z0-9:._-]*)\s*=\s*(?:"([^"]*)"|'([^']*)')/g;
  for (const match of String(nodeText || "").matchAll(attributePattern)) {
    attributes[match[1]] = unescapeXml(match[2] ?? match[3] ?? "");
  }
  return attributes;
}

function normalizeBookmarkLabelFromXml(label, index) {
  const trimmedLabel = String(label || "").trim().replace(/^\d+\s*:\s*/, "").trim();
  return trimmedLabel || `Bookmark ${index + 1}`;
}

function bookmarkMergeKey(bookmark, index = 0) {
  const normalized = normalizeBookmarks([bookmark])[0];
  if (!normalized) {
    return "";
  }
  return [
    String(bookmarkDisplayLabel(normalized, index) || "").trim().toLowerCase(),
    formatBookmarkCoordinate(normalized.worldX),
    formatBookmarkCoordinate(normalized.worldZ),
  ].join("\u0000");
}

function buildBookmarkExportFilename(timestamp = Date.now()) {
  const date = new Date(timestamp);
  const year = date.getUTCFullYear();
  const month = String(date.getUTCMonth() + 1).padStart(2, "0");
  const day = String(date.getUTCDate()).padStart(2, "0");
  const hours = String(date.getUTCHours()).padStart(2, "0");
  const minutes = String(date.getUTCMinutes()).padStart(2, "0");
  const seconds = String(date.getUTCSeconds()).padStart(2, "0");
  return `fishystuff-map-bookmarks-${year}${month}${day}-${hours}${minutes}${seconds}.xml`;
}

export function serializeBookmarksForExport(bookmarks) {
  const normalized = normalizeBookmarks(bookmarks);
  const lines = [
    "<WorldmapBookMark>",
    ...normalized.map(
      (bookmark, index) =>
        `\t<BookMark BookMarkName="${escapeXml(`${index + 1}: ${bookmarkDisplayLabel(bookmark, index)}`)}" PosX="${escapeXml(formatBookmarkCoordinate(bookmark.worldX))}" PosY="0.0" PosZ="${escapeXml(formatBookmarkCoordinate(bookmark.worldZ))}" />`,
    ),
    "</WorldmapBookMark>",
  ];
  return lines.join("\n");
}

export function parseImportedBookmarks(serializedBookmarks, options = {}) {
  const serialized = String(serializedBookmarks || "").trim();
  if (!serialized) {
    return [];
  }

  const idFactory =
    typeof options.idFactory === "function"
      ? options.idFactory
      : (() => {
          if (globalThis.crypto?.randomUUID) {
            return globalThis.crypto.randomUUID();
          }
          return `bookmark-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
        });

  const xmlNodes = Array.from(serialized.matchAll(/<BookMark\b[^>]*\/?>/gi));
  if (xmlNodes.length) {
    return normalizeBookmarks(
      xmlNodes.map((match, index) => {
        const attributes = parseBookmarkXmlAttributes(match[0]);
        return {
          id: idFactory(),
          label: normalizeBookmarkLabelFromXml(attributes.BookMarkName, index),
          worldX: attributes.PosX,
          worldZ: attributes.PosZ,
        };
      }),
    );
  }

  return normalizeBookmarks(JSON.parse(serialized));
}

export function mergeImportedBookmarks(existingBookmarks, importedBookmarks) {
  const merged = normalizeBookmarks(existingBookmarks);
  const seenKeys = new Set(merged.map((bookmark, index) => bookmarkMergeKey(bookmark, index)).filter(Boolean));
  for (const bookmark of normalizeBookmarks(importedBookmarks)) {
    const key = bookmarkMergeKey(bookmark, merged.length);
    if (!key || seenKeys.has(key)) {
      continue;
    }
    seenKeys.add(key);
    merged.push(bookmark);
  }
  return merged;
}

export async function copyTextToClipboard(text, options = {}) {
  const navigatorObject = options.navigator ?? globalThis.navigator;
  if (navigatorObject?.clipboard?.writeText) {
    await navigatorObject.clipboard.writeText(String(text ?? ""));
    return;
  }

  const doc = options.document ?? globalThis.document;
  if (!doc?.createElement || !doc?.body?.appendChild) {
    throw new Error("Clipboard API unavailable");
  }
  const probe = doc.createElement("textarea");
  probe.value = String(text ?? "");
  probe.setAttribute("readonly", "");
  probe.style.position = "fixed";
  probe.style.opacity = "0";
  probe.style.pointerEvents = "none";
  doc.body.appendChild(probe);
  probe.select();
  probe.setSelectionRange(0, probe.value.length);
  const copied = doc.execCommand?.("copy");
  probe.remove();
  if (!copied) {
    throw new Error("Clipboard API unavailable");
  }
}

export function downloadBookmarkExport(bookmarks, options = {}) {
  const doc = options.document ?? globalThis.document;
  const urlApi = options.url ?? globalThis.URL;
  const blobCtor = options.Blob ?? globalThis.Blob;
  if (
    !doc?.createElement ||
    !doc?.body?.appendChild ||
    typeof blobCtor !== "function" ||
    typeof urlApi?.createObjectURL !== "function"
  ) {
    throw new Error("Bookmark export is unavailable");
  }
  const timestamp = Number.isFinite(options.now) ? options.now : Date.now();
  const anchor = doc.createElement("a");
  const href = urlApi.createObjectURL(
    new blobCtor([serializeBookmarksForExport(bookmarks)], {
      type: "application/xml",
    }),
  );
  anchor.href = href;
  anchor.download = buildBookmarkExportFilename(timestamp);
  anchor.rel = "noopener";
  anchor.hidden = true;
  doc.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  globalThis.setTimeout?.(() => {
    urlApi.revokeObjectURL?.(href);
  }, 0);
}

export async function readBookmarkImportFile(file, options = {}) {
  if (typeof file?.text === "function") {
    return file.text();
  }
  const readerCtor = options.FileReader ?? globalThis.FileReader;
  if (typeof readerCtor !== "function") {
    throw new Error("Bookmark import is unavailable");
  }
  return new Promise((resolve, reject) => {
    const reader = new readerCtor();
    reader.onerror = () => reject(reader.error || new Error("Failed to read bookmark import"));
    reader.onload = () => resolve(String(reader.result ?? ""));
    reader.readAsText(file);
  });
}

export function buildBookmarkSelectionCopyMessage(count) {
  return `Copied XML for ${count} ${pluralizeBookmarks(count)}.`;
}

export function buildBookmarkExportMessage(count, selectedCount = 0) {
  return selectedCount
    ? `Exported ${count} selected ${pluralizeBookmarks(count)}.`
    : `Exported ${count} ${pluralizeBookmarks(count)}.`;
}

export function buildBookmarkImportMessage(importedCount, skippedCount = 0) {
  if (!importedCount) {
    return "No new bookmarks were imported.";
  }
  return `Imported ${importedCount} ${pluralizeBookmarks(importedCount)}${
    skippedCount ? `; skipped ${skippedCount} duplicate${skippedCount === 1 ? "" : "s"}.` : "."
  }`;
}
