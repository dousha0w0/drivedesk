<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import FileIcon from "./FileIcon.vue";
import Icon from "@/components/ui/Icon.vue";
import { formatBytes, formatDate } from "@/lib/format";
import { useDialogsStore } from "@/stores/dialogs";
import { useFilesStore } from "@/stores/files";
import { useUiStore, type MenuItem } from "@/stores/ui";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { i18n } from "@/i18n";
import type { FileMeta, SortKey } from "@/types/drive";

const files = useFilesStore();
const ui = useUiStore();
const dialogs = useDialogsStore();
const t = (key: string, params?: Record<string, unknown>) => i18n.global.t(key, params ?? {});

const viewport = ref<HTMLElement | null>(null);
const scrollTop = ref(0);
const viewportHeight = ref(600);
const rowHeight = ref(30);
const OVERSCAN = 6;
let observer: ResizeObserver | null = null;

const COLUMNS: { key: SortKey; label: string; className: string; align: "left" | "right" }[] = [
  { key: "name", label: "table.name", className: "file-col-name", align: "left" },
  { key: "size", label: "table.size", className: "file-col-size", align: "right" },
  { key: "modified", label: "table.modified", className: "file-col-modified", align: "left" },
  { key: "type", label: "table.type", className: "file-col-type", align: "left" },
];

const range = computed(() => {
  const height = rowHeight.value > 0 ? rowHeight.value : 30;
  const first = Math.max(0, Math.floor(scrollTop.value / height) - OVERSCAN);
  const visibleCount = Math.ceil(viewportHeight.value / height) + OVERSCAN * 2;
  return { first, last: Math.min(files.visibleEntries.length, first + visibleCount) };
});

const windowEntries = computed(() =>
  files.visibleEntries.slice(range.value.first, range.value.last).map((file, offset) => ({
    file,
    index: range.value.first + offset,
  })),
);

function readRowHeight(): void {
  if (!viewport.value) return;
  const value = getComputedStyle(viewport.value).getPropertyValue("--dr-row").trim();
  const parsed = Number.parseFloat(value);
  if (Number.isFinite(parsed) && parsed > 0) rowHeight.value = parsed;
}

function onScroll(): void {
  scrollTop.value = viewport.value?.scrollTop ?? 0;
}

function scrollToId(id: string): void {
  const index = files.visibleEntries.findIndex((entry) => entry.id === id);
  if (index === -1 || !viewport.value) return;
  const top = index * rowHeight.value;
  const bottom = top + rowHeight.value;
  if (top < viewport.value.scrollTop) viewport.value.scrollTop = top;
  else if (bottom > viewport.value.scrollTop + viewportHeight.value) {
    viewport.value.scrollTop = bottom - viewportHeight.value;
  }
}

function onRowClick(event: MouseEvent, file: FileMeta): void {
  files.select(file.id, { toggle: event.metaKey || event.ctrlKey, range: event.shiftKey });
}

function onRowDoubleClick(file: FileMeta): void {
  void files.open(file);
}

function onHeaderClick(key: SortKey): void {
  void files.setSort(key);
}

function contextActions(file: FileMeta): MenuItem[] {
  const inTrash = files.view === "trash";
  const many = files.selectedCount > 1;
  const items: MenuItem[] = [];

  if (inTrash) {
    items.push({
      label: t("actions.restore"),
      icon: "restore",
      action: () => void files.restore(files.selection.length ? files.selection : [file.id]),
    });
    items.push({
      label: t("actions.deleteForever"),
      icon: "trash",
      danger: true,
      action: () =>
        dialogs.ask({
          title: t("dialog.deleteTitle"),
          body: t("dialog.deleteBody"),
          confirmLabel: t("dialog.delete"),
          danger: true,
          action: () => void files.deleteForever(files.selection.length ? files.selection : [file.id]),
        }),
    });
    return items;
  }

  if (file.iconType === "folder") {
    items.push({ label: t("actions.open"), icon: "folder-open", action: () => void files.open(file) });
  } else {
    items.push({ label: t("actions.preview"), icon: "eye", action: () => void files.showPreview(file.id) });
  }
  items.push({ label: t("actions.download"), icon: "download", action: () => void files.download([file.id]) });
  items.push({ separator: true, label: "" });
  items.push({ label: t("actions.rename"), icon: "file", disabled: many, action: () => dialogs.rename(file) });
  items.push({ label: t("actions.copy"), icon: "copy", disabled: many, action: () => void files.duplicate(file.id) });
  items.push({ label: t("actions.move"), icon: "move", disabled: many, action: () => dialogs.moveTo(file) });
  items.push({
    label: file.starred ? t("actions.unstar") : t("actions.star"),
    icon: "star",
    action: () => void files.setStar(files.selection.length ? files.selection : [file.id], !file.starred),
  });
  items.push({ label: t("actions.shareLink"), icon: "link", disabled: many, action: () => void files.copyLink(file) });
  items.push({ label: t("actions.copyId"), icon: "copy", disabled: many, action: () => void files.copyId(file) });
  items.push({ separator: true, label: "" });
  items.push({ label: t("actions.details"), icon: "info", action: () => ui.setPrefs({ rightPanelOpen: true, rightPanelTab: "details" }) });
  items.push({
    label: t("actions.trash"),
    icon: "trash",
    danger: true,
    action: () =>
      dialogs.ask({
        title: t("dialog.trashTitle"),
        body: t("dialog.trashBody", { count: files.selection.length || 1 }),
        confirmLabel: t("actions.trash"),
        danger: true,
        action: () => void files.trash(files.selection.length ? files.selection : [file.id]),
      }),
  });
  return items;
}

function onContextMenu(event: MouseEvent, file: FileMeta): void {
  if (!files.selection.includes(file.id)) {
    files.select(file.id, { toggle: false, range: false });
  }
  ui.openMenu(event, contextActions(file));
}

function onViewportContextMenu(event: MouseEvent): void {
  if ((event.target as HTMLElement).closest(".file-table__row")) return;
  ui.openMenu(event, [
    { label: t("actions.selectAll"), shortcut: "⌘A", action: () => files.selectAll() },
    { label: t("toolbar.newFolder"), icon: "folder-plus", action: () => dialogs.newFolder() },
    {
      label: t("toolbar.upload"),
      icon: "upload",
      action: async () => {
        const picked = await openDialog({ multiple: true });
        if (!picked) return;
        const paths = Array.isArray(picked) ? picked : [picked];
        await files.upload(paths.filter((path): path is string => typeof path === "string"));
      },
    },
    { label: t("toolbar.refresh"), icon: "refresh", action: () => void files.reload() },
  ]);
}

function emptyText(): string {
  if (files.view === "trash") return t("table.emptyTrash");
  if (files.view === "search") return t("table.emptySearch");
  return t("table.emptyFolder");
}

onMounted(() => {
  readRowHeight();
  viewportHeight.value = viewport.value?.clientHeight ?? 600;
  observer = new ResizeObserver(() => {
    viewportHeight.value = viewport.value?.clientHeight ?? 600;
  });
  if (viewport.value) observer.observe(viewport.value);
});

onBeforeUnmount(() => observer?.disconnect());

watch(
  () => ui.prefs.density,
  async () => {
    await nextTick();
    readRowHeight();
  },
);

watch(
  () => files.selection[files.selection.length - 1],
  (id) => {
    if (id) scrollToId(id);
  },
);
</script>

<template>
  <div class="file-table" :class="`row-density-${ui.prefs.density}`" data-testid="file-table">
    <div class="file-table__head file-grid">
      <button
        v-for="column in COLUMNS"
        :key="column.key"
        class="file-table__head-cell"
        :class="[column.className, column.align === 'right' ? 'justify-end' : '']"
        data-sortable="true"
        @click="onHeaderClick(column.key)"
      >
        <span>{{ $t(column.label) }}</span>
        <Icon
          v-if="files.sort.key === column.key"
          :name="files.sort.direction === 'asc' ? 'arrow-up' : 'arrow-down'"
          :size="11"
        />
      </button>
    </div>

    <div ref="viewport" class="file-table__viewport scroll-thin" @scroll.passive="onScroll" @contextmenu.prevent="onViewportContextMenu">
      <div v-if="files.loading" class="flex h-full items-center justify-center gap-2 text-[12px] text-muted-foreground">
        <Icon name="spinner" :size="14" class="spin" />
        {{ $t("table.loading") }}
      </div>

      <div v-else-if="!files.visibleEntries.length" class="flex h-full flex-col items-center justify-center gap-2 px-8 text-center">
        <FileIcon :kind="files.view === 'trash' ? 'archive' : files.view === 'search' ? 'file' : 'folder'" :size="30" />
        <div class="text-[13px] font-medium">{{ emptyText() }}</div>
        <div class="max-w-[380px] text-[11.5px] leading-relaxed text-muted-foreground">
          {{ files.view === "search" ? $t("table.emptySearchHint") : $t("table.emptyHint") }}
        </div>
      </div>

      <div v-else class="file-table__spacer" :style="{ height: `${files.visibleEntries.length * rowHeight}px` }">
        <div
          v-for="item in windowEntries"
          :key="item.file.id"
          class="file-table__row file-grid"
          role="row"
          :aria-selected="files.selection.includes(item.file.id)"
          :data-selected="files.selection.includes(item.file.id) || undefined"
          :data-testid="`row-${item.file.name}`"
          :style="{ transform: `translateY(${item.index * rowHeight}px)` }"
          @click="onRowClick($event, item.file)"
          @dblclick="onRowDoubleClick(item.file)"
          @contextmenu.prevent.stop="onContextMenu($event, item.file)"
        >
          <div class="file-table__cell file-table__name file-col-name">
            <FileIcon :kind="item.file.iconType" :size="16" />
            <span class="truncate">{{ item.file.name }}</span>
            <Icon v-if="item.file.starred" name="star" :size="12" filled class="text-warning" />
            <span v-if="item.file.shared" class="badge shrink-0">{{ $t("details.shared") }}</span>
          </div>
          <div class="file-table__cell file-table__cell--num file-col-size">
            {{ item.file.iconType === "folder" ? "—" : formatBytes(item.file.size) }}
          </div>
          <div class="file-table__cell file-table__cell--muted file-col-modified">{{ formatDate(item.file.modifiedTime) }}</div>
          <div class="file-table__cell file-table__cell--muted file-col-type">
            {{ $t(`kind.${item.file.iconType}`) }}
          </div>
        </div>
      </div>

      <div v-if="files.nextToken" class="sticky bottom-0 flex justify-center border-t border-border bg-background/95 py-1.5">
        <button class="text-[11.5px] text-muted-foreground hover:text-foreground" :disabled="files.loadingMore" @click="files.loadMore()">
          {{ files.loadingMore ? $t("table.loading") : $t("table.loadMore") }}
        </button>
      </div>
    </div>
  </div>
</template>
