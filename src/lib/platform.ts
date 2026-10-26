/**
 * Rust reports `std::env::consts::OS`, which is `"macos"` (what Tauri itself
 * uses), not `"darwin"`. Accept both so the window chrome never falls back to
 * the non-mac padding and lets the traffic lights sit on the toolbar buttons.
 */
export function isMacPlatform(platform: string | null | undefined): boolean {
  return platform === "macos" || platform === "darwin";
}
