#!/usr/bin/env node

import fs from "node:fs/promises";
import path from "node:path";

function usage() {
  console.error("usage: print_runtime_map_asset_cache_key.mjs [--allow-empty] <runtime-config.js>");
}

function extractPayload(text) {
  const match = text.match(/Object\.freeze\((\{[\s\S]*\})\);?\s*$/);
  if (!match) {
    throw new Error("runtime config payload not found");
  }
  return JSON.parse(match[1]);
}

async function main() {
  const args = process.argv.slice(2);
  const allowEmptyIndex = args.indexOf("--allow-empty");
  const allowEmpty = allowEmptyIndex !== -1;
  if (allowEmpty) {
    args.splice(allowEmptyIndex, 1);
  }
  const [inputPath] = args;
  if (!inputPath) {
    usage();
    process.exitCode = 2;
    return;
  }

  const resolvedPath = path.resolve(process.cwd(), inputPath);
  const text = await fs.readFile(resolvedPath, "utf8");
  const payload = extractPayload(text);
  const cacheKey = String(payload.mapAssetCacheKey ?? "").trim();
  if (!cacheKey) {
    if (allowEmpty) {
      return;
    }
    throw new Error("mapAssetCacheKey is empty");
  }
  process.stdout.write(`${cacheKey}\n`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
