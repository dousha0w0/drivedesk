<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import AppButton from "@/components/ui/AppButton.vue";
import Icon from "@/components/ui/Icon.vue";
import { useFilesStore } from "@/stores/files";
import { useSessionStore } from "@/stores/session";
import { useTransfersStore } from "@/stores/transfers";
import { useUiStore } from "@/stores/ui";
import { useDialogsStore } from "@/stores/dialogs";
import { i18n } from "@/i18n";
import { isMacPlatform } from "@/lib/platform";
import { api } from "@/lib/api";

const files = useFilesStore();
const session = useSessionStore();
const transfers = useTransfersStore();
const ui = useUiStore();
const dialogs = useDialogsStore();
const t = (key: string, params?: Record<string, unknown>) => i18n.global.t(key, params ?? {});

const searchText = ref("");
const searchFocused = ref(false);
const isMac = computed(() => isMacPlatform(session.info?.platform));
const canDownload = computed(() => files.selectedCount > 0 || files.previewId !== null);

watch(
  () => [files.view, files.query] as const,
  ([view, query]) => {
    if (view === "search") searchText.value = query;
    else if (!searchFocused.value) searchText.value = files.filter;
  },
  { immediate: true },
);

function onSearchInput(value: string): void {
  searchText.value = value;
  if (files.view === "search") {
    files.setFilter(value);
  }
}

function submitSearch(): void {
  void files.search(searchText.value);
}

function clearSearch(): void {
  searchText.value = "";
  files.setFilter("");
}

async function pickAndUpload(): Promise<void> {
  if (!session.signedIn) {
    ui.loginOpen = true;
    return;
  }
  const picked = await openDialog({ multiple: true });
  if (!picked) return;
  const paths = Array.isArray(picked) ? picked : [picked];
  await files.upload(paths.filter((path): path is string => typeof path === "string"));
}

function openAccountMenu(event: MouseEvent): void {
  const items = [
    ...session.accounts.map((account) => ({
      label: account.isDefault ? `${account.name} · ${account.email}` : account.email,
      icon: account.isDefault ? "check" : "account",
      action: () => {
        if (account.isDefault) return;
        session
          .switchTo(account.email)
          .then(() => {
            ui.toast(t("toast.switched", { email: account.email }), "success");
            return files.navigate({ view: "folder", folderId: "root", query: "" });
          })
          .catch((error: unknown) => ui.toast(String(error), "error"));
      },
    })),
    { separator: true, label: "" },
    {
      label: t("nav.addAccount"),
      icon: "plus",
      action: () => {
        ui.loginOpen = true;
      },
    },
    ...(session.activeAccount
      ? [
          {
            label: t("nav.signOut"),
            icon: "logout",
            danger: true,
            action: () => {
              session
                .signOut(session.activeAccount!.email)
                .then(() => ui.toast(t("toast.loggedOut"), "success"))
                .catch((error: unknown) => ui.toast(String(error), "error"));
            },
          },
        ]
      : []),
  ];
  ui.openMenu(event, items);
}

function openSettings(): void {
  ui.settingsOpen = true;
}

function refresh(): void {
  void files.reload();
}

function showDetails(): void {
  ui.setPrefs({ rightPanelOpen: true, rightPanelTab: "details" });
}
</script>

<template>
  <header
    class="app-toolbar flex h-10 shrink-0 items-center gap-1 border-b px-2"
    :style="{ paddingLeft: isMac ? '84px' : '8px' }"
    data-tauri-drag-region
  >
    <div class="flex items-center gap-0.5">
      <AppButton size="icon-sm" variant="ghost" icon="arrow-left" :title="$t('toolbar.back')" :disabled="!files.canGoBack" @click="files.goBack()" />
      <AppButton size="icon-sm" variant="ghost" icon="arrow-right" :title="$t('toolbar.forward')" :disabled="!files.canGoForward" @click="files.goForward()" />
      <AppButton size="icon-sm" variant="ghost" icon="arrow-up" :title="$t('toolbar.up')" :disabled="!files.canGoUp" @click="files.goUp()" />
      <AppButton size="icon-sm" variant="ghost" icon="refresh" :title="$t('toolbar.refresh')" @click="refresh" />
    </div>

    <div class="mx-1 h-4 w-px bg-border" />

    <AppButton size="sm" variant="ghost" icon="folder-plus" :title="$t('toolbar.newFolder')" :disabled="!session.signedIn" @click="dialogs.newFolder()">
      <span class="hidden md:inline">{{ $t('toolbar.newFolder') }}</span>
    </AppButton>
    <AppButton size="sm" variant="ghost" icon="upload" :title="$t('toolbar.upload')" :disabled="!session.signedIn" @click="pickAndUpload">
      <span class="hidden md:inline">{{ $t('toolbar.upload') }}</span>
    </AppButton>
    <AppButton
      size="sm"
      variant="ghost"
      icon="download"
      :title="$t('toolbar.download')"
      :disabled="!session.signedIn || !canDownload"
      @click="files.download()"
    >
      <span class="hidden md:inline">{{ $t('toolbar.download') }}</span>
    </AppButton>

    <div class="flex min-w-0 flex-1 justify-center px-3">
      <div
        class="flex h-7 w-full max-w-[420px] items-center gap-2 rounded-[6px] border border-border bg-card px-2 transition-colors"
        :class="searchFocused ? 'border-ring' : ''"
      >
        <Icon name="search" :size="14" class="text-muted-foreground" />
        <input
          :value="searchText"
          class="h-full min-w-0 flex-1 border-0 bg-transparent text-[12.5px] outline-none placeholder:text-muted-foreground"
          :placeholder="files.view === 'search' ? $t('toolbar.searchLocal') : $t('toolbar.searchPlaceholder')"
          spellcheck="false"
          @input="onSearchInput(($event.target as HTMLInputElement).value)"
          @focus="searchFocused = true"
          @blur="searchFocused = false"
          @keydown.enter.prevent="submitSearch"
          @keydown.esc.prevent="clearSearch"
          data-testid="search-input"
        />
        <button
          v-if="searchText"
          class="rounded p-0.5 text-muted-foreground hover:bg-muted"
          :aria-label="$t('actions.cancel')"
          @click="clearSearch"
        >
          <Icon name="x" :size="12" />
        </button>
      </div>
    </div>

    <AppButton
      v-if="!session.signedIn && !session.isMock"
      size="sm"
      variant="primary"
      icon="account"
      data-testid="toolbar-login"
      @click="ui.loginOpen = true"
      >{{ $t("login.signInShort") }}</AppButton
    >

    <button
      class="flex h-8 items-center gap-1.5 rounded-[6px] px-1.5 hover:bg-muted"
      :title="session.activeAccount?.email || $t('empty.notSignedIn')"
      data-testid="account-button"
      @click="openAccountMenu"
    >
      <span class="flex h-6 w-6 items-center justify-center rounded-full bg-primary text-[10px] font-semibold text-primary-foreground">
        {{ (session.activeAccount?.name || "?").slice(0, 1).toUpperCase() }}
      </span>
      <Icon name="chevron-down" :size="12" class="text-muted-foreground" />
    </button>

    <button
      class="relative flex h-8 items-center gap-1 rounded-[6px] px-2 text-[12px] hover:bg-muted"
      :class="ui.prefs.transferDockOpen ? 'bg-accent text-foreground' : 'text-muted-foreground'"
      :title="$t('toolbar.transfers')"
      data-testid="transfers-button"
      @click="ui.toggleDock()"
    >
      <Icon name="arrow-up" :size="14" />
      <span v-if="transfers.activeCount" class="badge" data-tone="accent">{{ transfers.activeCount }}</span>
    </button>

    <AppButton
      size="icon-sm"
      :variant="ui.prefs.rightPanelOpen && ui.prefs.rightPanelTab === 'details' ? 'secondary' : 'ghost'"
      icon="panel-right"
      :title="$t('toolbar.details')"
      @click="showDetails"
    />
    <AppButton size="icon-sm" variant="ghost" icon="settings" :title="$t('toolbar.settings')" data-testid="settings-button" @click="openSettings" />
  </header>
</template>
