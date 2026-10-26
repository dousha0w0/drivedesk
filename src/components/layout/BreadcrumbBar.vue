<script setup lang="ts">
import { computed } from "vue";
import Icon from "@/components/ui/Icon.vue";
import { useFilesStore } from "@/stores/files";
import { useUiStore } from "@/stores/ui";
import { useSessionStore } from "@/stores/session";

const files = useFilesStore();
const ui = useUiStore();
const session = useSessionStore();

const isVirtual = computed(() => files.view !== "folder");
const icon = computed(() => {
  switch (files.view) {
    case "recent":
      return "clock";
    case "starred":
      return "star";
    case "shared":
      return "share";
    case "trash":
      return "trash";
    case "search":
      return "search";
    default:
      return "drive";
  }
});

function openCrumb(id: string): void {
  void files.navigate({ view: "folder", folderId: id, query: "" });
}
</script>

<template>
  <div class="app-breadcrumb flex h-9 shrink-0 items-center gap-1 border-b px-3">
    <template v-if="isVirtual">
      <Icon :name="icon" :size="14" class="text-muted-foreground" />
      <span class="truncate text-[12.5px] font-medium">{{ files.title }}</span>
    </template>
    <template v-else>
      <template v-for="(crumb, index) in files.crumbs" :key="crumb.id">
        <Icon v-if="index > 0" name="chevron-right" :size="12" class="text-muted-foreground/60" />
        <button
          class="max-w-[220px] truncate rounded px-1.5 py-0.5 text-[12.5px] hover:bg-muted"
          :class="index === files.crumbs.length - 1 ? 'font-medium text-foreground' : 'text-muted-foreground'"
          :title="crumb.name"
          @click="openCrumb(crumb.id)"
        >
          {{ index === 0 ? $t("nav.myDrive") : crumb.name }}
        </button>
      </template>
    </template>

    <div class="flex-1" />

    <button
      v-if="files.filter"
      class="badge"
      :title="$t('actions.cancel')"
      @click="files.setFilter('')"
    >
      {{ $t("table.filterPlaceholder") }}: {{ files.filter }}
      <Icon name="x" :size="10" />
    </button>
    <button
      class="badge"
      :data-tone="ui.prefs.rightPanelTab === 'details' && ui.prefs.rightPanelOpen ? 'accent' : undefined"
      :title="$t('details.title')"
      @click="ui.toggleRightPanel('details')"
    >
      <Icon name="info" :size="11" />
      {{ $t("details.title") }}
    </button>
    <button
      class="badge"
      :data-tone="ui.prefs.rightPanelTab === 'preview' && ui.prefs.rightPanelOpen ? 'accent' : undefined"
      :title="$t('preview.title')"
      @click="ui.toggleRightPanel('preview')"
    >
      <Icon name="eye" :size="11" />
      {{ $t("preview.title") }}
    </button>
    <span v-if="session.isMock" class="badge" data-tone="warning">{{ $t("status.mock") }}</span>
    <span class="badge">{{ $t("table.items", { count: files.visibleEntries.length }) }}</span>
  </div>
</template>
