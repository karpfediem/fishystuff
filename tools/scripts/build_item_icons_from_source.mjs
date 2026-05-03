#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, readdirSync, realpathSync, rmSync, statSync, writeFileSync } from "node:fs";
import { mkdtempSync } from "node:fs";
import { spawn, spawnSync } from "node:child_process";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const scriptDir = path.dirname(scriptPath);
const repoRoot = path.resolve(scriptDir, "../..");
const defaultSourceArchive = path.join(repoRoot, "data/scratch/paz");
const defaultOutputDir = path.join(repoRoot, "data/cdn/public/images/items");
const defaultHotspotsAssetPath = path.join(repoRoot, "data/cdn/public/hotspots/hotspots.v1.json");
const iconSize = 44;
const webpQuality = 86;
const scriptMtimeMs = statSync(scriptPath).mtimeMs;
const buildStateVersion = 1;
const targetCacheVersion = 8;
const sourceResolutionCacheVersion = 2;
const consumableIconTargetsSqlPath = path.join(scriptDir, "sql", "calculator_consumable_icon_targets.sql");
const consumableIconTargetsSqlStat = statSync(consumableIconTargetsSqlPath);
const consumableIconTargetsSql = readFileSync(consumableIconTargetsSqlPath, "utf8");
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
    15647,
    "ui_texture/icon/new_icon/09_cash/03_product/00015647.dds",
  ],
  [
    790580,
    "ui_texture/icon/new_icon/04_pc_skill/03_buff/event_item_00790580.dds",
  ],
  [
    830349,
    "ui_texture/icon/new_icon/09_cash/00830349_3.dds",
  ],
]);
const currentRenderSignature = JSON.stringify({
  version: buildStateVersion,
  iconSize,
  webpQuality,
  sourceIconPathOverrides: [...sourceIconPathOverrides.entries()].sort((left, right) => left[0] - right[0]),
});

function fail(message) {
  throw new Error(message);
}

function parseArgs(argv) {
  const options = {
    force: false,
    quiet: false,
    outputDir: defaultOutputDir,
    sourceArchive: defaultSourceArchive,
    hotspotsAsset: defaultHotspotsAssetPath,
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
    if (arg === "--hotspots-asset") {
      index += 1;
      options.hotspotsAsset = argv[index] ? path.resolve(argv[index]) : null;
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
  return options;
}

function runCommand(command, args, { capture = true } = {}) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
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

function formatElapsedMs(durationMs) {
  const totalSeconds = Math.max(0, Math.floor(durationMs / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes > 0) {
    return `${minutes}m${String(seconds).padStart(2, "0")}s`;
  }
  return `${seconds}s`;
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

function runCommandWithHeartbeat(
  command,
  args,
  {
    capture = true,
    heartbeatLabel = "",
    heartbeatIntervalMs = 15000,
    quiet = false,
  } = {},
) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: repoRoot,
      stdio: ["ignore", capture ? "pipe" : "inherit", "pipe"],
    });

    const startedAt = Date.now();
    let stdout = "";
    let stderr = "";
    const heartbeatTimer =
      !quiet && heartbeatLabel
        ? setInterval(() => {
            console.log(`${heartbeatLabel} still running after ${formatElapsedMs(Date.now() - startedAt)}`);
          }, heartbeatIntervalMs)
        : null;

    if (capture && child.stdout) {
      child.stdout.on("data", (chunk) => {
        stdout += chunk.toString();
      });
    }
    if (child.stderr) {
      child.stderr.on("data", (chunk) => {
        stderr += chunk.toString();
      });
    }

    const finish = (callback) => {
      if (heartbeatTimer) {
        clearInterval(heartbeatTimer);
      }
      callback();
    };

    child.on("error", (error) => finish(() => reject(error)));
    child.on("close", (code) => {
      finish(() => {
        if (code === 0) {
          resolve(capture ? stdout : "");
          return;
        }
        reject(
          new Error(
            `${command} ${args.join(" ")} failed with exit code ${code}${stderr.trim() ? `\n${stderr.trim()}` : ""}`,
          ),
        );
      });
    });
  });
}

function readJsonFile(filePath) {
  if (!filePath || !existsSync(filePath)) {
    return null;
  }
  try {
    return JSON.parse(readFileSync(filePath, "utf8"));
  } catch {
    return null;
  }
}

function fileSignature(filePath) {
  const resolvedPath = filePath ? path.resolve(filePath) : "";
  if (!resolvedPath || !existsSync(resolvedPath)) {
    return {
      path: resolvedPath,
      exists: false,
      mtimeMs: null,
      size: null,
    };
  }
  const stats = statSync(resolvedPath);
  return {
    path: resolvedPath,
    exists: true,
    mtimeMs: stats.mtimeMs,
    size: stats.size,
  };
}

function writeJsonFile(filePath, payload) {
  writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

function doltQueryJson(sql) {
  const output = runCommand("dolt", ["sql", "-r", "json", "-q", sql]);
  const parsed = JSON.parse(output);
  return parsed.rows ?? [];
}

function queryDoltWorkingHash() {
  const rows = doltQueryJson("SELECT DOLT_HASHOF_DB() AS db_hash");
  const row = rows[0] ?? {};
  return String(row.db_hash ?? row["dolt_hashof_db()"] ?? "");
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

function outputPathForTarget(outputDir, target) {
  return target.assetStem
    ? outputPathForStem(outputDir, target.assetStem)
    : outputPathForIcon(outputDir, target.iconId);
}

function outputBasenameForTarget(target) {
  return target.assetStem
    ? `${String(target.assetStem)}.webp`
    : `${padIconId(target.iconId)}.webp`;
}

function buildStatePath(outputDir) {
  return path.join(outputDir, ".build-state.json");
}

function targetCachePath(outputDir) {
  return path.join(outputDir, ".target-cache.json");
}

function sourceResolutionCachePath(outputDir) {
  return path.join(outputDir, ".source-resolution-cache.json");
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

function shouldReplaceDisplayName(currentName, nextName) {
  if (!nextName) {
    return false;
  }
  if (!currentName) {
    return true;
  }
  return currentName.startsWith("icon:") && !nextName.startsWith("icon:");
}

function dedupeTargetsByOutput(targets) {
  const dedupedTargets = [];
  const targetsByOutput = new Map();
  let duplicateCount = 0;

  for (const target of targets) {
    const outputKey = outputBasenameForTarget(target).toLowerCase();
    const existing = targetsByOutput.get(outputKey);
    if (!existing) {
      const normalizedTarget = { ...target };
      dedupedTargets.push(normalizedTarget);
      targetsByOutput.set(outputKey, normalizedTarget);
      continue;
    }

    duplicateCount += 1;
    if (!existing.sourcePath && target.sourcePath) {
      existing.sourcePath = target.sourcePath;
    }
    if (shouldReplaceDisplayName(existing.displayName, target.displayName)) {
      existing.displayName = target.displayName;
    }
    if (!existing.kind && target.kind) {
      existing.kind = target.kind;
    }
  }

  return {
    duplicateCount,
    targets: dedupedTargets,
  };
}

function shouldBuild(outputPath, force) {
  if (force || !existsSync(outputPath)) {
    return true;
  }
  return false;
}

function loadBuildState(outputDir) {
  const cached = readJsonFile(buildStatePath(outputDir));
  if (!cached || cached.version !== buildStateVersion) {
    return null;
  }
  if (cached.renderSignature !== currentRenderSignature) {
    return { stale: true };
  }
  return cached;
}

function writeBuildState(outputDir, targetCount) {
  writeJsonFile(buildStatePath(outputDir), {
    version: buildStateVersion,
    renderSignature: currentRenderSignature,
    targetCount,
    generatedAtUtc: new Date().toISOString(),
  });
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
  const match = stem.match(/(\d{5,8})(?:_[0-9]+)?$/i);
  if (!match) {
    return null;
  }
  const parsed = Number(match[1]);
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

function targetCacheKey(target) {
  return target.assetStem ? `stem:${String(target.assetStem).toLowerCase()}` : `id:${target.iconId}`;
}

function archiveSignature(sourceArchive) {
  const resolvedPath = realpathSync(sourceArchive);
  let signaturePath = resolvedPath;
  let signatureStats = statSync(resolvedPath);
  if (signatureStats.isDirectory()) {
    const metaCandidate = path.join(resolvedPath, "pad00000.meta");
    if (existsSync(metaCandidate)) {
      signaturePath = metaCandidate;
      signatureStats = statSync(signaturePath);
    }
  }
  return {
    resolvedPath,
    signaturePath,
    mtimeMs: signatureStats.mtimeMs,
    size: signatureStats.size,
  };
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

async function listArchiveMatches(sourceArchive, filters, options = {}) {
  if (filters.length === 0) {
    return [];
  }
  const args = ["run", "-q", "-p", "pazifista", "--", sourceArchive, "-l"];
  for (const filter of filters) {
    args.push("-f", filter);
  }
  const listing = await runCommandWithHeartbeat("cargo", args, {
    capture: true,
    heartbeatLabel: options.heartbeatLabel,
    quiet: options.quiet,
  });
  return parseArchiveMatches(listing);
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
  const rawIconPath = row.icon ?? null;
  const rawSourcePath = row.item_icon_file ?? row.skill_icon_file ?? null;
  const assetStem = parseAssetStemFromPath(rawIconPath) || parseAssetStemFromPath(rawSourcePath);
  const itemId = Number(row.item_id);
  const normalizedSourcePath = normalizeArchivePath(rawSourcePath);
  const canonicalIconId =
    Number(row.icon_id) ||
    parseIconIdFromAssetName(rawIconPath) ||
    parseIconIdFromAssetName(rawSourcePath) ||
    null;

  if (assetStem) {
    const key = `stem:${assetStem.toLowerCase()}`;
    const existing = targets.get(key) ?? {
      assetStem,
      displayName: row.display_name || row.source_name_en || row.set_name_ko || `icon:${assetStem}`,
      sourcePath: null,
      kind: row.kind || "item",
    };
    if (normalizedSourcePath) {
      existing.sourcePath = normalizedSourcePath;
    }
    const overrideSourcePath = normalizeArchivePath(sourceIconPathOverrides.get(canonicalIconId));
    if (overrideSourcePath) {
      existing.sourcePath = overrideSourcePath;
    }
    targets.set(key, existing);
    return;
  }

  const iconId =
    canonicalIconId ||
    (Number.isFinite(itemId) && itemId > 0 ? itemId : null) ||
    null;
  if (!Number.isFinite(iconId) || iconId <= 0) {
    return;
  }

  const existing = targets.get(iconId) ?? {
    iconId,
    displayName: row.display_name || row.source_name_en || row.set_name_ko || `icon:${iconId}`,
    sourcePath: null,
  };
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
    WHERE i.id IS NOT NULL
    ORDER BY CAST(i.id AS SIGNED)
  `);
}

function queryConsumableIconRows() {
  return doltQueryJson(consumableIconTargetsSql);
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
      NULLIF(TRIM(stype.SkillName), '') AS display_name,
      NULLIF(TRIM(ls.Description), '') AS set_name_ko,
      NULLIF(TRIM(stype.IconImageFile), '') AS skill_icon_file
    FROM lightstone_set_option ls
    LEFT JOIN skilltype_table_new stype
      ON stype.SkillNo = ls.SetOptionSkillNo
    WHERE NULLIF(TRIM(stype.IconImageFile), '') IS NOT NULL
  `);
}

function loadHotspotIconRows(assetPath) {
  const payload = readJsonFile(assetPath);
  if (!payload || !Array.isArray(payload.hotspots)) {
    return [];
  }

  const rows = [];
  const addRow = ({ itemId, iconId, displayName, sourcePath }) => {
    const numericItemId = Number(itemId);
    const numericIconId = Number(iconId);
    if (
      (!Number.isFinite(numericItemId) || numericItemId <= 0) &&
      (!Number.isFinite(numericIconId) || numericIconId <= 0) &&
      !sourcePath
    ) {
      return;
    }
    rows.push({
      item_id: Number.isFinite(numericItemId) && numericItemId > 0 ? numericItemId : null,
      icon_id: Number.isFinite(numericIconId) && numericIconId > 0 ? numericIconId : null,
      display_name: displayName || null,
      item_icon_file: sourcePath || null,
      kind: "item",
    });
  };
  const visitLootItems = (items) => {
    for (const lootItem of items ?? []) {
      addRow({
        itemId: lootItem.itemId,
        iconId: lootItem.iconItemId ?? lootItem.itemId,
        displayName: lootItem.label ?? lootItem.name,
        sourcePath: lootItem.iconImage,
      });
    }
  };
  const visitLootGroup = (lootGroup) => {
    visitLootItems(lootGroup?.lootItems);
    visitLootItems(lootGroup?.speciesRows);
    for (const option of lootGroup?.conditionOptions ?? []) {
      visitLootItems(option?.lootItems);
      visitLootItems(option?.speciesRows);
    }
  };

  for (const hotspot of payload.hotspots) {
    addRow({
      itemId: hotspot.primaryFishItemId,
      iconId: hotspot.primaryFishItemId,
      displayName: hotspot.primaryFishName,
      sourcePath: hotspot.primaryFishIconImage,
    });
    visitLootItems(hotspot.lootItems);
    for (const lootGroup of hotspot.lootGroups ?? []) {
      visitLootGroup(lootGroup);
    }
  }
  return rows;
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

function queryFishCatalogIconRows() {
  return doltQueryJson(`
    SELECT DISTINCT
      CAST(source.item_id AS SIGNED) AS item_id,
      NULLIF(TRIM(source.display_name), '') AS display_name,
      NULLIF(TRIM(source.item_icon_file), '') AS item_icon_file
    FROM (
      SELECT
        CAST(f.fish_id AS SIGNED) AS item_id,
        it.ItemName AS display_name,
        it.IconImageFile AS item_icon_file
      FROM fish_names_ko f
      JOIN item_table it
        ON CAST(it.Index AS SIGNED) = CAST(f.fish_id AS SIGNED)
      UNION ALL
      SELECT
        CAST(ft.item_key AS SIGNED) AS item_id,
        it.ItemName AS display_name,
        it.IconImageFile AS item_icon_file
      FROM fish_table ft
      LEFT JOIN item_table it
        ON CAST(it.Index AS SIGNED) = CAST(ft.item_key AS SIGNED)
      LEFT JOIN fish_names_ko f
        ON CAST(f.fish_id AS SIGNED) = CAST(ft.item_key AS SIGNED)
      WHERE f.fish_id IS NULL
    ) source
    WHERE source.item_id IS NOT NULL
      AND NULLIF(TRIM(source.item_icon_file), '') IS NOT NULL
    ORDER BY CAST(source.item_id AS SIGNED)
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

function queryCalculatorIconTargets(hotspotsAsset) {
  const targets = new Map();
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
  for (const row of queryFishCatalogIconRows()) {
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
  for (const row of loadHotspotIconRows(hotspotsAsset)) {
    addIconTarget(targets, row);
  }
  return [...targets.values()];
}

function loadCachedTargets(outputDir, doltWorkingHash, hotspotsAssetSignature) {
  const cached = readJsonFile(targetCachePath(outputDir));
  if (!cached || cached.version !== targetCacheVersion) {
    return null;
  }
  if (
    cached.scriptMtimeMs !== scriptMtimeMs ||
    cached.doltWorkingHash !== doltWorkingHash ||
    cached.consumableIconTargetsSqlPath !== consumableIconTargetsSqlPath ||
    cached.consumableIconTargetsSqlMtimeMs !== consumableIconTargetsSqlStat.mtimeMs ||
    cached.consumableIconTargetsSqlSize !== consumableIconTargetsSqlStat.size ||
    cached.hotspotsAssetPath !== hotspotsAssetSignature.path ||
    cached.hotspotsAssetExists !== hotspotsAssetSignature.exists ||
    cached.hotspotsAssetMtimeMs !== hotspotsAssetSignature.mtimeMs ||
    cached.hotspotsAssetSize !== hotspotsAssetSignature.size
  ) {
    return null;
  }
  return Array.isArray(cached.targets) ? cached.targets : null;
}

function writeCachedTargets(outputDir, doltWorkingHash, hotspotsAssetSignature, targets) {
  writeJsonFile(targetCachePath(outputDir), {
    version: targetCacheVersion,
    scriptMtimeMs,
    doltWorkingHash,
    consumableIconTargetsSqlPath,
    consumableIconTargetsSqlMtimeMs: consumableIconTargetsSqlStat.mtimeMs,
    consumableIconTargetsSqlSize: consumableIconTargetsSqlStat.size,
    hotspotsAssetPath: hotspotsAssetSignature.path,
    hotspotsAssetExists: hotspotsAssetSignature.exists,
    hotspotsAssetMtimeMs: hotspotsAssetSignature.mtimeMs,
    hotspotsAssetSize: hotspotsAssetSignature.size,
    generatedAtUtc: new Date().toISOString(),
    targets,
  });
}

function loadSourceResolutionCache(outputDir, signature) {
  const cached = readJsonFile(sourceResolutionCachePath(outputDir));
  if (!cached || cached.version !== sourceResolutionCacheVersion) {
    return {
      entries: new Map(),
      dirty: false,
      signature,
    };
  }
  if (
    cached.scriptMtimeMs !== scriptMtimeMs ||
    cached.signaturePath !== signature.signaturePath ||
    cached.signatureMtimeMs !== signature.mtimeMs ||
    cached.signatureSize !== signature.size
  ) {
    return {
      entries: new Map(),
      dirty: false,
      signature,
    };
  }
  return {
    entries: new Map(Object.entries(cached.entries ?? {})),
    dirty: false,
    signature,
  };
}

function writeSourceResolutionCache(outputDir, cache) {
  writeJsonFile(sourceResolutionCachePath(outputDir), {
    version: sourceResolutionCacheVersion,
    scriptMtimeMs,
    signaturePath: cache.signature.signaturePath,
    signatureMtimeMs: cache.signature.mtimeMs,
    signatureSize: cache.signature.size,
    generatedAtUtc: new Date().toISOString(),
    entries: Object.fromEntries(cache.entries),
  });
}

async function resolveMissingSourcePaths(targets, sourceArchive, resolutionCache, options = {}) {
  const requestedSourcePaths = new Map();
  const targetsToResolve = [];
  for (const target of targets) {
    const requestedSourcePath = target.sourcePath ? target.sourcePath.toLowerCase() : null;
    requestedSourcePaths.set(targetCacheKey(target), requestedSourcePath);
    const cached = resolutionCache.entries.get(targetCacheKey(target));
    if (cached && cached.requestedSourcePath === requestedSourcePath) {
      if (cached.resolvedSourcePath) {
        target.sourcePath = cached.resolvedSourcePath;
      } else {
        target.sourcePath = null;
      }
      if (cached.unresolved) {
        target.unresolved = true;
      }
      continue;
    }
    targetsToResolve.push(target);
  }

  const explicitTargets = targetsToResolve.filter((target) => target.sourcePath);
  const verifiedExactPaths = new Set();
  if (explicitTargets.length > 0) {
    if (!options.quiet) {
      console.log(`checking ${explicitTargets.length} explicit source icon paths in the archive`);
    }
    const exactMatches = await listArchiveMatches(
      sourceArchive,
      explicitTargets.map((target) => target.sourcePath),
      {
        heartbeatLabel: `archive verification for ${explicitTargets.length} explicit source icon paths`,
        quiet: options.quiet,
      },
    );
    for (const match of exactMatches) {
      verifiedExactPaths.add(match.path.toLowerCase());
    }
  }

  const unresolved = [];
  for (const target of targetsToResolve) {
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
    for (const target of targetsToResolve) {
      resolutionCache.entries.set(targetCacheKey(target), {
        requestedSourcePath: requestedSourcePaths.get(targetCacheKey(target)) ?? null,
        resolvedSourcePath: target.sourcePath ?? null,
        unresolved: false,
      });
    }
    resolutionCache.dirty = targetsToResolve.length > 0;
    return;
  }

  if (!options.quiet) {
    console.log(`resolving ${unresolved.length} archive source icon paths by source-backed stem`);
  }
  const wildcardMatches = await listArchiveMatches(
    sourceArchive,
    unresolved.flatMap((target) => [
      `*${target.assetStem || padIconId(target.iconId)}.dds`,
      `*${target.assetStem || padIconId(target.iconId)}.png`,
    ]),
    {
      heartbeatLabel: `archive wildcard scan for ${unresolved.length} unresolved source icon targets`,
      quiet: options.quiet,
    },
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

  for (const target of targetsToResolve) {
    resolutionCache.entries.set(targetCacheKey(target), {
      requestedSourcePath: requestedSourcePaths.get(targetCacheKey(target)) ?? null,
      resolvedSourcePath: target.unresolved ? null : target.sourcePath ?? null,
      unresolved: Boolean(target.unresolved),
    });
  }
  resolutionCache.dirty = targetsToResolve.length > 0;
}

function pruneStaleOutputs(outputDir, targets, quiet) {
  if (!existsSync(outputDir)) {
    return 0;
  }

  const expectedFiles = new Set(
    targets.map((target) => path.basename(outputPathForTarget(outputDir, target))),
  );
  let pruned = 0;
  for (const entry of readdirSync(outputDir, { withFileTypes: true })) {
    if (!entry.isFile() || !entry.name.toLowerCase().endsWith(".webp")) {
      continue;
    }
    if (expectedFiles.has(entry.name)) {
      continue;
    }
    rmSync(path.join(outputDir, entry.name), { force: true });
    pruned += 1;
  }

  if (pruned > 0 && !quiet) {
    console.log(`pruned ${pruned} stale source-backed item icons from ${path.relative(repoRoot, outputDir)}`);
  }
  return pruned;
}

async function extractSelectedSources(sourceArchive, sourcePaths, tempDir, options = {}) {
  if (sourcePaths.length === 0) {
    return;
  }
  const args = ["run", "-q", "-p", "pazifista", "--", sourceArchive];
  for (const sourcePath of sourcePaths) {
    args.push("-f", sourcePath);
  }
  args.push("-o", tempDir, "-y", "-q");
  await runCommandWithHeartbeat("cargo", args, {
    capture: false,
    heartbeatLabel: `archive extraction for ${sourcePaths.length} source icon files`,
    quiet: options.quiet,
  });
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
      const outputPath = outputPathForTarget(options.outputDir, target);
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

  if (!options.quiet) {
    console.log(`resolving source-backed item icon targets into ${path.relative(repoRoot, options.outputDir)}`);
  }

  const doltWorkingHash = queryDoltWorkingHash();
  const hotspotsAssetSignature = fileSignature(options.hotspotsAsset);
  let targets = loadCachedTargets(options.outputDir, doltWorkingHash, hotspotsAssetSignature);
  if (!targets) {
    targets = queryCalculatorIconTargets(options.hotspotsAsset);
  } else if (!options.quiet) {
    console.log(`using cached source-backed item icon targets (${targets.length} targets)`);
  }

  const dedupedTargets = dedupeTargetsByOutput(targets);
  targets = dedupedTargets.targets;
  if (dedupedTargets.duplicateCount > 0 && !options.quiet) {
    console.log(
      `collapsed ${dedupedTargets.duplicateCount} duplicate source-backed icon targets into ${targets.length} output files`,
    );
  }
  writeCachedTargets(options.outputDir, doltWorkingHash, hotspotsAssetSignature, targets);

  const buildState = loadBuildState(options.outputDir);
  pruneStaleOutputs(options.outputDir, targets, options.quiet);
  const pendingTargets = targets.filter((target) =>
    shouldBuild(outputPathForTarget(options.outputDir, target), options.force) ||
    Boolean(buildState?.stale),
  );

  if (!options.quiet) {
    console.log(
      `resolved ${targets.length} source-backed item icon targets (${pendingTargets.length} pending)`,
    );
  }

  if (pendingTargets.length === 0) {
    if (!options.quiet) {
      console.log(`source-backed fishing icons are current under ${path.relative(repoRoot, options.outputDir)}`);
    }
    writeBuildState(options.outputDir, targets.length);
    return;
  }

  if (!existsSync(options.sourceArchive)) {
    fail(
      `source archive not found: ${options.sourceArchive}\n` +
        "Provide --source-archive or populate data/scratch/paz before building source-backed item icons.",
    );
  }

  if (!options.quiet) {
    console.log(`verifying source icon paths from ${options.sourceArchive}`);
  }
  const resolutionCache = loadSourceResolutionCache(
    options.outputDir,
    archiveSignature(options.sourceArchive),
  );
  await resolveMissingSourcePaths(pendingTargets, options.sourceArchive, resolutionCache, {
    quiet: options.quiet,
  });
  const unresolvedTargets = pendingTargets.filter((target) => target.unresolved);
  for (const target of unresolvedTargets) {
    console.warn(
      `warning: could not resolve a source asset for ${targetStem(target) || "unknown"} (${target.displayName})`,
    );
  }
  const readyTargets = pendingTargets.filter((target) => target.sourcePath && !target.unresolved);
  if (!options.quiet) {
    console.log(
      `preparing ${readyTargets.length} source-backed item icons (${unresolvedTargets.length} unresolved)`,
    );
  }

  if (resolutionCache.dirty) {
    writeSourceResolutionCache(options.outputDir, resolutionCache);
  }

  const tempDir = mkdtempSync(path.join(os.tmpdir(), "fishystuff-item-icons-"));
  try {
    if (!options.quiet) {
      console.log(`extracting ${new Set(readyTargets.map((target) => target.sourcePath)).size} source icon files`);
    }
    await extractSelectedSources(
      options.sourceArchive,
      [...new Set(readyTargets.map((target) => target.sourcePath))],
      tempDir,
      { quiet: options.quiet },
    );
    if (!options.quiet) {
      console.log(`building ${readyTargets.length} source-backed item icons`);
    }
    await buildReadyTargets(readyTargets, options, tempDir);
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }

  writeBuildState(options.outputDir, targets.length);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
