import { existsSync, mkdirSync, statSync } from "node:fs";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const siteDir = path.resolve(scriptDir, "..");
const scriptMtimeMs = statSync(fileURLToPath(import.meta.url)).mtimeMs;

const tasks = [
  {
    source: "content/en-US/guides/groups/groups.png",
    outputs: [
      { path: "assets/img/guides/groups-card-320.webp", width: 320, quality: 76 },
      { path: "assets/img/guides/groups-card-640.webp", width: 640, quality: 76 },
    ],
  },
  {
    source: "content/en-US/guides/zones/zones.png",
    outputs: [
      { path: "assets/img/guides/zones-card-320.webp", width: 320, quality: 76 },
      { path: "assets/img/guides/zones-card-640.webp", width: 640, quality: 76 },
    ],
  },
  {
    source: "content/en-US/guides/experience/exp.png",
    outputs: [
      { path: "assets/img/guides/experience-card-320.webp", width: 320, quality: 76 },
      { path: "assets/img/guides/experience-card-640.webp", width: 640, quality: 76 },
    ],
  },
  {
    source: "content/en-US/guides/money/money.png",
    outputs: [
      { path: "assets/img/guides/money-card-320.webp", width: 320, quality: 76 },
      { path: "assets/img/guides/money-card-640.webp", width: 640, quality: 76 },
    ],
  },
  {
    source: "content/en-US/guides/mystical/mystical.png",
    outputs: [
      { path: "assets/img/guides/mystical-card-320.webp", width: 320, quality: 76 },
      { path: "assets/img/guides/mystical-card-640.webp", width: 640, quality: 76 },
    ],
  },
  {
    source: "content/en-US/guides/drr/dura.png",
    outputs: [
      { path: "assets/img/guides/drr-card-320.webp", width: 320, quality: 76 },
      { path: "assets/img/guides/drr-card-640.webp", width: 640, quality: 76 },
    ],
  },
  {
    source: "content/en-US/guides/where-am-i-fishing/where_am_i.png",
    outputs: [
      { path: "assets/img/guides/where-am-i-fishing-card-320.webp", width: 320, quality: 76 },
      { path: "assets/img/guides/where-am-i-fishing-card-640.webp", width: 640, quality: 76 },
    ],
  },
  {
    source: "assets/img/logo.png",
    outputs: [
      { path: "assets/img/favicon-16x16.png", width: 16, quality: 95 },
      { path: "assets/img/favicon-32x32.png", width: 32, quality: 95 },
      { path: "assets/img/logo-32.png", width: 32, quality: 95 },
      { path: "assets/img/logo-64.png", width: 64, quality: 95 },
    ],
  },
];

function shouldBuild(sourcePath, outputPath) {
  if (!existsSync(outputPath)) {
    return true;
  }
  const newestInputMtimeMs = Math.max(statSync(sourcePath).mtimeMs, scriptMtimeMs);
  return statSync(outputPath).mtimeMs < newestInputMtimeMs;
}

function runMagick(sourcePath, outputPath, width, quality) {
  const args = outputPath.endsWith(".webp")
    ? [
        sourcePath,
        "-auto-orient",
        "-strip",
        "-resize",
        `${width}x`,
        "-define",
        "webp:method=6",
        "-quality",
        String(quality),
        outputPath,
      ]
    : [
        sourcePath,
        "-auto-orient",
        "-strip",
        "-resize",
        `${width}x${width}`,
        outputPath,
      ];

  const result = spawnSync("magick", args, {
    cwd: siteDir,
    stdio: "inherit",
  });
  if (result.status !== 0) {
    throw new Error(`magick failed for ${path.relative(siteDir, sourcePath)} -> ${path.relative(siteDir, outputPath)}`);
  }
}

function main() {
  for (const task of tasks) {
    const sourcePath = path.resolve(siteDir, task.source);
    for (const output of task.outputs) {
      const outputPath = path.resolve(siteDir, output.path);
      mkdirSync(path.dirname(outputPath), { recursive: true });
      if (!shouldBuild(sourcePath, outputPath)) {
        continue;
      }
      runMagick(sourcePath, outputPath, output.width, output.quality);
    }
  }
}

main();
