import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const siteDir = path.resolve(scriptDir, "..");
const entryPath = path.join(scriptDir, "otel-entry.mjs");
const outDir = path.join(siteDir, ".tmp-otel-build");
const targetPaths = [
  path.join(siteDir, "assets", "js", "otel.js"),
  path.join(siteDir, "public", "js", "otel.js"),
];

const result = await Bun.build({
  entrypoints: [entryPath],
  outdir: outDir,
  naming: "otel.js",
  format: "esm",
  target: "browser",
  minify: false,
  splitting: false,
  sourcemap: "none",
});

if (!result.success || result.outputs.length !== 1) {
  const logs = (result.logs || []).map((log) => log.message).join("\n");
  throw new Error(`Failed to build otel.js bundle.\n${logs}`);
}

const bundledSource = await result.outputs[0].text();

for (const targetPath of targetPaths) {
  await mkdir(path.dirname(targetPath), { recursive: true });
  await writeFile(targetPath, bundledSource, "utf8");
}

console.log("Wrote assets/js/otel.js and public/js/otel.js from OpenTelemetry browser bootstrap");
