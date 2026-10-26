#!/usr/bin/env node
/**
 * Runs the scripted UI smoke against the release binary in demo mode and
 * fails the process when any step reports false.
 *
 *   pnpm test:smoke
 *   DRIVEDESK_BIN=path/to/drivedesk pnpm test:smoke
 */
import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const binary =
  process.env.DRIVEDESK_BIN ??
  join(
    "src-tauri",
    "target",
    "release",
    process.platform === "win32" ? "drivedesk.exe" : "drivedesk",
  );

if (!existsSync(binary)) {
  console.error(`smoke: binary not found at ${binary}\nBuild it first: pnpm tauri build --no-bundle (or cargo build --release --features custom-protocol)`);
  process.exit(1);
}

const report = join(mkdtempSync(join(tmpdir(), "drivedesk-smoke-")), "report.json");

const child = spawn(binary, [], {
  env: {
    ...process.env,
    DRIVEDESK_MOCK: "1",
    DRIVEDESK_SMOKE: "1",
    DRIVEDESK_SMOKE_OUT: report,
  },
  stdio: ["ignore", "inherit", "inherit"],
});

const timeout = setTimeout(() => {
  console.error("smoke: timed out after 120s");
  child.kill("SIGKILL");
}, 120_000);

child.on("exit", (code) => {
  clearTimeout(timeout);
  if (!existsSync(report)) {
    console.error(`smoke: no report written (exit ${code})`);
    process.exit(1);
  }
  const result = JSON.parse(readFileSync(report, "utf8"));
  const failed = result.steps.filter((step) => !step.ok);
  for (const step of result.steps) {
    console.log(`${step.ok ? "PASS" : "FAIL"}  ${step.name}${step.detail ? `  ${step.detail}` : ""}`);
  }
  console.log(`\nsmoke: ${result.steps.length - failed.length}/${result.steps.length} steps passed`);
  process.exit(failed.length === 0 ? 0 : 1);
});
