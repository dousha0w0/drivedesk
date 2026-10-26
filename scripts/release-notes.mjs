#!/usr/bin/env node
/**
 * Prints the CHANGELOG section for a version, used as the GitHub release body
 * (which in turn becomes `notes` in latest.json and the in-app update dialog).
 *
 *   node scripts/release-notes.mjs 0.1.0
 */
import { readFileSync } from "node:fs";

const version = (process.argv[2] ?? "").replace(/^v/, "");
const fallback = `DriveDesk ${version}\n\nSee CHANGELOG.md for details.`;

let changelog = "";
try {
  changelog = readFileSync(new URL("../CHANGELOG.md", import.meta.url), "utf8");
} catch {
  console.log(fallback);
  process.exit(0);
}

const lines = changelog.split("\n");
const start = lines.findIndex((line) => new RegExp(`^##\\s*\\[?v?${version.replace(/\./g, "\\.")}\\]?`).test(line.trim()));
if (start === -1) {
  console.log(fallback);
  process.exit(0);
}

const body = [];
for (let index = start + 1; index < lines.length; index += 1) {
  if (/^##\s/.test(lines[index])) break;
  body.push(lines[index]);
}

const text = body.join("\n").trim();
console.log(text.length > 0 ? text : fallback);
