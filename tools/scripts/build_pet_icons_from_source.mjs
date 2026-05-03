#!/usr/bin/env node

import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { mkdtempSync } from "node:fs";
import { spawn, spawnSync } from "node:child_process";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const scriptDir = path.dirname(scriptPath);
const repoRoot = path.resolve(scriptDir, "../..");
const defaultSourceArchive = path.join(repoRoot, "data/scratch/paz");
const defaultOutputDir = path.join(repoRoot, "data/cdn/public/images/pets");
const petImageSize = 96;
const webpQuality = 88;
const buildStateVersion = 1;
const scriptMtimeMs = statSync(scriptPath).mtimeMs;
const defaultConvertConcurrency = Math.max(
  2,
  Math.min(
    8,
    Number.parseInt(process.env.FISHYSTUFF_PET_ICON_CONCURRENCY ?? "", 10)
      || (typeof os.availableParallelism === "function" ? os.availableParallelism() : os.cpus().length || 4),
  ),
);
const currentRenderSignature = JSON.stringify({
  version: buildStateVersion,
  petImageSize,
  webpQuality,
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

function formatElapsedMs(durationMs) {
  const totalSeconds = Math.max(0, Math.floor(durationMs / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes > 0) {
    return `${minutes}m${String(seconds).padStart(2, "0")}s`;
  }
  return `${seconds}s`;
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
  if (!existsSync(filePath)) {
    return null;
  }
  try {
    return JSON.parse(readFileSync(filePath, "utf8"));
  } catch {
    return null;
  }
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

function buildStatePath(outputDir) {
  return path.join(outputDir, ".build-state.json");
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

function writeBuildState(outputDir, doltWorkingHash, targetCount) {
  writeJsonFile(buildStatePath(outputDir), {
    version: buildStateVersion,
    renderSignature: currentRenderSignature,
    doltWorkingHash,
    targetCount,
    generatedAtUtc: new Date().toISOString(),
  });
}

function parseAssetStemFromPath(rawPath) {
  if (!rawPath) {
    return null;
  }
  const file = String(rawPath).trim().replaceAll("\\", "/").split(/[?#]/, 1)[0];
  const basename = file.split("/").pop() ?? file;
  const stem = basename.replace(/\.[^.]+$/, "").trim();
  return stem ? stem.toLowerCase() : null;
}

function normalizeArchivePath(rawPath) {
  if (!rawPath) {
    return null;
  }
  const normalized = String(rawPath).trim().replaceAll("\\", "/").toLowerCase();
  if (!normalized) {
    return null;
  }
  if (normalized.startsWith("ui_texture/")) {
    return normalized;
  }
  if (normalized.endsWith(".dds") || normalized.endsWith(".png")) {
    return `ui_texture/${normalized}`;
  }
  return null;
}

function outputPathForTarget(outputDir, target) {
  return path.join(outputDir, `${String(target.assetStem).toLowerCase()}.webp`);
}

function shouldBuild(outputPath, force) {
  if (force || !existsSync(outputPath)) {
    return true;
  }
  return false;
}

function dedupeTargetsByOutput(targets) {
  const dedupedTargets = [];
  const targetsByOutput = new Map();

  for (const target of targets) {
    const outputKey = `${String(target.assetStem).toLowerCase()}.webp`;
    if (!targetsByOutput.has(outputKey)) {
      const normalizedTarget = { ...target };
      dedupedTargets.push(normalizedTarget);
      targetsByOutput.set(outputKey, normalizedTarget);
    }
  }

  return dedupedTargets;
}

function queryPetIconTargets() {
  const rows = doltQueryJson(`
    SELECT DISTINCT
      NULLIF(TRIM(p.IconImageFile1), '') AS pet_icon_file,
      NULL AS display_name
    FROM pet_table p
    WHERE NULLIF(TRIM(p.IconImageFile1), '') IS NOT NULL
    ORDER BY pet_icon_file
  `);

  return rows
    .map((row) => {
      const assetStem = parseAssetStemFromPath(row.pet_icon_file);
      const sourcePath = normalizeArchivePath(row.pet_icon_file);
      if (!assetStem || !sourcePath) {
        return null;
      }
      return {
        assetStem,
        displayName: String(row.display_name || `pet:${assetStem}`),
        sourcePath,
      };
    })
    .filter(Boolean);
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

function chooseBestArchiveMatch(matches) {
  return [...matches]
    .sort((left, right) => {
      const score = (candidate) => (
        candidate.path.length * -1
        + Math.min(candidate.size, 20000) / 1000
        + (candidate.path.toLowerCase().startsWith("ui_texture/") ? 100 : 0)
      );
      return score(right) - score(left);
    })[0] ?? null;
}

async function resolveSourcePaths(targets, sourceArchive, options = {}) {
  if (targets.length === 0) {
    return;
  }

  if (!options.quiet) {
    console.log(`checking ${targets.length} explicit source pet texture paths in the archive`);
  }
  const exactMatches = await listArchiveMatches(
    sourceArchive,
    targets.map((target) => target.sourcePath),
    {
      heartbeatLabel: `archive verification for ${targets.length} explicit pet texture paths`,
      quiet: options.quiet,
    },
  );
  const verified = new Set(exactMatches.map((match) => String(match.path).toLowerCase()));
  const unresolved = [];
  for (const target of targets) {
    if (!verified.has(String(target.sourcePath).toLowerCase())) {
      unresolved.push(target);
      target.sourcePath = null;
    }
  }

  if (unresolved.length === 0) {
    return;
  }

  if (!options.quiet) {
    console.log(`resolving ${unresolved.length} pet textures by asset stem fallback`);
  }
  const wildcardMatches = await listArchiveMatches(
    sourceArchive,
    unresolved.flatMap((target) => [
      `*${target.assetStem}.dds`,
      `*${target.assetStem}.png`,
    ]),
    {
      heartbeatLabel: `archive wildcard scan for ${unresolved.length} unresolved pet textures`,
      quiet: options.quiet,
    },
  );
  const matchesByStem = new Map();
  for (const match of wildcardMatches) {
    const stem = parseAssetStemFromPath(match.path);
    if (!stem) {
      continue;
    }
    const group = matchesByStem.get(stem) ?? [];
    group.push(match);
    matchesByStem.set(stem, group);
  }

  for (const target of unresolved) {
    const bestMatch = chooseBestArchiveMatch(matchesByStem.get(target.assetStem) ?? []);
    if (!bestMatch) {
      target.unresolved = true;
      continue;
    }
    target.sourcePath = bestMatch.path;
  }
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
    console.log(`pruned ${pruned} stale source-backed pet textures from ${path.relative(repoRoot, outputDir)}`);
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
    heartbeatLabel: `archive extraction for ${sourcePaths.length} pet textures`,
    quiet: options.quiet,
  });
}

async function convertToWebp(sourcePath, outputPath) {
  const args = [
    sourcePath,
    "-auto-orient",
    "-strip",
    "-resize",
    `${petImageSize}x${petImageSize}`,
    "-define",
    "webp:method=6",
    "-quality",
    String(webpQuality),
    outputPath,
  ];
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
        fail(`expected extracted pet texture is missing: ${extractedPath}`);
      }
      const outputPath = outputPathForTarget(options.outputDir, target);
      await convertToWebp(extractedPath, outputPath);
      if (!options.quiet) {
        console.log(`built ${path.relative(repoRoot, outputPath)} from ${target.sourcePath} (${target.displayName})`);
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
    console.log(`resolving source-backed pet textures into ${path.relative(repoRoot, options.outputDir)}`);
  }

  const doltWorkingHash = queryDoltWorkingHash();
  let targets = dedupeTargetsByOutput(queryPetIconTargets());
  const buildState = loadBuildState(options.outputDir);
  const buildStateIsStale = Boolean(
    buildState?.stale
      || !buildState
      || buildState.doltWorkingHash !== doltWorkingHash
      || buildState.targetCount !== targets.length,
  );

  pruneStaleOutputs(options.outputDir, targets, options.quiet);
  const pendingTargets = targets.filter((target) => (
    shouldBuild(outputPathForTarget(options.outputDir, target), options.force) || buildStateIsStale
  ));

  if (!options.quiet) {
    console.log(`resolved ${targets.length} source-backed pet textures (${pendingTargets.length} pending)`);
  }

  if (pendingTargets.length === 0) {
    if (!options.quiet) {
      console.log(`source-backed pet textures are current under ${path.relative(repoRoot, options.outputDir)}`);
    }
    writeBuildState(options.outputDir, doltWorkingHash, targets.length);
    return;
  }

  if (!existsSync(options.sourceArchive)) {
    fail(
      `source archive not found: ${options.sourceArchive}\n`
      + "Provide --source-archive or populate data/scratch/paz before building source-backed pet textures.",
    );
  }

  await resolveSourcePaths(pendingTargets, options.sourceArchive, { quiet: options.quiet });
  const unresolvedTargets = pendingTargets.filter((target) => target.unresolved || !target.sourcePath);
  for (const target of unresolvedTargets) {
    console.warn(`warning: could not resolve a source asset for ${target.assetStem} (${target.displayName})`);
  }
  const readyTargets = pendingTargets.filter((target) => target.sourcePath && !target.unresolved);
  if (!options.quiet) {
    console.log(`preparing ${readyTargets.length} source-backed pet textures (${unresolvedTargets.length} unresolved)`);
  }

  const tempDir = mkdtempSync(path.join(os.tmpdir(), "fishystuff-pet-icons-"));
  try {
    const uniqueSourcePaths = [...new Set(readyTargets.map((target) => target.sourcePath))];
    if (!options.quiet) {
      console.log(`extracting ${uniqueSourcePaths.length} pet texture files`);
    }
    await extractSelectedSources(options.sourceArchive, uniqueSourcePaths, tempDir, { quiet: options.quiet });
    if (!options.quiet) {
      console.log(`building ${readyTargets.length} source-backed pet textures`);
    }
    await buildReadyTargets(readyTargets, options, tempDir);
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }

  writeBuildState(options.outputDir, doltWorkingHash, targets.length);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
