#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, statSync } from "node:fs";
import { spawnSync } from "node:child_process";
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
const visualTilePx = 512;
const visualMaxLevel = 2;
const visualRootUrl = "/images/tiles/minimap_visual/v1";

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

function buildRawMinimapTiles(options) {
  const args = [
    "run",
    "--manifest-path",
    path.join(repoRoot, "Cargo.toml"),
    "--release",
    "-p",
    "fishystuff_tilegen",
    "--bin",
    "minimap_source_tiles",
    "--",
    "--source-archive",
    options.sourceArchive,
    "--out-dir",
    options.rawOutputDir,
  ];
  if (options.force) {
    args.push("--force");
  }
  if (options.quiet) {
    args.push("--quiet");
  }
  runCommand("cargo", args, { capture: false });
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

function buildVisualTiles(options) {
  mkdirSync(path.dirname(options.visualOutputDir), { recursive: true });
  const tempOutputDir = `${options.visualOutputDir}.tmp.${process.pid}`;
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
    options.rawOutputDir,
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
  rmSync(options.visualOutputDir, { recursive: true, force: true });
  renameSync(tempOutputDir, options.visualOutputDir);
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  buildRawMinimapTiles(options);

  if (options.skipVisual) {
    return;
  }

  const rawManifestPath = path.join(options.rawOutputDir, "source-manifest.json");
  if (!existsSync(rawManifestPath)) {
    fail(`expected raw minimap manifest is missing: ${rawManifestPath}`);
  }
  const visualManifestPath = path.join(options.visualOutputDir, "tileset.json");
  const visualSummary = readVisualManifestSummary(visualManifestPath);
  const rawManifestMtimeMs = statSync(rawManifestPath).mtimeMs;
  const visualManifestMtimeMs = existsSync(visualManifestPath)
    ? statSync(visualManifestPath).mtimeMs
    : 0;

  const shouldRebuildVisual =
    options.force ||
    options.forceVisual ||
    !visualSummary ||
    visualSummary.tileSizePx !== visualTilePx ||
    visualSummary.maxLevel !== visualMaxLevel ||
    rawManifestMtimeMs > visualManifestMtimeMs;

  if (shouldRebuildVisual) {
    if (!options.quiet) {
      console.log(
        `rebuilding minimap visual pyramid under ${path.relative(repoRoot, options.visualOutputDir)}`,
      );
    }
    buildVisualTiles(options);
  } else if (!options.quiet) {
    console.log(
      `minimap visual pyramid is current under ${path.relative(repoRoot, options.visualOutputDir)}`,
    );
  }
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
