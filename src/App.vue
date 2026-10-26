<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import AppToolbar from "@/components/layout/AppToolbar.vue";
import AppSidebar from "@/components/layout/AppSidebar.vue";
import BreadcrumbBar from "@/components/layout/BreadcrumbBar.vue";
import FileTable from "@/components/layout/FileTable.vue";
import SignedOutState from "@/components/layout/SignedOutState.vue";
import PreviewPanel from "@/components/layout/PreviewPanel.vue";
import TransferDock from "@/components/layout/TransferDock.vue";
import StatusBar from "@/components/layout/StatusBar.vue";
import ContextMenu from "@/components/ui/ContextMenu.vue";
import ToastHost from "@/components/ui/ToastHost.vue";
import Icon from "@/components/ui/Icon.vue";
import LoginDialog from "@/components/dialogs/LoginDialog.vue";
import SettingsDialog from "@/components/dialogs/SettingsDialog.vue";
import PromptDialog from "@/components/dialogs/PromptDialog.vue";
import ConfirmDialog from "@/components/dialogs/ConfirmDialog.vue";
import MoveDialog from "@/components/dialogs/MoveDialog.vue";
import { api } from "@/lib/api";
import { errorMessage } from "@/lib/errors";
import { runSmoke } from "@/lib/smoke";
import { useResizeHandle } from "@/composables/useResizeHandle";
import { useDialogsStore } from "@/stores/dialogs";
import { useFilesStore } from "@/stores/files";
import { useSessionStore } from "@/stores/session";
import { useTransfersStore } from "@/stores/transfers";
import { useUiStore } from "@/stores/ui";
import { i18n } from "@/i18n";

const ui = useUiStore();
const session = useSessionStore();
const files = useFilesStore();
const transfers = useTransfersStore();
const dialogs = useDialogsStore();
const t = (key: string, params?: Record<string, unknown>) => i18n.global.t(key, params ?? {});

const sidebarWidth = ref(ui.prefs.sidebarWidth);
const detailsWidth = ref(ui.prefs.rightPanelWidth);
const dockHeight = ref(ui.prefs.dockHeight);
const dragOver = ref(false);
const signedOut = computed(() => !session.signedIn && !session.isMock);

let unlistenDrag: (() => void) | null = null;
let completionTimer: number | null = null;

watch(
  () => ui.prefs.sidebarWidth,
  (value) => (sidebarWidth.value = value),
);
watch(
  () => ui.prefs.rightPanelWidth,
  (value) => (detailsWidth.value = value),
);
watch(
  () => ui.prefs.dockHeight,
  (value) => (dockHeight.value = value),
);

const sidebarResize = useResizeHandle({
  axis: "x",
  direction: 1,
  current: () => sidebarWidth.value,
  min: 220,
  max: () => Math.min(420, window.innerWidth * 0.4),
  onChange: (value) => (sidebarWidth.value = value),
  onCommit: (value) => ui.setPanelSize("sidebarWidth", value),
});

const detailsResize = useResizeHandle({
  axis: "x",
  direction: -1,
  current: () => detailsWidth.value,
  min: 300,
  max: () => Math.min(720, window.innerWidth * 0.55),
  onChange: (value) => (detailsWidth.value = value),
  onCommit: (value) => ui.setPanelSize("rightPanelWidth", value),
});

const dockResize = useResizeHandle({
  axis: "y",
  direction: -1,
  current: () => dockHeight.value,
  min: 110,
  max: () => Math.min(360, window.innerHeight * 0.5),
  onChange: (value) => (dockHeight.value = value),
  onCommit: (value) => ui.setPanelSize("dockHeight", value),
});

// A finished transfer changes remote state (new upload, completed download);
// refresh once the queue settles instead of once per event.
watch(
  () => transfers.completionTick,
  () => {
    if (completionTimer) window.clearTimeout(completionTimer);
    completionTimer = window.setTimeout(() => {
      void files.reload();
      void session.refresh();
    }, 250);
  },
);

function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return Boolean(target.closest("input, textarea, select, [contenteditable='true']"));
}

function moveSelection(delta: number): void {
  const order = files.orderIds;
  if (!order.length) return;
  const current = files.selection[files.selection.length - 1];
  const index = current ? order.indexOf(current) : -1;
  const next = Math.min(order.length - 1, Math.max(0, index + delta));
  files.select(order[next], { toggle: false, range: false });
}

function confirmTrash(): void {
  const ids = files.selection.length ? [...files.selection] : files.previewId ? [files.previewId] : [];
  if (!ids.length) return;
  if (files.view === "trash") {
    dialogs.ask({
      title: t("dialog.deleteTitle"),
      body: t("dialog.deleteBody"),
      confirmLabel: t("dialog.delete"),
      danger: true,
      action: () => void files.deleteForever(ids),
    });
    return;
  }
  dialogs.ask({
    title: t("dialog.trashTitle"),
    body: t("dialog.trashBody", { count: ids.length }),
    confirmLabel: t("actions.trash"),
    danger: true,
    action: () => void files.trash(ids),
  });
}

function focusSearch(): void {
  const input = document.querySelector<HTMLInputElement>('[data-testid="search-input"]');
  input?.focus();
  input?.select();
}

function onKeydown(event: KeyboardEvent): void {
  const mod = event.metaKey || event.ctrlKey;
  if (mod && event.key.toLowerCase() === "a" && !isTypingTarget(event.target)) {
    event.preventDefault();
    files.selectAll();
    return;
  }
  if (mod && event.key === ",") {
    event.preventDefault();
    ui.settingsOpen = true;
    return;
  }
  if (mod && event.key.toLowerCase() === "f") {
    event.preventDefault();
    focusSearch();
    return;
  }
  if (mod && event.key.toLowerCase() === "r") {
    event.preventDefault();
    void files.reload();
    return;
  }
  if (isTypingTarget(event.target)) return;

  if (event.key === "Backspace") {
    if (event.metaKey) {
      event.preventDefault();
      confirmTrash();
      return;
    }
    if (files.canGoUp) {
      event.preventDefault();
      void files.goUp();
    }
    return;
  }
  if (event.key === "Delete") {
    event.preventDefault();
    confirmTrash();
    return;
  }

  switch (event.key) {
    case "ArrowDown":
      event.preventDefault();
      moveSelection(1);
      break;
    case "ArrowUp":
      event.preventDefault();
      moveSelection(-1);
      break;
    case "Enter": {
      const target = files.selectedFiles[0] ?? files.previewFile;
      if (target) {
        event.preventDefault();
        void files.open(target);
      }
      break;
    }
    case " ":
      if (files.selectedFiles[0]) {
        event.preventDefault();
        void files.showPreview(files.selectedFiles[0].id);
      }
      break;
    case "Escape":
      files.clearSelection();
      ui.closeMenu();
      break;
    default:
      break;
  }
}

async function registerDragAndDrop(): Promise<void> {
  try {
    const webview = getCurrentWebview();
    unlistenDrag = await webview.onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === "over" || payload.type === "enter") {
        dragOver.value = true;
        return;
      }
      if (payload.type === "leave") {
        dragOver.value = false;
        return;
      }
      dragOver.value = false;
      if (payload.type === "drop" && payload.paths?.length) {
        void files.upload(payload.paths);
      }
    });
  } catch (caught) {
    ui.toast(errorMessage(caught), "error");
  }
}

async function bootstrap(): Promise<void> {
  const log = (message: string) => void api.log("info", message);
  log("bootstrap: start");
  await session.init();
  log(`bootstrap: session (signedIn=${session.signedIn} mock=${session.isMock} builtinClient=${session.info?.builtinClient === true} error=${session.error ?? ""})`);
  await transfers.init();
  await registerDragAndDrop();

  if (session.signedIn || session.isMock) {
    await files.navigate({ view: "folder", folderId: "root", query: "" });
  }

  await api.appReady();
  log("bootstrap: window ready");

  const smoke = await api.smokeConfig().catch(() => null);
  log(`bootstrap: smoke=${smoke?.enabled ?? false}`);
  if (smoke?.enabled) {
    await runSmoke();
  }
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
  void bootstrap();
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
  unlistenDrag?.();
  transfers.dispose();
  if (completionTimer) window.clearTimeout(completionTimer);
});
</script>

<template>
  <div class="app-root">
    <div v-if="session.loading" class="flex flex-1 items-center justify-center gap-2 text-[13px] text-muted-foreground">
      <Icon name="spinner" :size="16" class="spin" />
      {{ $t("table.loading") }}
    </div>

    <template v-else>
      <AppToolbar />

      <div class="app-body" :data-layout="ui.prefs.layout">
        <div class="app-sidebar app-pane relative" :style="{ width: `${sidebarWidth}px` }">
          <AppSidebar />
          <div class="resize-handle resize-handle--right" @pointerdown="sidebarResize.onPointerDown" />
        </div>

        <main class="app-workspace app-pane">
          <SignedOutState v-if="signedOut" />
          <template v-else>
            <BreadcrumbBar />
            <FileTable />
            <div v-if="ui.prefs.transferDockOpen" class="relative shrink-0">
              <div class="resize-handle resize-handle--row resize-handle--row-top" @pointerdown="dockResize.onPointerDown" />
              <TransferDock />
            </div>
          </template>
        </main>

        <aside v-if="ui.prefs.rightPanelOpen && !signedOut" class="app-details app-pane relative" :style="{ width: `${detailsWidth}px` }">
          <div class="resize-handle resize-handle--left" @pointerdown="detailsResize.onPointerDown" />
          <PreviewPanel />
        </aside>
      </div>

      <StatusBar />

      <ContextMenu />
      <ToastHost />
      <LoginDialog />
      <SettingsDialog />
      <PromptDialog />
      <ConfirmDialog />
      <MoveDialog />

      <div v-if="dragOver" class="drag-overlay">{{ $t("transfers.upload") }}</div>
    </template>
  </div>
</template>
