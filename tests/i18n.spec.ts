import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import zh from "@/i18n/locales/zh-CN";
import en from "@/i18n/locales/en";

type Tree = { [key: string]: string | Tree };

function flatten(tree: Tree, prefix = ""): string[] {
  return Object.entries(tree).flatMap(([key, value]) =>
    typeof value === "string" ? [`${prefix}${key}`] : flatten(value, `${prefix}${key}.`),
  );
}

function lookup(tree: Tree, key: string): string | undefined {
  let node: string | Tree | undefined = tree;
  for (const part of key.split(".")) {
    if (typeof node !== "object") return undefined;
    node = node[part];
  }
  return typeof node === "string" ? node : undefined;
}

const SOURCE_ROOT = resolve(import.meta.dirname, "../src");

function sourceFiles(dir: string, out: string[] = []): string[] {
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) {
      if (entry === "i18n" || entry === "__tests__") continue;
      sourceFiles(path, out);
    } else if (entry.endsWith(".vue") || entry.endsWith(".ts")) {
      out.push(path);
    }
  }
  return out;
}

describe("locale coverage", () => {
  it("zh-CN and en define exactly the same keys", () => {
    expect(flatten(en as unknown as Tree).sort()).toEqual(flatten(zh as unknown as Tree).sort());
  });

  it("every static key referenced in source exists in both locales", () => {
    const pattern = /(?:\$t|\bt)\(\s*"([a-zA-Z0-9_.]+)"/g;
    const missing: string[] = [];
    for (const file of sourceFiles(SOURCE_ROOT)) {
      const source = readFileSync(file, "utf8");
      for (const match of source.matchAll(pattern)) {
        const key = match[1];
        if (!lookup(zh as unknown as Tree, key) || !lookup(en as unknown as Tree, key)) {
          missing.push(`${file.replace(SOURCE_ROOT, "src")}: ${key}`);
        }
      }
    }
    expect(missing).toEqual([]);
  });
});
