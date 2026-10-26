#!/usr/bin/env node
/**
 * Keeps the three version fields in lockstep before tagging a release.
 *
 *   node scripts/bump-version.mjs 0.2.0
 */
import { readFileSync, writeFileSync } from "node:fs";

const version = (process.argv[2] ?? "").replace(/^v/, "");
if (!/^\d+\.\d+\.\d+$/.test(version)) {
  console.error("用法: node scripts/bump-version.mjs <major.minor.patch>");
  process.exit(1);
}

const packageJson = JSON.parse(readFileSync("package.json", "utf8"));
packageJson.version = version;
writeFileSync("package.json", JSON.stringify(packageJson, null, 2) + "\n");

const tauriConf = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
tauriConf.version = version;
writeFileSync("src-tauri/tauri.conf.json", JSON.stringify(tauriConf, null, 2) + "\n");

const cargoPath = "src-tauri/Cargo.toml";
let cargo = readFileSync(cargoPath, "utf8");
cargo = cargo.replace(/^(version\s*=\s*)"[^"]*"/m, `$1"${version}"`);
writeFileSync(cargoPath, cargo);

console.log(`版本已更新为 ${version}`);
console.log("下一步：在 CHANGELOG.md 写 ## [" + version + "] 段落，然后 git tag v" + version);
