import fs from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

import { resolvePublicBaseUrls } from "./write-runtime-config.mjs";

function parseArgs(argv) {
  const args = {
    template: "zine.ziggy",
    out: "zine.ziggy",
    help: false,
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      continue;
    }
    if (arg === "--template" && i + 1 < argv.length) {
      args.template = argv[++i];
      continue;
    }
    if (arg === "--out" && i + 1 < argv.length) {
      args.out = argv[++i];
      continue;
    }
    throw new Error(`unknown arg: ${arg}`);
  }

  return args;
}

function printHelp() {
  console.log(`write-zine-config.mjs

Emit a zine.ziggy config with the public site host resolved from the
FISHYSTUFF_PUBLIC_* environment layer.

Options:
  --template <path>  Input template file (default: zine.ziggy)
  --out <path>       Output file (default: zine.ziggy)
  --help             Show this message
`);
}

export function rewriteZineHostUrl(templateSource, siteBaseUrl) {
  const normalizedSiteBaseUrl = String(siteBaseUrl ?? "").trim();
  if (!normalizedSiteBaseUrl) {
    throw new Error("site base URL is required");
  }
  const source = String(templateSource ?? "");
  const next = source.replace(
    /^(\s*\.host_url\s*=\s*)"[^"]*",?$/m,
    `$1"${normalizedSiteBaseUrl}",`,
  );
  if (next === source) {
    throw new Error("failed to find .host_url in zine config template");
  }
  return next;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    printHelp();
    return;
  }

  const { publicSiteBaseUrl } = resolvePublicBaseUrls(process.env);
  const templatePath = path.resolve(process.cwd(), args.template);
  const outPath = path.resolve(process.cwd(), args.out);
  const templateSource = await fs.readFile(templatePath, "utf8");
  const nextConfig = rewriteZineHostUrl(templateSource, publicSiteBaseUrl);
  await fs.mkdir(path.dirname(outPath), { recursive: true });
  await fs.writeFile(outPath, nextConfig, "utf8");
}

const isMainModule =
  process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url;

if (isMainModule) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
