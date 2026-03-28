#!/usr/bin/env node

import { existsSync, mkdirSync, statSync } from "node:fs";
import { mkdtempSync, rmSync } from "node:fs";
import { spawn, spawnSync } from "node:child_process";
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
const defaultConvertConcurrency = Math.max(
  2,
  Math.min(
    8,
    Number.parseInt(process.env.FISHYSTUFF_ITEM_ICON_CONCURRENCY ?? "", 10) ||
      (typeof os.availableParallelism === "function"
        ? os.availableParallelism()
        : os.cpus().length || 4),
  ),
);
const sourceIconPathOverrides = new Map([
  [
    768425,
    "ui_texture/icon/new_icon/03_etc/00768388.dds",
  ],
  [
    830349,
    "ui_texture/icon/new_icon/09_cash/00830349_3.dds",
  ],
]);

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

function runCommandAsync(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: repoRoot,
      stdio: ["ignore", "ignore", "pipe"],
    });

    let stderr = "";
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });
    child.on("error", reject);
    child.on("close", (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(
        new Error(
          `${command} ${args.join(" ")} failed with exit code ${code}${stderr.trim() ? `\n${stderr.trim()}` : ""}`,
        ),
      );
    });
  });
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

function outputPathForStem(outputDir, stem) {
  return path.join(outputDir, `${stem}.webp`);
}

function targetStem(target) {
  if (target?.assetStem) {
    return String(target.assetStem);
  }
  if (Number.isFinite(target?.iconId)) {
    return padIconId(target.iconId);
  }
  return "";
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
  if (normalized.endsWith(".png")) {
    if (normalized.startsWith("ui_texture/icon/new_icon/product_icon_png/")) {
      return normalized;
    }
    return null;
  }
  if (!normalized.endsWith(".dds")) {
    return null;
  }
  if (normalized.startsWith("ui_texture/ui_artwork/")) {
    return normalized;
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

function parseAssetStemFromPath(rawPath) {
  if (!rawPath) {
    return null;
  }
  const file = String(rawPath).trim().split(/[?#]/, 1)[0];
  const basename = file.split("/").pop() ?? file;
  const stem = basename.replace(/\.[^.]+$/, "");
  return stem || null;
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
  const overrideSourcePath = normalizeArchivePath(sourceIconPathOverrides.get(iconId));
  if (overrideSourcePath) {
    existing.sourcePath = overrideSourcePath;
  }
  targets.set(iconId, existing);
}

function addNamedIconTarget(targets, target) {
  const assetStem = String(target.assetStem ?? "").trim();
  if (!assetStem) {
    return;
  }
  const key = `stem:${assetStem.toLowerCase()}`;
  const existing = targets.get(key) ?? {
    assetStem,
    displayName: target.displayName || `icon:${assetStem}`,
    sourcePath: null,
    kind: target.kind || "item",
  };
  existing.displayName = target.displayName || existing.displayName;
  existing.kind = target.kind || existing.kind || "item";
  const normalizedSourcePath = normalizeArchivePath(target.sourcePath);
  if (normalizedSourcePath) {
    existing.sourcePath = normalizedSourcePath;
  }
  targets.set(key, existing);
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

function queryFishingDomainItemIconRows() {
  return doltQueryJson(`
    SELECT DISTINCT
      CAST(v.item_key AS SIGNED) AS item_id,
      NULLIF(TRIM(it.ItemName), '') AS display_name,
      NULLIF(TRIM(it.IconImageFile), '') AS item_icon_file
    FROM item_sub_group_item_variants v
    LEFT JOIN item_table it
      ON CAST(it.Index AS SIGNED) = CAST(v.item_key AS SIGNED)
    WHERE v.item_key IS NOT NULL
      AND NULLIF(TRIM(it.IconImageFile), '') IS NOT NULL
    ORDER BY CAST(v.item_key AS SIGNED)
  `);
}

function queryFishTableIconRows() {
  return doltQueryJson(`
    SELECT DISTINCT
      CAST(ft.item_key AS SIGNED) AS item_id,
      NULLIF(TRIM(ft.name), '') AS display_name,
      NULLIF(TRIM(ft.icon), '') AS fish_item_icon_file,
      NULLIF(TRIM(ft.encyclopedia_icon), '') AS encyclopedia_icon_file,
      NULLIF(TRIM(it.IconImageFile), '') AS item_icon_file
    FROM fish_table ft
    LEFT JOIN item_table it
      ON CAST(it.Index AS SIGNED) = CAST(ft.item_key AS SIGNED)
    ORDER BY CAST(ft.item_key AS SIGNED)
  `);
}

function queryCalculatorIconTargets(calculatorApiUrl) {
  const targets = new Map();
  const apiRows = queryCalculatorApiIconRows(calculatorApiUrl);
  for (const row of apiRows) {
    addIconTarget(targets, row);
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
  for (const row of queryFishingDomainItemIconRows()) {
    addIconTarget(targets, row);
  }
  for (const row of queryFishTableIconRows()) {
    if (row.fish_item_icon_file) {
      const assetStem = parseAssetStemFromPath(row.fish_item_icon_file);
      const preferredSourcePath = row.item_icon_file
        ? normalizeArchivePath(row.item_icon_file)
        : null;
      if (assetStem) {
        addNamedIconTarget(targets, {
          assetStem,
          displayName: row.display_name || `fish:${assetStem}`,
          kind: "item",
          sourcePath:
            preferredSourcePath ||
            `ui_texture/icon/new_icon/product_icon_png/${String(row.fish_item_icon_file).trim().toLowerCase()}`,
        });
      }
    }
    if (row.encyclopedia_icon_file) {
      const assetStem = parseAssetStemFromPath(row.encyclopedia_icon_file);
      if (assetStem) {
        addNamedIconTarget(targets, {
          assetStem,
          displayName: row.display_name || `encyclopedia:${assetStem}`,
          kind: "encyclopedia",
          sourcePath: `ui_texture/ui_artwork/encyclopedia/${assetStem.toLowerCase()}.dds`,
        });
      }
    }
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
    unresolved.flatMap((target) => [`*${target.assetStem || padIconId(target.iconId)}.dds`, `*${target.assetStem || padIconId(target.iconId)}.png`]),
  );
  const matchesByStem = new Map();
  for (const match of wildcardMatches) {
    const stem = parseAssetStemFromPath(match.path)?.toLowerCase();
    if (!stem) {
      continue;
    }
    const group = matchesByStem.get(stem) ?? [];
    group.push(match);
    matchesByStem.set(stem, group);
  }

  for (const target of unresolved) {
    const targetStem = String(target.assetStem || padIconId(target.iconId)).toLowerCase();
    const bestMatch = chooseBestArchiveMatch(matchesByStem.get(targetStem) ?? []);
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

async function convertToWebp(sourcePath, outputPath, target) {
  const args = [
    sourcePath,
    "-auto-orient",
    "-strip",
  ];
  if (target?.kind !== "encyclopedia") {
    args.push("-resize", `${iconSize}x${iconSize}`);
  }
  args.push(
    "-define",
    "webp:method=6",
    "-quality",
    String(webpQuality),
    outputPath,
  );
  await runCommandAsync("magick", args);
}

async function buildReadyTargets(readyTargets, options, tempDir) {
  const concurrency = Math.max(1, defaultConvertConcurrency);
  let nextIndex = 0;

  async function worker() {
    while (true) {
      const currentIndex = nextIndex;
      nextIndex += 1;
      const target = readyTargets[currentIndex];
      if (!target) {
        return;
      }
      const extractedPath = path.join(tempDir, target.sourcePath);
      if (!existsSync(extractedPath)) {
        fail(`expected extracted source icon is missing: ${extractedPath}`);
      }
      const outputPath = target.assetStem
        ? outputPathForStem(options.outputDir, target.assetStem)
        : outputPathForIcon(options.outputDir, target.iconId);
      await convertToWebp(extractedPath, outputPath, target);
      if (!options.quiet) {
        console.log(
          `built ${path.relative(repoRoot, outputPath)} from ${target.sourcePath} (${target.displayName})`,
        );
      }
    }
  }

  const workerCount = Math.min(concurrency, readyTargets.length);
  await Promise.all(Array.from({ length: workerCount }, () => worker()));
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  mkdirSync(options.outputDir, { recursive: true });

  const targets = queryCalculatorIconTargets(options.calculatorApiUrl);
  const pendingTargets = targets.filter((target) =>
    shouldBuild(
      target.assetStem
        ? outputPathForStem(options.outputDir, target.assetStem)
        : outputPathForIcon(options.outputDir, target.iconId),
      options.force,
    ),
  );

  if (pendingTargets.length === 0) {
    if (!options.quiet) {
      console.log(`source-backed fishing icons are current under ${path.relative(repoRoot, options.outputDir)}`);
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
      `warning: could not resolve a source asset for ${targetStem(target) || "unknown"} (${target.displayName})`,
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
    await buildReadyTargets(readyTargets, options, tempDir);
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
