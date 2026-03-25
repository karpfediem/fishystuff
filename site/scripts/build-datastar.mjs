import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const siteDir = path.resolve(scriptDir, "..");
const packageDir = path.join(siteDir, "node_modules", "@starfederation", "datastar");
const packageJsonPath = path.join(packageDir, "package.json");
const sourcePath = path.join(packageDir, "dist", "datastar.js");
const targetPath = path.join(siteDir, "assets", "js", "datastar.js");

const packageJson = JSON.parse(await readFile(packageJsonPath, "utf8"));
let source = await readFile(sourcePath, "utf8");

source = source.replace(/\n\/\/# sourceMappingURL=.*\n?$/, "\n");

await mkdir(path.dirname(targetPath), { recursive: true });
await writeFile(targetPath, source, "utf8");

console.log(
  `Wrote assets/js/datastar.js from @starfederation/datastar@${packageJson.version}`,
);
