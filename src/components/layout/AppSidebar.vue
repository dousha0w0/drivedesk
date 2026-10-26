<script setup lang="ts">
import { computed } from "vue";
import FolderTree from "./FolderTree.vue";
import Icon from "@/components/ui/Icon.vue";
import { formatBytes, quotaPercent } from "@/lib/format";
import { useFilesStore } from "@/stores/files";
import { useSessionStore } from "@/stores/session";
import { useUiStore } from "@/stores/ui";
import type { ViewKind } from "@/types/drive";
import { i18n } from "@/i18n";

const t = (key: string) => i18n.global.t(key);

const files = useFilesStore();
const session = useSessionStore();
const ui = useUiStore();

const NAV: { view: ViewKind; icon: string; label: string }[] = [
  { view: "folder", icon: "drive", label: "nav.myDrive" },
  { view: "recent", icon: "clock", label: "nav.recent" },
  { view: "starred", icon: "star", label: "nav.starred" },
  { view: "shared", icon: "share", label: "nav.shared" },
  { view: "trash", icon: "trash", label: "nav.trash" },
];

const storagePercent = computed(() => quotaPercent(session.quota?.usage, session.quota?.limit));

function go(view: ViewKind): void {
  void files.navigate({ view, folderId: "root", query: "" });
}

function isActive(view: ViewKind): boolean {
  return view === "folder" ? files.view === "folder" : files.view === view;
}

function openAccountMenu(event: MouseEvent): void {
  ui.openMenu(event, [
    ...session.accounts.map((account) => ({
      label: account.email,
      icon: account.isDefault ? "check" : "account",
      action: () => {
        if (account.isDefault) return;
        void session.switchTo(account.email).then(() => files.navigate({ view: "folder", folderId: "root", query: "" }));
      },
    })),
    { separator: true, label: "" },
    { label: t("nav.addAccount"), icon: "plus", action: () => (ui.loginOpen = true) },
    ...(session.activeAccount
      ? [
          {
            label: t("nav.signOut"),
            icon: "logout",
            danger: true,
            action: () => void session.signOut(session.activeAccount!.email),
          },
        ]
      : []),
  ]);
}
</script>

<template>
  <aside class="app-sidebar app-pane select-none" data-testid="sidebar">
    <div class="app-sidebar-toolbar flex items-center gap-2 border-b px-2.5 pb-1.5 pt-2">
      <span class="flex h-6 w-6 items-center justify-center rounded-full bg-primary text-[10px] font-semibold text-primary-foreground">
        {{ (session.activeAccount?.name || "D").slice(0, 1).toUpperCase() }}
      </span>
      <div class="min-w-0 flex-1">
        <div class="truncate text-[12px] font-medium text-foreground">{{ session.activeAccount?.name || $t("empty.notSignedIn") }}</div>
        <div class="truncate text-[10.5px] text-muted-foreground">{{ session.activeAccount?.email || "—" }}</div>
      </div>
      <button class="rounded p-1 text-muted-foreground hover:bg-muted" :aria-label="$t('nav.accounts')" @click="openAccountMenu">
        <Icon name="chevron-down" :size="13" />
      </button>
    </div>

    <nav class="flex flex-col gap-0.5 px-2 pb-1">
      <button
        v-for="item in NAV"
        :key="item.label"
        class="nav-row text-[12.5px]"
        :data-active="isActive(item.view) || undefined"
        :data-testid="`nav-${item.view}`"
        @click="go(item.view)"
      >
        <Icon :name="item.icon" :size="15" class="text-muted-foreground" />
        <span class="flex-1 text-left">{{ $t(item.label) }}</span>
        <span v-if="item.view === 'trash' && files.view === 'trash'" class="text-[10px] text-muted-foreground">{{ files.entries.length }}</span>
      </button>
    </nav>

    <div class="mx-2 my-1 h-px bg-sidebar-border" />

    <div class="flex items-center justify-between px-3 pb-1 text-[10.5px] font-medium uppercase tracking-wide text-muted-foreground">
      <span>{{ $t("nav.myDrive") }}</span>
      <button class="rounded p-0.5 hover:bg-muted" :aria-label="$t('toolbar.refresh')" @click="files.reload()">
        <Icon name="refresh" :size="12" />
      </button>
    </div>

    <div class="scroll-thin min-h-0 flex-1 overflow-auto px-2 pb-2">
      <FolderTree parent-id="root" :depth="0" />
      <div v-if="!session.signedIn" class="px-2 py-3 text-[11.5px] text-muted-foreground">{{ $t("empty.notSignedIn") }}</div>
    </div>

    <div class="border-t px-2.5 py-2" :style="{ borderColor: 'var(--sidebar-border)' }">
      <div class="mb-1 flex items-center justify-between text-[10.5px] text-muted-foreground">
        <span class="inline-flex items-center gap-1"><Icon name="cloud" :size="12" />{{ $t("nav.storage") }}</span>
        <span>{{ formatBytes(session.quota?.usage) }} / {{ formatBytes(session.quota?.limit) }}</span>
      </div>
      <div class="h-1 overflow-hidden rounded-full bg-sidebar-border">
        <div class="h-full rounded-full bg-primary transition-[width] duration-300" :style="{ width: `${storagePercent}%` }" />
      </div>
      <div class="mt-1.5 flex flex-wrap items-center gap-1">
        <span v-if="session.isMock" class="badge" data-tone="warning">{{ $t("status.mock") }}</span>
        <span v-if="session.gdirect?.found && session.settings?.gdirectEnabled" class="badge">{{ $t("status.gdirect") }}</span>
        <span v-if="session.error" class="badge" data-tone="warning">{{ session.error }}</span>
      </div>
    </div>
  </aside>
</template>
