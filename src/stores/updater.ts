import { defineStore } from "pinia";
import { computed, ref, shallowRef } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { errorMessage } from "@/lib/errors";
import { api } from "@/lib/api";
import { useUiStore } from "@/stores/ui";
import { i18n } from "@/i18n";

const STORAGE_KEY = "drivedesk.updater";

interface UpdaterPrefs {
  autoCheck: boolean;
  ignoredVersion: string | null;
  lastCheckAt: number | null;
}

const DEFAULTS: UpdaterPrefs = { autoCheck: true, ignoredVersion: null, lastCheckAt: null };

function loadPrefs(): UpdaterPrefs {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    const parsed = raw ? JSON.parse(raw) : null;
    return {
      autoCheck: parsed?.autoCheck !== false,
      ignoredVersion: typeof parsed?.ignoredVersion === "string" ? parsed.ignoredVersion : null,
      lastCheckAt: typeof parsed?.lastCheckAt === "number" ? parsed.lastCheckAt : null,
    };
  } catch {
    return { ...DEFAULTS };
  }
}

function savePrefs(prefs: UpdaterPrefs): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
  } catch {
    // ignore storage failures
  }
}

/**
 * Update state shared by the toolbar badge, the settings section and the
 * update dialog. `Update` is a class instance, so it lives in a shallowRef to
 * keep Vue from proxying its methods.
 */
export const useUpdaterStore = defineStore("updater", () => {
  const ui = useUiStore();
  const t = (key: string, params?: Record<string, unknown>) => i18n.global.t(key, params ?? {});

  const prefs = ref<UpdaterPrefs>(loadPrefs());
  const currentVersion = ref("0.1.0");
  const update = shallowRef<Update | null>(null);
  const checking = ref(false);
  const installing = ref(false);
  const progress = ref<{ downloaded: number; total: number | null } | null>(null);
  const error = ref<string | null>(null);
  const dialogOpen = ref(false);

  const availableVersion = computed(() => update.value?.version ?? null);
  const notes = computed(() => update.value?.body ?? "");
  const publishedAt = computed(() => update.value?.date ?? null);
  const percent = computed(() => {
    if (!progress.value) return 0;
    if (!progress.value.total) return installing.value ? 100 : 0;
    return Math.min(100, (progress.value.downloaded / progress.value.total) * 100);
  });

  function persist(): void {
    savePrefs(prefs.value);
  }

  function setAutoCheck(enabled: boolean): void {
    prefs.value.autoCheck = enabled;
    persist();
  }

  async function init(): Promise<void> {
    currentVersion.value = await getVersion().catch(() => "0.1.0");
    if (prefs.value.autoCheck) {
      // Do not compete with the first paint or the folder listing.
      window.setTimeout(() => void checkForUpdates({ silent: true }), 4000);
    }
  }

  async function checkForUpdates(options: { silent?: boolean } = {}): Promise<void> {
    if (checking.value || installing.value) return;
    checking.value = true;
    error.value = null;
    try {
      const found = await check();
      prefs.value.lastCheckAt = Date.now();
      persist();
      update.value = found && prefs.value.ignoredVersion !== found.version ? found : null;
      void api.log("info", update.value ? `updater: available ${update.value.version}` : "updater: up to date");
      if (!update.value && options.silent !== true) {
        ui.toast(t("updates.upToDate", { version: currentVersion.value }), "success");
      }
      if (update.value) dialogOpen.value = true;
    } catch (caught) {
      error.value = errorMessage(caught);
      void api.log("warn", `updater: check failed: ${error.value}`);
      if (options.silent !== true) ui.toast(error.value, "error", 5200);
    } finally {
      checking.value = false;
    }
  }

  async function install(): Promise<void> {
    if (!update.value || installing.value) return;
    installing.value = true;
    error.value = null;
    progress.value = { downloaded: 0, total: null };
    try {
      await update.value.downloadAndInstall((event) => {
        if (event.event === "Started") {
          progress.value = { downloaded: 0, total: event.data.contentLength ?? null };
        } else if (event.event === "Progress") {
          progress.value = {
            downloaded: (progress.value?.downloaded ?? 0) + event.data.chunkLength,
            total: progress.value?.total ?? null,
          };
        } else if (event.event === "Finished") {
          progress.value = {
            downloaded: progress.value?.total ?? progress.value?.downloaded ?? 0,
            total: progress.value?.total ?? null,
          };
        }
      });
      await relaunch();
    } catch (caught) {
      error.value = errorMessage(caught);
      installing.value = false;
    }
  }

  function ignoreVersion(): void {
    if (!update.value) return;
    prefs.value.ignoredVersion = update.value.version;
    persist();
    update.value = null;
    dialogOpen.value = false;
  }

  function clearIgnored(): void {
    prefs.value.ignoredVersion = null;
    persist();
  }

  return {
    prefs,
    currentVersion,
    update,
    checking,
    installing,
    progress,
    error,
    dialogOpen,
    availableVersion,
    notes,
    publishedAt,
    percent,
    init,
    checkForUpdates,
    install,
    ignoreVersion,
    clearIgnored,
    setAutoCheck,
  };
});
