#!/usr/bin/env node

import {
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { spawn, spawnSync } from "node:child_process";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const scriptDir = path.dirname(scriptPath);
const repoRoot = path.resolve(scriptDir, "../..");
const defaultSourceArchive = path.join(repoRoot, "data/scratch/paz");
const defaultRawOutputDir = path.join(repoRoot, "data/cdn/public/images/tiles/minimap");
const defaultVisualOutputDir = path.join(
  repoRoot,
  "data/cdn/public/images/tiles/minimap_visual/v1",
);
const sourceArchiveFilter =
  "ui_texture/new_ui_common_forlua/widget/rader/minimap_data_pack/rader_*.dds";
const visualTilePx = 512;
const visualMaxLevel = 2;
const visualRootUrl = "/images/tiles/minimap_visual/v1";
const defaultConvertConcurrency = Math.max(
  2,
  Math.min(
    8,
    Number.parseInt(process.env.FISHYSTUFF_MINIMAP_CONCURRENCY ?? "", 10) ||
      (typeof os.availableParallelism === "function"
        ? os.availableParallelism()
        : os.cpus().length || 4),
  ),
);
const extractBatchSize = 256;
const wildcardExtractThreshold = 4096;

function fail(message) {
  throw new Error(message);
}

function usage() {
  return [
    "Usage: node tools/scripts/build_minimap_tiles_from_source.mjs [options]",
    "",
    "Options:",
    "  --source-archive <path>     PAZ archive root, .meta file, or archive directory",
    "  --raw-output-dir <path>     Output directory for raw rader_*.png tiles",
    "  --visual-output-dir <path>  Output directory for minimap_visual/v1 tiles",
    "  --force                     Rebuild raw tiles and visual tiles",
    "  --force-visual             Rebuild only the visual minimap pyramid",
    "  --skip-visual              Stop after raw rader_*.png tile generation",
    "  --quiet                    Reduce progress output",
    "  -h, --help                 Show this help",
  ].join("\n");
}

function parseArgs(argv) {
  const options = {
    force: false,
    forceVisual: false,
    skipVisual: false,
    quiet: false,
    sourceArchive: defaultSourceArchive,
    rawOutputDir: defaultRawOutputDir,
    visualOutputDir: defaultVisualOutputDir,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "-h" || arg === "--help") {
      process.stdout.write(`${usage()}\n`);
      process.exit(0);
    }
    if (arg === "--force") {
      options.force = true;
      continue;
    }
    if (arg === "--force-visual") {
      options.forceVisual = true;
      continue;
    }
    if (arg === "--skip-visual") {
      options.skipVisual = true;
      continue;
    }
    if (arg === "--quiet") {
      options.quiet = true;
      continue;
    }
    if (arg === "--source-archive") {
      index += 1;
      options.sourceArchive = argv[index] ? path.resolve(argv[index]) : null;
      continue;
    }
    if (arg === "--raw-output-dir") {
      index += 1;
      options.rawOutputDir = argv[index] ? path.resolve(argv[index]) : null;
      continue;
    }
    if (arg === "--visual-output-dir") {
      index += 1;
      options.visualOutputDir = argv[index] ? path.resolve(argv[index]) : null;
      continue;
    }
    fail(`unknown argument: ${arg}`);
  }

  if (!options.sourceArchive) {
    fail("--source-archive requires a value");
  }
  if (!options.rawOutputDir) {
    fail("--raw-output-dir requires a value");
  }
  if (!options.visualOutputDir) {
    fail("--visual-output-dir requires a value");
  }

  return options;
}

function runCommand(command, args, { capture = true } = {}) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
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
  const args = ["run", "-q", "-p", "pazifista", "--", sourceArchive, "-l", "-q"];
  for (const filter of filters) {
    args.push("-f", filter);
  }
  return parseArchiveMatches(runCommand("cargo", args));
}

function outputNameForMatch(match) {
  const basename = path.basename(match.path).toLowerCase();
  const tileNameMatch = basename.match(/^(rader_-?\d+_-?\d+)\.dds$/);
  if (!tileNameMatch) {
    return null;
  }
  return `${tileNameMatch[1]}.png`;
}

function shouldBuildRaw(outputPath, force) {
  return force || !existsSync(outputPath);
}

function chunkArray(values, size) {
  const chunks = [];
  for (let index = 0; index < values.length; index += size) {
    chunks.push(values.slice(index, index + size));
  }
  return chunks;
}

function extractSourceBatch(sourceArchive, filters, tempDir, quiet) {
  if (filters.length === 0) {
    return;
  }
  const args = ["run", "-q", "-p", "pazifista", "--", sourceArchive];
  for (const filter of filters) {
    args.push("-f", filter);
  }
  args.push("-o", tempDir, "-y", "-q");
  runCommand("cargo", args, { capture: false });
}

function extractSelectedSources(sourceArchive, pendingMatches, tempDir, quiet, totalMatches) {
  if (pendingMatches.length === 0) {
    return;
  }

  const useWildcardExtraction =
    pendingMatches.length === totalMatches ||
    pendingMatches.length >= wildcardExtractThreshold ||
    pendingMatches.length * 5 >= totalMatches * 4;

  if (useWildcardExtraction) {
    extractSourceBatch(sourceArchive, [sourceArchiveFilter], tempDir, quiet);
    return;
  }

  const filters = pendingMatches.map((match) => match.path);
  for (const batch of chunkArray(filters, extractBatchSize)) {
    extractSourceBatch(sourceArchive, batch, tempDir, quiet);
  }
}

async function convertToPng(sourcePath, outputPath) {
  const args = [
    sourcePath,
    "-strip",
    `PNG32:${outputPath}`,
  ];
  await runCommandAsync("magick", args);
}

async function buildPendingRawTiles(pendingMatches, tempDir, outputDir, quiet) {
  if (pendingMatches.length === 0) {
    return 0;
  }

  mkdirSync(outputDir, { recursive: true });
  let nextIndex = 0;
  let completedCount = 0;

  async function worker() {
    while (true) {
      const currentIndex = nextIndex;
      nextIndex += 1;
      const match = pendingMatches[currentIndex];
      if (!match) {
        return;
      }
      const outputName = outputNameForMatch(match);
      if (!outputName) {
        fail(`unexpected minimap archive path: ${match.path}`);
      }
      const extractedPath = path.join(tempDir, match.path);
      if (!existsSync(extractedPath)) {
        fail(`expected extracted source tile is missing: ${extractedPath}`);
      }
      const outputPath = path.join(outputDir, outputName);
      await convertToPng(extractedPath, outputPath);
      completedCount += 1;
      if (
        !quiet &&
        (completedCount === pendingMatches.length || completedCount % 250 === 0)
      ) {
        console.log(
          `converted ${completedCount}/${pendingMatches.length} raw minimap tiles`,
        );
      }
    }
  }

  const workerCount = Math.min(defaultConvertConcurrency, pendingMatches.length);
  await Promise.all(Array.from({ length: workerCount }, () => worker()));
  return pendingMatches.length;
}

function pruneStaleRawTiles(outputDir, expectedOutputNames, quiet) {
  if (!existsSync(outputDir)) {
    return 0;
  }
  let removed = 0;
  for (const entry of readdirSync(outputDir, { withFileTypes: true })) {
    if (!entry.isFile() || !/^rader_-?\d+_-?\d+\.png$/i.test(entry.name)) {
      continue;
    }
    if (expectedOutputNames.has(entry.name.toLowerCase())) {
      continue;
    }
    const entryPath = path.join(outputDir, entry.name);
    unlinkSync(entryPath);
    removed += 1;
    if (!quiet) {
      console.log(`removed stale raw minimap tile ${path.relative(repoRoot, entryPath)}`);
    }
  }
  return removed;
}

function writeRawMetadata(outputDir, sourceArchive, totalMatches) {
  const metadataPath = path.join(outputDir, "source-manifest.json");
  const payload = {
    generated_at_utc: new Date().toISOString(),
    source_archive: path.relative(repoRoot, sourceArchive),
    archive_filter: sourceArchiveFilter,
    tile_count: totalMatches,
  };
  writeFileSync(metadataPath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

function readVisualManifestSummary(manifestPath) {
  if (!existsSync(manifestPath)) {
    return null;
  }
  const payload = JSON.parse(readFileSync(manifestPath, "utf8"));
  const levels = Array.isArray(payload.levels) ? payload.levels : [];
  return {
    tileSizePx: Number(payload.tile_size_px) || 0,
    maxLevel: levels.reduce(
      (maxLevel, level) => Math.max(maxLevel, Number(level?.z) || 0),
      0,
    ),
  };
}

function buildVisualTiles(rawOutputDir, visualOutputDir, quiet) {
  mkdirSync(path.dirname(visualOutputDir), { recursive: true });
  const tempOutputDir = `${visualOutputDir}.tmp.${process.pid}`;
  rmSync(tempOutputDir, { recursive: true, force: true });

  const args = [
    "run",
    "--manifest-path",
    path.join(repoRoot, "Cargo.toml"),
    "--release",
    "-p",
    "fishystuff_tilegen",
    "--bin",
    "minimap_display_tiles",
    "--",
    "--input-dir",
    rawOutputDir,
    "--out-dir",
    tempOutputDir,
    "--tile-px",
    String(visualTilePx),
    "--max-level",
    String(visualMaxLevel),
    "--root-url",
    visualRootUrl,
  ];

  runCommand("cargo", args, { capture: false });
  rmSync(visualOutputDir, { recursive: true, force: true });
  renameSync(tempOutputDir, visualOutputDir);

  if (!quiet) {
    console.log(`rebuilt ${path.relative(repoRoot, visualOutputDir)}`);
  }
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (!existsSync(options.sourceArchive)) {
    fail(
      `source archive not found: ${options.sourceArchive}\n` +
        "Provide --source-archive or populate data/scratch/paz before building source-backed minimap tiles.",
    );
  }

  const matches = listArchiveMatches(options.sourceArchive, [sourceArchiveFilter]);
  if (matches.length === 0) {
    fail(`no archive entries matched ${sourceArchiveFilter}`);
  }

  const expectedOutputNames = new Set();
  const pendingMatches = [];
  for (const match of matches) {
    const outputName = outputNameForMatch(match);
    if (!outputName) {
      continue;
    }
    expectedOutputNames.add(outputName);
    const outputPath = path.join(options.rawOutputDir, outputName);
    if (shouldBuildRaw(outputPath, options.force)) {
      pendingMatches.push(match);
    }
  }

  const tempDir = mkdtempSync(path.join(os.tmpdir(), "fishystuff-minimap-"));
  try {
    if (!options.quiet) {
      console.log(
        `resolved ${matches.length} source-backed minimap tiles from ${path.relative(repoRoot, options.sourceArchive)}`,
      );
      if (pendingMatches.length > 0) {
        console.log(`extracting and converting ${pendingMatches.length} pending raw minimap tiles`);
      }
    }

    extractSelectedSources(
      options.sourceArchive,
      pendingMatches,
      tempDir,
      options.quiet,
      matches.length,
    );
    const convertedCount = await buildPendingRawTiles(
      pendingMatches,
      tempDir,
      options.rawOutputDir,
      options.quiet,
    );
    const prunedCount = pruneStaleRawTiles(
      options.rawOutputDir,
      expectedOutputNames,
      options.quiet,
    );
    writeRawMetadata(options.rawOutputDir, options.sourceArchive, matches.length);

    if (options.skipVisual) {
      if (!options.quiet) {
        console.log(
          `raw minimap tile set is current under ${path.relative(repoRoot, options.rawOutputDir)} ` +
            `(converted ${convertedCount}, pruned ${prunedCount})`,
        );
      }
      return;
    }

    const manifestPath = path.join(options.visualOutputDir, "tileset.json");
    const manifestSummary = readVisualManifestSummary(manifestPath);
    const manifestMtimeMs = existsSync(manifestPath) ? statSync(manifestPath).mtimeMs : 0;
    const shouldRebuildVisual =
      options.force ||
      options.forceVisual ||
      convertedCount > 0 ||
      prunedCount > 0 ||
      !manifestSummary ||
      manifestSummary.tileSizePx !== visualTilePx ||
      manifestSummary.maxLevel !== visualMaxLevel;

    if (shouldRebuildVisual) {
      if (!options.quiet) {
        console.log(`rebuilding minimap visual pyramid under ${path.relative(repoRoot, options.visualOutputDir)}`);
      }
      buildVisualTiles(options.rawOutputDir, options.visualOutputDir, options.quiet);
    } else if (!options.quiet) {
      console.log(
        `minimap visual pyramid is current under ${path.relative(repoRoot, options.visualOutputDir)}`,
      );
    }
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
