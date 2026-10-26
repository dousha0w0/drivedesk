import { i18n } from "@/i18n";
import { googleErrorHint } from "@/lib/googleErrors";

/** Tauri rejects with strings and JS throws Errors; both reach a toast. */
export function errorMessage(error: unknown): string {
  const raw =
    typeof error === "string"
      ? error
      : error instanceof Error
        ? error.message
        : typeof error === "object" && error !== null && "message" in error && typeof error.message === "string"
          ? error.message
          : String(error);

  const hint = googleErrorHint(raw);
  if (!hint) return raw;
  const short = raw.length > 200 ? `${raw.slice(0, 200)}…` : raw;
  return `${i18n.global.t(hint)} — ${short}`;
}
