import { describe, expect, it } from "vitest";
import { isMacPlatform } from "@/lib/platform";

describe("isMacPlatform", () => {
  it("accepts both the Tauri/Rust spelling and the legacy one", () => {
    expect(isMacPlatform("macos")).toBe(true);
    expect(isMacPlatform("darwin")).toBe(true);
  });

  it("rejects other platforms and missing info", () => {
    expect(isMacPlatform("windows")).toBe(false);
    expect(isMacPlatform("linux")).toBe(false);
    expect(isMacPlatform(null)).toBe(false);
    expect(isMacPlatform(undefined)).toBe(false);
  });
});
