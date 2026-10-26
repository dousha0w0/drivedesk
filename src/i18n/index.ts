import { createI18n } from "vue-i18n";
import zhCN from "./locales/zh-CN";
import en from "./locales/en";

export type Locale = "zh-CN" | "en";
export const LOCALES: { value: Locale; label: string }[] = [
  { value: "zh-CN", label: "简体中文" },
  { value: "en", label: "English" },
];

const STORAGE_KEY = "drivedesk.locale";

function initialLocale(): Locale {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "zh-CN" || saved === "en") return saved;
  } catch {
    // storage unavailable
  }
  return (navigator.language || "").toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

export const i18n = createI18n({
  legacy: false,
  locale: initialLocale(),
  fallbackLocale: "en",
  messages: { "zh-CN": zhCN, en },
});

export function setLocale(locale: Locale): void {
  i18n.global.locale.value = locale;
  try {
    localStorage.setItem(STORAGE_KEY, locale);
  } catch {
    // ignore storage failures
  }
}

export function currentLocale(): Locale {
  return i18n.global.locale.value as Locale;
}
