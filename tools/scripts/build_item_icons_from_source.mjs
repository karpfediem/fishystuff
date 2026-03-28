#!/usr/bin/env node

import { existsSync, mkdirSync, statSync } from "node:fs";
import { mkdtempSync, rmSync } from "node:fs";
import { spawnSync } from "node:child_process";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const scriptDir = path.dirname(scriptPath);
const repoRoot = path.resolve(scriptDir, "../..");
const defaultSourceArchive = path.join(repoRoot, "data/scratch/paz");
const defaultOutputDir = path.join(repoRoot, "data/cdn/public/images/items");
const defaultCalculatorApiUrl =
  process.env.FISHYSTUFF_CALCULATOR_API_URL?.trim() || "http://127.0.0.1:8080/api/v1/calculator";
const iconSize = 44;
const webpQuality = 86;
const scriptMtimeMs = statSync(scriptPath).mtimeMs;

function fail(message) {
  throw new Error(message);
}

function parseArgs(argv) {
  const options = {
    force: false,
    quiet: false,
    outputDir: defaultOutputDir,
    sourceArchive: defaultSourceArchive,
    calculatorApiUrl: defaultCalculatorApiUrl,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--force") {
      options.force = true;
      continue;
    }
    if (arg === "--quiet") {
      options.quiet = true;
      continue;
    }
    if (arg === "--output-dir") {
      index += 1;
      options.outputDir = argv[index] ? path.resolve(argv[index]) : null;
      continue;
    }
    if (arg === "--source-archive") {
      index += 1;
      options.sourceArchive = argv[index] ? path.resolve(argv[index]) : null;
      continue;
    }
    if (arg === "--calculator-api-url") {
      index += 1;
      options.calculatorApiUrl = argv[index] ? String(argv[index]).trim() : null;
      continue;
    }
    fail(`unknown argument: ${arg}`);
  }

  if (!options.outputDir) {
    fail("--output-dir requires a value");
  }
  if (!options.sourceArchive) {
    fail("--source-archive requires a value");
  }
  if (!options.calculatorApiUrl) {
    options.calculatorApiUrl = "";
  }

  return options;
}

function runCommand(command, args, { capture = true } = {}) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: capture ? ["ignore", "pipe", "pipe"] : "inherit",
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    const details = capture
      ? [result.stdout, result.stderr].filter(Boolean).join("\n").trim()
      : "";
    throw new Error(
      `${command} ${args.join(" ")} failed with exit code ${result.status}${details ? `\n${details}` : ""}`,
    );
  }
  return capture ? result.stdout : "";
}

function doltQueryJson(sql) {
  const output = runCommand("dolt", ["sql", "-r", "json", "-q", sql]);
  const parsed = JSON.parse(output);
  return parsed.rows ?? [];
}

function padIconId(iconId) {
  return String(iconId).padStart(8, "0");
}

function outputPathForIcon(outputDir, iconId) {
  return path.join(outputDir, `${padIconId(iconId)}.webp`);
}

function shouldBuild(outputPath, force) {
  if (force || !existsSync(outputPath)) {
    return true;
  }
  return statSync(outputPath).mtimeMs < scriptMtimeMs;
}

function normalizeArchivePath(rawPath) {
  if (!rawPath) {
    return null;
  }
  const normalized = rawPath.trim().replaceAll("\\", "/").toLowerCase();
  if (!normalized.endsWith(".dds")) {
    return null;
  }
  if (normalized.startsWith("ui_texture/")) {
    return normalized;
  }
  if (normalized.startsWith("new_icon/") || normalized.startsWith("quest/")) {
    return `ui_texture/icon/${normalized}`;
  }
  if (normalized.startsWith("icon/")) {
    return `ui_texture/${normalized}`;
  }
  return null;
}

function parseIconIdFromSourcePath(rawPath) {
  if (!rawPath) {
    return null;
  }
  const match = rawPath.match(/(\d{5,8})(?:_[0-9]+)?\.dds$/i);
  return match ? Number(match[1]) : null;
}

function parseIconIdFromAssetName(rawPath) {
  if (!rawPath) {
    return null;
  }
  const file = String(rawPath).trim().split(/[?#]/, 1)[0];
  const basename = file.split("/").pop() ?? file;
  const stem = basename.replace(/\.[^.]+$/, "");
  const digits = [...stem].filter((ch) => ch >= "0" && ch <= "9").join("");
  if (!digits) {
    return null;
  }
  const parsed = Number(digits);
  return Number.isFinite(parsed) ? parsed : null;
}

function parseArchiveMatches(listingText) {
  const matches = [];
  for (const line of listingText.split(/\r?\n/)) {
    const match = line.match(/^\[[^\]]+\]\s+(.+?)\s+\(size:\s*(\d+)\)$/);
    if (!match) {
      continue;
    }
    matches.push({
      path: match[1],
      size: Number(match[2]),
    });
  }
  return matches;
}

function listArchiveMatches(sourceArchive, filters) {
  if (filters.length === 0) {
    return [];
  }
  const args = ["run", "-q", "-p", "pazifista", "--", sourceArchive, "-l"];
  for (const filter of filters) {
    args.push("-f", filter);
  }
  return parseArchiveMatches(runCommand("cargo", args));
}

function scoreArchivePath(match) {
  let score = 0;
  const archivePath = match.path.toLowerCase();

  if (archivePath.includes("/new_icon/06_pc_equipitem/")) {
    score += 500;
  } else if (archivePath.includes("/new_icon/03_etc/")) {
    score += 400;
  } else if (archivePath.includes("/new_icon/09_cash/03_product/")) {
    score += 350;
  } else if (archivePath.includes("/new_icon/")) {
    score += 250;
  }

  if (archivePath.includes("/quest/")) {
    score -= 1000;
  }
  if (/_\d+\.dds$/i.test(archivePath)) {
    score -= 50;
  }

  score += Math.min(match.size, 20000) / 1000;
  score -= archivePath.length / 1000;
  return score;
}

function chooseBestArchiveMatch(matches) {
  return [...matches].sort((left, right) => scoreArchivePath(right) - scoreArchivePath(left))[0] ?? null;
}

function addIconTarget(targets, row) {
  const rawSourcePath = row.item_icon_file ?? row.skill_icon_file ?? null;
  const iconId =
    Number(row.icon_id) ||
    parseIconIdFromAssetName(rawSourcePath) ||
    Number(row.item_id) ||
    null;
  if (!Number.isFinite(iconId) || iconId <= 0) {
    return;
  }

  const existing = targets.get(iconId) ?? {
    iconId,
    displayName: row.display_name || row.source_name_en || row.set_name_ko || `icon:${iconId}`,
    sourcePath: null,
  };
  const normalizedSourcePath = normalizeArchivePath(rawSourcePath);
  if (normalizedSourcePath) {
    existing.sourcePath = normalizedSourcePath;
  }
  targets.set(iconId, existing);
}

function queryLegacyIconRows() {
  return doltQueryJson(`
    SELECT DISTINCT
      CAST(i.icon_id AS SIGNED) AS icon_id,
      CAST(i.id AS SIGNED) AS item_id,
      NULLIF(TRIM(i.name), '') AS display_name,
      NULLIF(TRIM(it.IconImageFile), '') AS item_icon_file
    FROM items i
    LEFT JOIN item_table it
      ON CAST(it.Index AS SIGNED) = CAST(i.id AS SIGNED)
    WHERE i.icon_id IS NOT NULL
    ORDER BY CAST(i.icon_id AS SIGNED)
  `);
}

function queryItemMetadataRowsByIds(itemIds) {
  if (itemIds.length === 0) {
    return [];
  }
  const idList = [...new Set(itemIds.filter((value) => Number.isFinite(value) && value > 0))]
    .sort((left, right) => left - right)
    .join(",");
  if (!idList) {
    return [];
  }
  return doltQueryJson(`
    SELECT DISTINCT
      CAST(it.Index AS SIGNED) AS item_id,
      NULLIF(TRIM(it.ItemName), '') AS display_name,
      NULLIF(TRIM(it.IconImageFile), '') AS item_icon_file
    FROM item_table it
    WHERE CAST(it.Index AS SIGNED) IN (${idList})
    ORDER BY CAST(it.Index AS SIGNED)
  `);
}

function queryCalculatorApiIconRows(calculatorApiUrl) {
  if (!calculatorApiUrl) {
    return [];
  }
  let parsed;
  try {
    parsed = JSON.parse(runCommand("curl", ["-sS", calculatorApiUrl]));
  } catch {
    return [];
  }

  const apiItems = Array.isArray(parsed?.items) ? parsed.items : [];
  const metadataByItemId = new Map(
    queryItemMetadataRowsByIds(
      apiItems
        .map((item) => Number(item?.item_id))
        .filter((itemId) => Number.isFinite(itemId) && itemId > 0),
    ).map((row) => [Number(row.item_id), row]),
  );

  return apiItems
    .map((item) => {
      const itemId = Number(item?.item_id);
      const metadata = Number.isFinite(itemId) ? metadataByItemId.get(itemId) : null;
      return {
        icon_id: item?.icon_id ?? null,
        item_id: Number.isFinite(itemId) ? itemId : null,
        display_name: item?.name ?? metadata?.display_name ?? null,
        item_icon_file: metadata?.item_icon_file ?? null,
      };
    })
    .filter((row) => row.icon_id != null || row.item_icon_file || row.item_id != null);
}

function queryConsumableIconRows() {
  return doltQueryJson(`
    SELECT DISTINCT
      CAST(item_id AS SIGNED) AS item_id,
      NULLIF(TRIM(item_name_ko), '') AS display_name,
      NULLIF(TRIM(item_icon_file), '') AS item_icon_file
    FROM calculator_consumable_effect_sources
    WHERE item_id IS NOT NULL
    ORDER BY CAST(item_id AS SIGNED)
  `);
}

function queryEnchantItemIconRows() {
  return doltQueryJson(`
    SELECT DISTINCT
      CAST(it.Index AS SIGNED) AS item_id,
      NULLIF(TRIM(it.ItemName), '') AS display_name,
      NULLIF(TRIM(it.IconImageFile), '') AS item_icon_file
    FROM calculator_enchant_item_metadata em
    JOIN item_table it
      ON CAST(it.Index AS SIGNED) = CAST(em.item_id AS SIGNED)
    ORDER BY CAST(it.Index AS SIGNED)
  `);
}

function queryLightstoneIconRows() {
  return doltQueryJson(`
    SELECT DISTINCT
      source_name_en AS display_name,
      set_name_ko,
      skill_icon_file
    FROM calculator_lightstone_effect_sources
    WHERE NULLIF(TRIM(skill_icon_file), '') IS NOT NULL
  `);
}

function queryCalculatorIconTargets(calculatorApiUrl) {
  const targets = new Map();
  const apiRows = queryCalculatorApiIconRows(calculatorApiUrl);
  if (apiRows.length > 0) {
    for (const row of apiRows) {
      addIconTarget(targets, row);
    }
    return [...targets.values()];
  }
  for (const row of queryLegacyIconRows()) {
    addIconTarget(targets, row);
  }
  for (const row of queryConsumableIconRows()) {
    addIconTarget(targets, row);
  }
  for (const row of queryEnchantItemIconRows()) {
    addIconTarget(targets, row);
  }
  for (const row of queryLightstoneIconRows()) {
    addIconTarget(targets, row);
  }
  return [...targets.values()];
}

function resolveMissingSourcePaths(targets, sourceArchive) {
  const explicitTargets = targets.filter((target) => target.sourcePath);
  const verifiedExactPaths = new Set();
  if (explicitTargets.length > 0) {
    const exactMatches = listArchiveMatches(
      sourceArchive,
      explicitTargets.map((target) => target.sourcePath),
    );
    for (const match of exactMatches) {
      verifiedExactPaths.add(match.path.toLowerCase());
    }
  }

  const unresolved = [];
  for (const target of targets) {
    if (!target.sourcePath) {
      unresolved.push(target);
      continue;
    }
    if (!verifiedExactPaths.has(target.sourcePath.toLowerCase())) {
      target.sourcePath = null;
      unresolved.push(target);
    }
  }

  if (unresolved.length === 0) {
    return;
  }

  const wildcardMatches = listArchiveMatches(
    sourceArchive,
    unresolved.map((target) => `*${padIconId(target.iconId)}.dds`),
  );
  const matchesByIconId = new Map();
  for (const match of wildcardMatches) {
    const iconId = parseIconIdFromSourcePath(match.path);
    if (!iconId) {
      continue;
    }
    const group = matchesByIconId.get(iconId) ?? [];
    group.push(match);
    matchesByIconId.set(iconId, group);
  }

  for (const target of unresolved) {
    const bestMatch = chooseBestArchiveMatch(matchesByIconId.get(target.iconId) ?? []);
    if (!bestMatch) {
      target.unresolved = true;
      continue;
    }
    target.sourcePath = bestMatch.path;
  }
}

function extractSelectedSources(sourceArchive, sourcePaths, tempDir) {
  if (sourcePaths.length === 0) {
    return;
  }
  const args = ["run", "-q", "-p", "pazifista", "--", sourceArchive];
  for (const sourcePath of sourcePaths) {
    args.push("-f", sourcePath);
  }
  args.push("-o", tempDir, "-y", "-q");
  runCommand("cargo", args, { capture: false });
}

function convertToWebp(sourcePath, outputPath) {
  runCommand("magick", [
    sourcePath,
    "-auto-orient",
    "-strip",
    "-resize",
    `${iconSize}x${iconSize}`,
    "-define",
    "webp:method=6",
    "-quality",
    String(webpQuality),
    outputPath,
  ]);
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  mkdirSync(options.outputDir, { recursive: true });

  const targets = queryCalculatorIconTargets(options.calculatorApiUrl);
  const pendingTargets = targets.filter((target) =>
    shouldBuild(outputPathForIcon(options.outputDir, target.iconId), options.force),
  );

  if (pendingTargets.length === 0) {
    if (!options.quiet) {
      console.log(`calculator item icons are current under ${path.relative(repoRoot, options.outputDir)}`);
    }
    return;
  }

  if (!existsSync(options.sourceArchive)) {
    fail(
      `source archive not found: ${options.sourceArchive}\n` +
        "Provide --source-archive or populate data/scratch/paz before building source-backed item icons.",
    );
  }

  resolveMissingSourcePaths(pendingTargets, options.sourceArchive);
  const unresolvedTargets = pendingTargets.filter((target) => target.unresolved);
  for (const target of unresolvedTargets) {
    console.warn(
      `warning: could not resolve a source DDS for icon ${padIconId(target.iconId)} (${target.displayName})`,
    );
  }
  const readyTargets = pendingTargets.filter((target) => target.sourcePath && !target.unresolved);

  const tempDir = mkdtempSync(path.join(os.tmpdir(), "fishystuff-item-icons-"));
  try {
    extractSelectedSources(
      options.sourceArchive,
      [...new Set(readyTargets.map((target) => target.sourcePath))],
      tempDir,
    );

    for (const target of readyTargets) {
      const extractedPath = path.join(tempDir, target.sourcePath);
      if (!existsSync(extractedPath)) {
        fail(`expected extracted source icon is missing: ${extractedPath}`);
      }
      const outputPath = outputPathForIcon(options.outputDir, target.iconId);
      convertToWebp(extractedPath, outputPath);
      if (!options.quiet) {
        console.log(
          `built ${path.relative(repoRoot, outputPath)} from ${target.sourcePath} (${target.displayName})`,
        );
      }
    }
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
}

main();
