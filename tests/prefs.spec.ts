import { beforeEach, describe, expect, it, vi } from "vitest";
import { applyAppearance, DEFAULT_PREFERENCES, loadPreferences, sanitizePreferences, savePreferences } from "@/lib/prefs";

describe("sanitizePreferences", () => {
  it("fills defaults for an empty blob", () => {
    expect(sanitizePreferences({})).toEqual(DEFAULT_PREFERENCES);
    expect(sanitizePreferences(null)).toEqual(DEFAULT_PREFERENCES);
  });

  it("clamps panel sizes into their usable range", () => {
    const tiny = sanitizePreferences({ sidebarWidth: 10, rightPanelWidth: 10, dockHeight: 10 });
    expect(tiny.sidebarWidth).toBe(220);
    expect(tiny.rightPanelWidth).toBe(300);
    expect(tiny.dockHeight).toBe(110);

    const huge = sanitizePreferences({ sidebarWidth: 5000, rightPanelWidth: 5000, dockHeight: 5000 });
    expect(huge.sidebarWidth).toBe(420);
    expect(huge.rightPanelWidth).toBe(720);
    expect(huge.dockHeight).toBe(360);
  });

  it("rejects unknown enum values instead of trusting storage", () => {
    const preferences = sanitizePreferences({
      themeMode: "neon" as never,
      palette: "rainbow" as never,
      corners: "round" as never,
      layout: "floating" as never,
      density: "cosy" as never,
      rightPanelTab: "graph" as never,
    });
    expect(preferences.themeMode).toBe("system");
    expect(preferences.palette).toBe("neutral");
    expect(preferences.corners).toBe("default");
    expect(preferences.layout).toBe("classic");
    expect(preferences.density).toBe("comfortable");
    expect(preferences.rightPanelTab).toBe("preview");
  });
});

describe("persistence", () => {
  beforeEach(() => window.localStorage.clear());

  it("round-trips through localStorage and survives corrupt JSON", () => {
    const preferences = sanitizePreferences({ themeMode: "dark", layout: "separated", sidebarWidth: 300 });
    savePreferences(preferences);
    expect(loadPreferences()).toEqual(preferences);

    window.localStorage.setItem("drivedesk.preferences", "{not json");
    expect(loadPreferences()).toEqual(DEFAULT_PREFERENCES);
  });
});

describe("applyAppearance", () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.documentElement.className = "";
    delete document.documentElement.dataset.corners;
  });

  it("toggles the dark class, palette class and corner data attribute", () => {
    const dark = applyAppearance({ ...DEFAULT_PREFERENCES, themeMode: "dark", palette: "graphite", corners: "none" });
    expect(dark).toBe(true);
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(document.documentElement.classList.contains("palette-graphite")).toBe(true);
    expect(document.documentElement.dataset.corners).toBe("none");

    applyAppearance({ ...DEFAULT_PREFERENCES, themeMode: "light", palette: "neutral", corners: "default" });
    expect(document.documentElement.classList.contains("dark")).toBe(false);
    expect(document.documentElement.classList.contains("palette-graphite")).toBe(false);
    expect(document.documentElement.dataset.corners).toBeUndefined();
  });

  it("follows the system preference when the mode is system", () => {
    vi.spyOn(window, "matchMedia").mockReturnValue({ matches: true } as MediaQueryList);
    expect(applyAppearance({ ...DEFAULT_PREFERENCES, themeMode: "system" })).toBe(true);
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });
});
