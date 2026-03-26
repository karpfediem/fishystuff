import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const siteDir = path.resolve(scriptDir, "..");
const sourceDir = path.join(siteDir, "node_modules", "datastar-rc");
const sourcePath = path.join(sourceDir, "bundles", "datastar.js");
const targetPaths = [
  path.join(siteDir, "assets", "js", "datastar.js"),
  path.join(siteDir, "public", "js", "datastar.js"),
];

let source = await readFile(sourcePath, "utf8");

source = source.replace(/\n\/\/# sourceMappingURL=.*\n?$/, "\n");
const exportIndex = source.lastIndexOf("export{");
if (exportIndex !== -1) {
  source = source.slice(0, exportIndex).trimEnd() + "\n";
}

for (const targetPath of targetPaths) {
  await mkdir(path.dirname(targetPath), { recursive: true });
  await writeFile(targetPath, source, "utf8");
}

console.log(
  "Wrote assets/js/datastar.js and public/js/datastar.js from datastar-rc v1.0.0-RC.6 (classic script)",
);
