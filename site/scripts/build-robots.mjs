import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { readHostUrlFromZineConfig } from "./build-sitemap.mjs";

const scriptPath = fileURLToPath(import.meta.url);
const siteDir = path.resolve(path.dirname(scriptPath), "..");

function parseArgs(argv) {
  const args = {
    out: "",
    hostUrl: "",
    zineConfigPath: "",
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--out" && index + 1 < argv.length) {
      args.out = argv[++index];
      continue;
    }
    if (arg === "--host-url" && index + 1 < argv.length) {
      args.hostUrl = argv[++index];
      continue;
    }
    if (arg === "--zine-config" && index + 1 < argv.length) {
      args.zineConfigPath = argv[++index];
      continue;
    }
    throw new Error(`unknown arg: ${arg}`);
  }

  if (!args.out) {
    throw new Error("missing required --out");
  }

  return args;
}

function ensureTrailingSlash(value) {
  return String(value ?? "").endsWith("/") ? String(value ?? "") : `${String(value ?? "")}/`;
}

export function buildSitemapUrl(hostUrl) {
  return new URL("/sitemap.xml", ensureTrailingSlash(hostUrl)).toString();
}

export function renderRobotsTxt({ hostUrl }) {
  return [
    "User-agent: *",
    "Allow: /",
    `Sitemap: ${buildSitemapUrl(hostUrl)}`,
    "",
  ].join("\n");
}

export function buildRobots({
  hostUrl = "",
  zineConfigPath = "",
  outPath,
} = {}) {
  const resolvedHostUrl = hostUrl
    || readHostUrlFromZineConfig(path.resolve(zineConfigPath || path.join(siteDir, "zine.ziggy")));
  const robotsTxt = renderRobotsTxt({ hostUrl: resolvedHostUrl });
  if (outPath) {
    fs.mkdirSync(path.dirname(outPath), { recursive: true });
    fs.writeFileSync(outPath, robotsTxt, "utf8");
  }
  return robotsTxt;
}

const isMainModule = process.argv[1] && path.resolve(process.argv[1]) === scriptPath;

if (isMainModule) {
  try {
    const args = parseArgs(process.argv.slice(2));
    buildRobots({
      hostUrl: args.hostUrl,
      zineConfigPath: args.zineConfigPath,
      outPath: path.resolve(args.out),
    });
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
