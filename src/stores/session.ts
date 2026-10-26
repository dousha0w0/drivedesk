import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { api } from "@/lib/api";
import { errorMessage } from "@/lib/errors";
import type { Account, AppInfo, BackendSettings, GdirectInfo, Quota, SettingsPatch } from "@/types/drive";

export const useSessionStore = defineStore("session", () => {
  const info = ref<AppInfo | null>(null);
  const settings = ref<BackendSettings | null>(null);
  const quota = ref<Quota | null>(null);
  const gdirect = ref<GdirectInfo | null>(null);
  const loading = ref(true);
  const error = ref<string | null>(null);

  const accounts = computed<Account[]>(() => info.value?.accounts ?? []);
  const activeAccount = computed<Account | null>(() => accounts.value.find((account) => account.isDefault) ?? null);
  const isMock = computed(() => info.value?.mock === true);
  const signedIn = computed(() => accounts.value.length > 0);

  async function init(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      info.value = await api.appInfo();
      settings.value = await api.settingsGet();
      gdirect.value = { found: info.value.gdirectFound, path: info.value.gdirectPath, version: info.value.gdirectVersion };
      if (signedIn.value) {
        quota.value = await api.quotaGet().catch(() => null);
      }
    } catch (caught) {
      error.value = errorMessage(caught);
    } finally {
      loading.value = false;
    }
  }

  async function refresh(): Promise<void> {
    info.value = await api.appInfo();
    if (signedIn.value) {
      quota.value = await api.quotaGet().catch(() => null);
    }
  }

  async function login(clientJson: string | null): Promise<void> {
    const account = await api.accountLogin(clientJson);
    await refresh();
    return void account;
  }

  async function switchTo(email: string): Promise<void> {
    await api.accountSwitch(email);
    await refresh();
  }

  async function signOut(email: string): Promise<void> {
    await api.accountLogout(email);
    await refresh();
  }

  async function updateSettings(patch: SettingsPatch): Promise<void> {
    settings.value = await api.settingsUpdate(patch);
  }

  async function clearCache(): Promise<void> {
    await api.cacheClear();
  }

  async function probeGdirect(path?: string | null): Promise<void> {
    gdirect.value = await api.gdirectProbe(path ?? settings.value?.gdirectPath ?? null);
  }

  return {
    info,
    settings,
    quota,
    gdirect,
    loading,
    error,
    accounts,
    activeAccount,
    isMock,
    signedIn,
    init,
    refresh,
    login,
    switchTo,
    signOut,
    updateSettings,
    clearCache,
    probeGdirect,
  };
});
