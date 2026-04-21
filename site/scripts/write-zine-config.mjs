import fs from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

import { LANGUAGE_CONFIG } from "./language-config.mjs";
import { resolvePublicBaseUrls } from "./write-runtime-config.mjs";

function parseArgs(argv) {
  const args = {
    template: "zine.ziggy",
    out: "zine.ziggy",
    generatedContentRoot: "",
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
    if (arg === "--generated-content-root" && i + 1 < argv.length) {
      args.generatedContentRoot = argv[++i];
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
  --generated-content-root <path>
                     Rewrite locale content_dir_path values to this root
  --help             Show this message
`);
}

export function rewriteZineHostUrl(templateSource, siteBaseUrl) {
  const normalizedSiteBaseUrl = String(siteBaseUrl ?? "").trim();
  if (!normalizedSiteBaseUrl) {
    throw new Error("site base URL is required");
  }
  const source = String(templateSource ?? "");
  const hostUrlPattern = /^(\s*\.host_url\s*=\s*)"[^"]*",?$/m;
  if (!hostUrlPattern.test(source)) {
    throw new Error("failed to find .host_url in zine config template");
  }
  return source.replace(
    hostUrlPattern,
    `$1"${normalizedSiteBaseUrl}",`,
  );
}

export function rewriteZineContentDirPaths(templateSource, generatedContentRoot, config = LANGUAGE_CONFIG) {
  const normalizedRoot = String(generatedContentRoot ?? "").trim().replace(/\\/g, "/").replace(/\/+$/, "");
  if (!normalizedRoot) {
    return String(templateSource ?? "");
  }
  const matches = Array.from(String(templateSource ?? "").matchAll(/^(\s*\.content_dir_path\s*=\s*)"([^"]*)"(\s*,?)$/gm));
  if (matches.length < config.contentLanguages.length) {
    throw new Error("failed to find locale content_dir_path entries in zine config template");
  }
  let index = 0;
  return String(templateSource ?? "").replace(
    /^(\s*\.content_dir_path\s*=\s*)"([^"]*)"(\s*,?)$/gm,
    (_match, prefix, _currentValue, suffix) => {
      const language = config.contentLanguages[index++];
      if (!language) {
        return _match;
      }
      return `${prefix}"${normalizedRoot}/${language.code}"${suffix}`;
    },
  );
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
  const nextConfig = rewriteZineContentDirPaths(
    rewriteZineHostUrl(templateSource, publicSiteBaseUrl),
    args.generatedContentRoot,
  );
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
