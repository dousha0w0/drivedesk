#!/usr/bin/env node
/**
 * Builds DriveDesk with your own Google OAuth client baked in, so users get a
 * single "Sign in with Google" button and never touch the Cloud console.
 *
 *   pnpm bundle:with-client ~/Downloads/client_secret_xxx.json
 *   pnpm bundle:with-client ~/Downloads/client_secret_xxx.json --check
 *   pnpm bundle:with-client ~/Downloads/client_secret_xxx.json --bundles app
 *
 * A Desktop-app client secret is public by design (installed apps cannot keep
 * secrets), so embedding it is the normal approach. What it does not do is
 * bypass Google's verification rules — see README "一键登录不是万能的".
 */
import { spawn } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const argv = process.argv.slice(2);
const check = argv.includes("--check");
const passthrough = argv.filter((arg) => arg.startsWith("--") && arg !== "--check");
const file = argv.find((arg) => !arg.startsWith("--"));

function fail(message) {
  console.error(`bundle:with-client: ${message}`);
  process.exit(1);
}

if (!file) {
  console.error(`用法: pnpm bundle:with-client <client_secret.json> [--check] [tauri build 参数]

  --check   只校验并打印将注入的客户端，不执行构建
  其余参数  原样传给 tauri build，例如 --bundles app`);
  process.exit(1);
}

let parsed;
try {
  parsed = JSON.parse(readFileSync(resolve(file), "utf8"));
} catch (error) {
  fail(`无法读取 ${file}: ${error.message}`);
}

if (!parsed.installed) {
  if (parsed.web) {
    fail("这是「Web 应用」类型的客户端，不能用 loopback 回调。请在 Google Cloud 重新创建「桌面应用（Desktop app）」类型。");
  }
  fail("JSON 里没有 installed 段，不是 OAuth 桌面客户端文件。");
}

const clientId = String(parsed.installed.client_id ?? "").trim();
const clientSecret = String(parsed.installed.client_secret ?? "").trim();

if (!clientId || !clientSecret) {
  fail("客户端 JSON 缺少 client_id 或 client_secret。");
}
if (!clientId.endsWith(".apps.googleusercontent.com")) {
  console.warn(`bundle:with-client: 警告：client_id 不像 Google 客户端（${clientId}）`);
}

const maskedId = `${clientId.slice(0, 12)}…${clientId.slice(-24)}`;
console.log(`类型       桌面应用 ✓`);
console.log(`client_id  ${maskedId}`);
console.log(`secret     ${clientSecret.slice(0, 7)}…（长度 ${clientSecret.length}）`);
console.log(`作用       构建产物内置该客户端 → 登录页只显示「使用 Google 登录」`);
console.log(`提醒       未通过 Google 验证时，仅测试用户可登录且令牌 7 天过期`);

if (check) {
  console.log(`\ncheck ok：未执行构建。去掉 --check 即开始打包。`);
  process.exit(0);
}

const child = spawn("pnpm", ["tauri", "build", ...passthrough], {
  stdio: "inherit",
  env: {
    ...process.env,
    DRIVEDESK_GOOGLE_CLIENT_ID: clientId,
    DRIVEDESK_GOOGLE_CLIENT_SECRET: clientSecret,
  },
  shell: process.platform === "win32",
});

child.on("exit", (code) => process.exit(code ?? 1));
