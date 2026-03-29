import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const siteDir = path.resolve(scriptDir, "..");
const entryPath = path.join(scriptDir, "d3-entry.mjs");
const outDir = path.join(siteDir, ".tmp-d3-build");
const targetPaths = [
  path.join(siteDir, "assets", "js", "d3.js"),
  path.join(siteDir, "public", "js", "d3.js"),
];

const result = await Bun.build({
  entrypoints: [entryPath],
  outdir: outDir,
  naming: "d3.js",
  format: "esm",
  target: "browser",
  minify: false,
  splitting: false,
  sourcemap: "none",
});

if (!result.success || result.outputs.length !== 1) {
  const logs = (result.logs || []).map((log) => log.message).join("\n");
  throw new Error(`Failed to build d3.js bundle.\n${logs}`);
}

const bundledSource = await result.outputs[0].text();

for (const targetPath of targetPaths) {
  await mkdir(path.dirname(targetPath), { recursive: true });
  await writeFile(targetPath, bundledSource, "utf8");
}

console.log("Wrote assets/js/d3.js and public/js/d3.js from d3");
