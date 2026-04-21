import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { LANGUAGE_CONFIG } from "./language-config.mjs";
import { buildShellPageEntries, buildShellPagePathSet, renderShellPageSource } from "./shell-pages.mjs";

const scriptPath = fileURLToPath(import.meta.url);
const siteDir = path.resolve(path.dirname(scriptPath), "..");

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

export function buildShellContentTree({
  config = LANGUAGE_CONFIG,
  rootDir = siteDir,
  outRoot = path.join(rootDir, ".generated", "content"),
} = {}) {
  const shellEntries = buildShellPageEntries({ config, rootDir });
  const shellPathsByLocale = buildShellPagePathSet({ config, rootDir });
  fs.rmSync(outRoot, { recursive: true, force: true });
  for (const contentLanguage of config.contentLanguages) {
    const locale = contentLanguage.code;
    const sourceDir = path.join(rootDir, "content", locale);
    const targetDir = path.join(outRoot, locale);
    copyTrackedContentTree(sourceDir, targetDir, shellPathsByLocale.get(locale) ?? new Set());
  }
  for (const entry of shellEntries) {
    const targetPath = path.join(outRoot, entry.locale, entry.relativePath);
    fs.mkdirSync(path.dirname(targetPath), { recursive: true });
    fs.writeFileSync(targetPath, renderShellPageSource(entry), "utf8");
  }
  return { outRoot, entries: shellEntries };
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
