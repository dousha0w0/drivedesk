import { defineStore } from "pinia";
import { ref, watch } from "vue";
import type { UiPreferences } from "@/types/drive";
import { applyAppearance, loadPreferences, sanitizePreferences, savePreferences, watchSystemTheme } from "@/lib/prefs";

export type ToastKind = "info" | "success" | "error";

export interface Toast {
  id: number;
  message: string;
  kind: ToastKind;
}

export interface MenuItem {
  label: string;
  icon?: string;
  shortcut?: string;
  danger?: boolean;
  disabled?: boolean;
  separator?: boolean;
  action?: () => void;
}

export interface MenuState {
  x: number;
  y: number;
  items: MenuItem[];
}

const MENU_WIDTH = 216;
const MENU_ROW = 28;

export const useUiStore = defineStore("ui", () => {
  const prefs = ref<UiPreferences>(loadPreferences());
  const dark = ref(applyAppearance(prefs.value));
  const toasts = ref<Toast[]>([]);
  const settingsOpen = ref(false);
  const loginOpen = ref(false);
  const menu = ref<MenuState | null>(null);
  let toastSequence = 0;

  function persist(): void {
    savePreferences(prefs.value);
  }

  function setPrefs(patch: Partial<UiPreferences>): void {
    prefs.value = sanitizePreferences({ ...prefs.value, ...patch });
    dark.value = applyAppearance(prefs.value);
    persist();
  }

  function toast(message: string, kind: ToastKind = "info", timeout = 3600): number {
    const id = ++toastSequence;
    toasts.value.push({ id, message, kind });
    if (timeout > 0) {
      window.setTimeout(() => dismissToast(id), timeout);
    }
    return id;
  }

  function dismissToast(id: number): void {
    toasts.value = toasts.value.filter((item) => item.id !== id);
  }

  function openMenu(event: MouseEvent, items: MenuItem[]): void {
    const height = items.reduce((total, item) => total + (item.separator ? 9 : MENU_ROW), 8);
    const x = Math.min(event.clientX, window.innerWidth - MENU_WIDTH - 8);
    const y = Math.min(event.clientY, Math.max(8, window.innerHeight - height - 8));
    menu.value = { x, y, items };
  }

  function closeMenu(): void {
    menu.value = null;
  }

  function toggleRightPanel(tab?: "preview" | "details"): void {
    if (tab && prefs.value.rightPanelTab !== tab) {
      setPrefs({ rightPanelTab: tab, rightPanelOpen: true });
      return;
    }
    setPrefs({ rightPanelOpen: !prefs.value.rightPanelOpen });
  }

  function toggleDock(): void {
    setPrefs({ transferDockOpen: !prefs.value.transferDockOpen });
  }

  function setPanelSize(key: "sidebarWidth" | "rightPanelWidth" | "dockHeight", value: number): void {
    setPrefs({ [key]: value } as Partial<UiPreferences>);
  }

  const stopSystemWatch = watchSystemTheme(() => {
    if (prefs.value.themeMode === "system") {
      dark.value = applyAppearance(prefs.value);
    }
  });

  watch(
    () => prefs.value,
    () => applyAppearance(prefs.value),
    { deep: true },
  );

  return {
    prefs,
    dark,
    toasts,
    settingsOpen,
    loginOpen,
    menu,
    setPrefs,
    toast,
    dismissToast,
    openMenu,
    closeMenu,
    toggleRightPanel,
    toggleDock,
    setPanelSize,
    stopSystemWatch,
  };
});
