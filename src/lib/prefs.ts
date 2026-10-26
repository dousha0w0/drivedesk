import type { CornerStyle, LayoutMode, Palette, RightPanelTab, ThemeMode, UiPreferences } from "@/types/drive";

const STORAGE_KEY = "drivedesk.preferences";

export const DEFAULT_PREFERENCES: UiPreferences = {
  themeMode: "system",
  palette: "neutral",
  corners: "default",
  layout: "classic",
  density: "comfortable",
  rightPanelTab: "preview",
  rightPanelOpen: true,
  transferDockOpen: true,
  sidebarWidth: 260,
  rightPanelWidth: 360,
  dockHeight: 168,
};

function clamp(value: unknown, min: number, max: number, fallback: number): number {
  const number = typeof value === "number" && Number.isFinite(value) ? value : fallback;
  return Math.min(max, Math.max(min, Math.round(number)));
}

function pick<T extends string>(value: unknown, allowed: readonly T[], fallback: T): T {
  return allowed.includes(value as T) ? (value as T) : fallback;
}

export function sanitizePreferences(raw: Partial<UiPreferences> | null | undefined): UiPreferences {
  const merged = { ...DEFAULT_PREFERENCES, ...(raw ?? {}) };
  return {
    themeMode: pick<ThemeMode>(merged.themeMode, ["light", "dark", "system"], "system"),
    palette: pick<Palette>(merged.palette, ["neutral", "graphite", "vscode"], "neutral"),
    corners: pick<CornerStyle>(merged.corners, ["default", "small", "none"], "default"),
    layout: pick<LayoutMode>(merged.layout, ["classic", "separated"], "classic"),
    density: pick(merged.density, ["comfortable", "compact"], "comfortable"),
    rightPanelTab: pick<RightPanelTab>(merged.rightPanelTab, ["preview", "details"], "preview"),
    rightPanelOpen: merged.rightPanelOpen !== false,
    transferDockOpen: merged.transferDockOpen !== false,
    sidebarWidth: clamp(merged.sidebarWidth, 220, 420, DEFAULT_PREFERENCES.sidebarWidth),
    rightPanelWidth: clamp(merged.rightPanelWidth, 300, 720, DEFAULT_PREFERENCES.rightPanelWidth),
    dockHeight: clamp(merged.dockHeight, 110, 360, DEFAULT_PREFERENCES.dockHeight),
  };
}

export function loadPreferences(): UiPreferences {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return sanitizePreferences(raw ? JSON.parse(raw) : null);
  } catch {
    return { ...DEFAULT_PREFERENCES };
  }
}

export function savePreferences(preferences: UiPreferences): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(preferences));
  } catch {
    // A locked-down webview without storage still gets a working session.
  }
}

function systemPrefersDark(): boolean {
  return typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: dark)").matches;
}

/**
 * Apply appearance to <html> before the first paint (index.html replays the
 * same values from localStorage) and keep the startup script's keys in sync.
 */
export function applyAppearance(preferences: UiPreferences): boolean {
  const dark =
    preferences.themeMode === "dark" || (preferences.themeMode === "system" && systemPrefersDark());
  const root = document.documentElement;
  root.classList.toggle("dark", dark);
  root.style.colorScheme = dark ? "dark" : "light";
  for (const palette of ["neutral", "graphite", "vscode"]) {
    root.classList.toggle(`palette-${palette}`, palette === preferences.palette && palette !== "neutral");
  }
  if (preferences.corners === "default") {
    delete root.dataset.corners;
  } else {
    root.dataset.corners = preferences.corners;
  }
  try {
    localStorage.setItem("drivedesk.theme", preferences.themeMode);
    localStorage.setItem("drivedesk.palette", preferences.palette);
    localStorage.setItem("drivedesk.corners", preferences.corners);
  } catch {
    // ignore storage failures
  }
  return dark;
}

export function watchSystemTheme(onChange: () => void): () => void {
  if (typeof window === "undefined" || !window.matchMedia) return () => {};
  const query = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = () => onChange();
  query.addEventListener("change", handler);
  return () => query.removeEventListener("change", handler);
}
