<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import AppModal from "@/components/ui/AppModal.vue";
import AppButton from "@/components/ui/AppButton.vue";
import SwitchToggle from "@/components/ui/SwitchToggle.vue";
import Icon from "@/components/ui/Icon.vue";
import { LOCALES, currentLocale, i18n, setLocale, type Locale } from "@/i18n";
import { api } from "@/lib/api";
import { errorMessage } from "@/lib/errors";
import { useSessionStore } from "@/stores/session";
import { useUiStore } from "@/stores/ui";
import type { BackendSettings, CornerStyle, LayoutMode, Palette, ThemeMode, Density } from "@/types/drive";

const ui = useUiStore();
const session = useSessionStore();
const t = (key: string, params?: Record<string, unknown>) => i18n.global.t(key, params ?? {});

const settings = ref<BackendSettings | null>(null);
const saving = ref(false);
const notice = ref<string | null>(null);
const probing = ref(false);

watch(
  () => [ui.settingsOpen, session.settings] as const,
  () => {
    if (session.settings) settings.value = { ...session.settings };
    notice.value = null;
  },
  { immediate: true },
);

const themes = computed<{ value: ThemeMode; label: string }[]>(() => [
  { value: "light", label: t("settings.themeLight") },
  { value: "dark", label: t("settings.themeDark") },
  { value: "system", label: t("settings.themeSystem") },
]);
const palettes = computed<{ value: Palette; label: string }[]>(() => [
  { value: "neutral", label: t("settings.paletteNeutral") },
  { value: "graphite", label: t("settings.paletteGraphite") },
  { value: "vscode", label: t("settings.paletteVscode") },
]);
const corners = computed<{ value: CornerStyle; label: string }[]>(() => [
  { value: "default", label: t("settings.cornersDefault") },
  { value: "small", label: t("settings.cornersSmall") },
  { value: "none", label: t("settings.cornersNone") },
]);
const layouts = computed<{ value: LayoutMode; label: string; hint: string }[]>(() => [
  { value: "classic", label: t("settings.layoutClassic"), hint: t("settings.layoutClassicHint") },
  { value: "separated", label: t("settings.layoutSeparated"), hint: t("settings.layoutSeparatedHint") },
]);
const densities = computed<{ value: Density; label: string }[]>(() => [
  { value: "comfortable", label: t("settings.densityComfortable") },
  { value: "compact", label: t("settings.densityCompact") },
]);

const locale = ref<Locale>(currentLocale());
const gdirectStatus = computed(() => session.gdirect);

async function persist(patch: Parameters<typeof session.updateSettings>[0]): Promise<void> {
  saving.value = true;
  notice.value = null;
  try {
    await session.updateSettings(patch);
    settings.value = session.settings ? { ...session.settings } : null;
  } catch (caught) {
    notice.value = errorMessage(caught);
  } finally {
    saving.value = false;
  }
}

async function browseDownloadDir(): Promise<void> {
  const picked = await openDialog({ directory: true, multiple: false });
  if (!picked || Array.isArray(picked)) return;
  await persist({ downloadDir: picked });
}

async function browseGdirect(): Promise<void> {
  const picked = await openDialog({ multiple: false, filters: [{ name: "gdirect", extensions: ["*"] }] });
  if (!picked || Array.isArray(picked)) return;
  await persist({ gdirectPath: picked });
  await session.probeGdirect(picked);
}

async function probe(): Promise<void> {
  probing.value = true;
  try {
    await session.probeGdirect(settings.value?.gdirectPath ?? null);
  } finally {
    probing.value = false;
  }
}

async function clearCache(): Promise<void> {
  try {
    await session.clearCache();
    ui.toast(t("toast.cacheCleared"), "success");
  } catch (caught) {
    ui.toast(errorMessage(caught), "error");
  }
}

async function signOut(email: string): Promise<void> {
  try {
    await session.signOut(email);
    ui.toast(t("toast.loggedOut"), "success");
  } catch (caught) {
    ui.toast(errorMessage(caught), "error");
  }
}

function chooseLocale(next: Locale): void {
  locale.value = next;
  setLocale(next);
}


</script>

<template>
  <AppModal :open="ui.settingsOpen" :title="$t('settings.title')" :width="700" @close="ui.settingsOpen = false">
    <div class="space-y-5 pb-3">
      <section>
        <h3 class="mb-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">{{ $t("settings.appearance") }}</h3>
        <div class="settings-row">
          <span>{{ $t("settings.theme") }}</span>
          <div class="segmented">
            <button
              v-for="theme in themes"
              :key="theme.value"
              :data-active="ui.prefs.themeMode === theme.value || undefined"
              @click="ui.setPrefs({ themeMode: theme.value })"
            >
              {{ theme.label }}
            </button>
          </div>
        </div>
        <div class="settings-row">
          <span>{{ $t("settings.palette") }}</span>
          <div class="segmented">
            <button
              v-for="palette in palettes"
              :key="palette.value"
              :data-active="ui.prefs.palette === palette.value || undefined"
              @click="ui.setPrefs({ palette: palette.value })"
            >
              {{ palette.label }}
            </button>
          </div>
        </div>
        <div class="settings-row">
          <span>{{ $t("settings.corners") }}</span>
          <div class="segmented">
            <button
              v-for="corner in corners"
              :key="corner.value"
              :data-active="ui.prefs.corners === corner.value || undefined"
              @click="ui.setPrefs({ corners: corner.value })"
            >
              {{ corner.label }}
            </button>
          </div>
        </div>
        <div class="settings-row items-start">
          <span>{{ $t("settings.layout") }}</span>
          <div class="flex flex-col gap-1">
            <div class="segmented">
              <button
                v-for="layout in layouts"
                :key="layout.value"
                :data-active="ui.prefs.layout === layout.value || undefined"
                @click="ui.setPrefs({ layout: layout.value })"
              >
                {{ layout.label }}
              </button>
            </div>
            <span class="text-[10.5px] text-muted-foreground">
              {{ layouts.find((layout) => layout.value === ui.prefs.layout)?.hint }}
            </span>
          </div>
        </div>
        <div class="settings-row">
          <span>{{ $t("settings.density") }}</span>
          <div class="segmented">
            <button
              v-for="density in densities"
              :key="density.value"
              :data-active="ui.prefs.density === density.value || undefined"
              @click="ui.setPrefs({ density: density.value })"
            >
              {{ density.label }}
            </button>
          </div>
        </div>
        <div class="settings-row">
          <span>{{ $t("settings.language") }}</span>
          <div class="segmented">
            <button v-for="item in LOCALES" :key="item.value" :data-active="locale === item.value || undefined" @click="chooseLocale(item.value)">
              {{ item.label }}
            </button>
          </div>
        </div>
      </section>

      <section>
        <h3 class="mb-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">{{ $t("settings.downloads") }}</h3>
        <div class="settings-row">
          <span>{{ $t("settings.downloadDir") }}</span>
          <div class="flex min-w-0 flex-1 items-center gap-1.5">
            <input :value="settings?.downloadDir ?? ''" readonly class="settings-input flex-1" />
            <AppButton size="sm" variant="secondary" icon="folder" @click="browseDownloadDir">{{ $t("actions.browse") }}</AppButton>
          </div>
        </div>
        <div class="settings-row">
          <span>{{ $t("settings.downloadConcurrency") }}</span>
          <input
            :value="settings?.downloadConcurrency ?? 3"
            type="number"
            min="1"
            max="8"
            class="settings-input w-20"
            @change="persist({ downloadConcurrency: Number(($event.target as HTMLInputElement).value) })"
          />
        </div>
        <div class="settings-row">
          <span>{{ $t("settings.uploadConcurrency") }}</span>
          <input
            :value="settings?.uploadConcurrency ?? 2"
            type="number"
            min="1"
            max="8"
            class="settings-input w-20"
            @change="persist({ uploadConcurrency: Number(($event.target as HTMLInputElement).value) })"
          />
        </div>
      </section>

      <section>
        <h3 class="mb-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">{{ $t("settings.gdirect") }}</h3>
        <div class="settings-row">
          <span>{{ $t("settings.gdirectEnable") }}</span>
          <SwitchToggle
            :model-value="settings?.gdirectEnabled ?? false"
            :label="$t('settings.gdirectEnable')"
            @update:model-value="persist({ gdirectEnabled: $event })"
          />
        </div>
        <div class="settings-row">
          <span>{{ $t("settings.gdirectPath") }}</span>
          <div class="flex min-w-0 flex-1 items-center gap-1.5">
            <input :value="settings?.gdirectPath ?? ''" readonly placeholder="PATH" class="settings-input flex-1" />
            <AppButton size="sm" variant="secondary" icon="folder" @click="browseGdirect">{{ $t("actions.browse") }}</AppButton>
            <AppButton size="sm" variant="ghost" icon="refresh" :disabled="probing" @click="probe">{{ $t("actions.retry") }}</AppButton>
          </div>
        </div>
        <div class="settings-row items-start" v-if="gdirectStatus">
          <span />
          <span class="text-[11px]" :class="gdirectStatus.found ? 'text-success' : 'text-muted-foreground'">
            {{ gdirectStatus.found ? $t("settings.gdirectFound", { version: gdirectStatus.version || "" }) : $t("settings.gdirectMissing") }}
          </span>
        </div>
        <p class="mt-1 text-[11px] leading-relaxed text-muted-foreground">{{ $t("settings.gdirectHint") }}</p>
      </section>

      <section>
        <h3 class="mb-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">{{ $t("settings.account") }}</h3>
        <div v-for="account in session.accounts" :key="account.email" class="settings-row">
          <span class="flex min-w-0 items-center gap-1.5">
            <Icon name="account" :size="13" class="text-muted-foreground" />
            <span class="truncate">{{ account.email }}</span>
            <span v-if="account.isDefault" class="badge" data-tone="accent">default</span>
            <span v-if="account.authKind === 'token'" class="badge" data-tone="warning">token</span>
          </span>
          <AppButton size="sm" variant="ghost" icon="logout" @click="signOut(account.email)">{{ $t("settings.signOut") }}</AppButton>
        </div>
        <div class="settings-row">
          <span>{{ $t("settings.storage") }}</span>
          <span class="truncate text-[11.5px] text-muted-foreground">{{ session.info?.secretStorage }}</span>
        </div>
        <div class="settings-row">
          <span>{{ $t("settings.dataDir") }}</span>
          <span class="truncate font-mono text-[10.5px] text-muted-foreground" :title="session.info?.dataDir">{{ session.info?.dataDir }}</span>
        </div>
        <div class="settings-row">
          <span>{{ $t("settings.cache") }}</span>
          <AppButton size="sm" variant="secondary" icon="trash" @click="clearCache">{{ $t("settings.clearCache") }}</AppButton>
        </div>
      </section>

      <section>
        <h3 class="mb-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">{{ $t("settings.about") }}</h3>
        <div class="settings-row">
          <span>{{ $t("settings.version") }}</span>
          <span class="text-[11.5px] text-muted-foreground">DriveDesk {{ session.info?.version }} · Tauri + Vue</span>
        </div>
      </section>

      <div v-if="notice" class="text-[11.5px] text-destructive">{{ notice }}</div>
    </div>
  </AppModal>
</template>

<style scoped>
.settings-row {
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 30px;
  padding: 3px 0;
  font-size: 12px;
}
.settings-row > span:first-child {
  width: 150px;
  flex: 0 0 auto;
  color: var(--muted-foreground);
}
.settings-input {
  height: 28px;
  border: 1px solid var(--input);
  border-radius: var(--dr-sm);
  background: var(--background);
  padding: 0 8px;
  font-size: 12px;
  outline: none;
}
.settings-input:focus {
  border-color: var(--ring);
}
.segmented {
  display: inline-flex;
  padding: 2px;
  gap: 2px;
  border: 1px solid var(--border);
  border-radius: var(--dr-md);
  background: var(--muted);
}
.segmented button {
  height: 24px;
  padding: 0 8px;
  border-radius: calc(var(--dr-md) - 3px);
  font-size: 11.5px;
  color: var(--muted-foreground);
}
.segmented button[data-active] {
  background: var(--card);
  color: var(--foreground);
  box-shadow: 0 1px 2px rgb(0 0 0 / 0.08);
}
.segmented button:hover:not([data-active]) {
  color: var(--foreground);
}
</style>
