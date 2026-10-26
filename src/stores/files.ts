import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { api } from "@/lib/api";
import { copyText } from "@/lib/clipboard";
import { errorMessage } from "@/lib/errors";
import { i18n } from "@/i18n";
import { matchesFilter, sortEntries, toggleSort } from "@/lib/sort";
import { applySelection, pruneSelection, toggleSelectAll } from "@/lib/selection";
import type { Crumb, FileMeta, PreviewInfo, SortDirection, SortKey, ViewKind } from "@/types/drive";
import { useSessionStore } from "@/stores/session";
import { useUiStore } from "@/stores/ui";

interface Target {
  view: ViewKind;
  folderId: string;
  query: string;
}

const ROOT: Target = { view: "folder", folderId: "root", query: "" };

export const useFilesStore = defineStore("files", () => {
  const ui = useUiStore();
  const session = useSessionStore();

  const target = ref<Target>({ ...ROOT });
  const crumbs = ref<Crumb[]>([]);
  const entries = ref<FileMeta[]>([]);
  const nextToken = ref<string | null>(null);
  const loading = ref(false);
  const loadingMore = ref(false);
  const error = ref<string | null>(null);
  const selection = ref<string[]>([]);
  const anchor = ref<string | null>(null);
  const sort = ref<{ key: SortKey; direction: SortDirection }>({ key: "name", direction: "asc" });
  const filter = ref("");
  const history = ref<Target[]>([]);
  const historyIndex = ref(-1);
  const treeVersion = ref(0);

  const previewId = ref<string | null>(null);
  const preview = ref<PreviewInfo | null>(null);
  const previewLoading = ref(false);
  const previewError = ref<string | null>(null);

  const t = (key: string, params?: Record<string, unknown>) => i18n.global.t(key, params ?? {});

  const view = computed(() => target.value.view);
  const folderId = computed(() => target.value.folderId);
  const query = computed(() => target.value.query);
  const visibleEntries = computed(() =>
    sortEntries(entries.value.filter((entry) => matchesFilter(entry, filter.value)), sort.value.key, sort.value.direction),
  );
  const orderIds = computed(() => visibleEntries.value.map((entry) => entry.id));
  const selectedFiles = computed(() => visibleEntries.value.filter((entry) => selection.value.includes(entry.id)));
  const selectedCount = computed(() => selection.value.length);
  const canGoBack = computed(() => historyIndex.value > 0);
  const canGoForward = computed(() => historyIndex.value >= 0 && historyIndex.value < history.value.length - 1);
  const canGoUp = computed(() => view.value === "folder" && crumbs.value.length > 1);
  const previewFile = computed(() => entries.value.find((entry) => entry.id === previewId.value) ?? null);
  const focusedFile = computed<FileMeta | null>(() => selectedFiles.value[0] ?? previewFile.value ?? null);
  const title = computed(() => {
    if (view.value === "folder") {
      return crumbs.value.length ? crumbs.value[crumbs.value.length - 1].name : t("nav.myDrive");
    }
    if (view.value === "search") return `${t("view.search")}${query.value ? `: ${query.value}` : ""}`;
    return t(`view.${view.value}`);
  });

  function fail(caught: unknown): false {
    ui.toast(errorMessage(caught), "error", 5200);
    return false;
  }

  function succeed(successKey: string, params?: Record<string, unknown>): true {
    ui.toast(t(successKey, params), "success", 2400);
    return true;
  }

  function pushHistory(next: Target): void {
    history.value = history.value.slice(0, historyIndex.value + 1);
    history.value.push({ ...next });
    historyIndex.value = history.value.length - 1;
  }

  function resetPreviewIfGone(ids: Set<string>): void {
    if (previewId.value && !ids.has(previewId.value)) {
      previewId.value = null;
      preview.value = null;
      previewError.value = null;
    }
  }

  async function load(next: Target, record = true): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      const response = await api.filesList({
        view: next.view,
        folderId: next.view === "folder" ? next.folderId : null,
        query: next.query || null,
        sortBy: sort.value.key,
        sortDir: sort.value.direction,
        pageSize: 300,
      });
      target.value = { ...next };
      entries.value = response.files;
      nextToken.value = response.nextPageToken;
      filter.value = "";
      const available = new Set(entries.value.map((entry) => entry.id));
      selection.value = pruneSelection(selection.value, [...available]);
      anchor.value = selection.value.length ? selection.value[selection.value.length - 1] : null;
      resetPreviewIfGone(available);
      crumbs.value =
        next.view === "folder"
          ? await api.filePath(next.folderId).catch(() => [{ id: next.folderId, name: next.folderId }])
          : [];
      if (record) pushHistory(next);
      treeVersion.value += 1;
    } catch (caught) {
      error.value = errorMessage(caught);
      entries.value = [];
      nextToken.value = null;
    } finally {
      loading.value = false;
    }
  }

  async function navigate(next: Partial<Target>): Promise<void> {
    const resolved: Target = {
      view: next.view ?? target.value.view,
      folderId: next.folderId ?? target.value.folderId,
      query: next.query ?? "",
    };
    if (resolved.view !== "folder") resolved.folderId = "root";
    await load(resolved, true);
  }

  async function reload(): Promise<void> {
    await load({ ...target.value }, false);
  }

  async function loadMore(): Promise<void> {
    if (!nextToken.value || loadingMore.value) return;
    loadingMore.value = true;
    try {
      const response = await api.filesList({
        view: view.value,
        folderId: view.value === "folder" ? folderId.value : null,
        query: query.value || null,
        sortBy: sort.value.key,
        sortDir: sort.value.direction,
        pageToken: nextToken.value,
        pageSize: 300,
      });
      const seen = new Set(entries.value.map((entry) => entry.id));
      entries.value = [...entries.value, ...response.files.filter((entry) => !seen.has(entry.id))];
      nextToken.value = response.nextPageToken;
    } catch (caught) {
      fail(caught);
    } finally {
      loadingMore.value = false;
    }
  }

  async function goBack(): Promise<void> {
    if (!canGoBack.value) return;
    historyIndex.value -= 1;
    await load({ ...history.value[historyIndex.value] }, false);
  }

  async function goForward(): Promise<void> {
    if (!canGoForward.value) return;
    historyIndex.value += 1;
    await load({ ...history.value[historyIndex.value] }, false);
  }

  async function goUp(): Promise<void> {
    if (!canGoUp.value) return;
    const parent = crumbs.value[crumbs.value.length - 2];
    await navigate({ view: "folder", folderId: parent.id });
  }

  async function setSort(key: SortKey): Promise<void> {
    sort.value = toggleSort(sort.value, key);
    await reload();
  }

  function setFilter(value: string): void {
    filter.value = value;
    selection.value = pruneSelection(selection.value, orderIds.value);
  }

  async function search(text: string): Promise<void> {
    const trimmed = text.trim();
    if (!trimmed) {
      await navigate({ view: "folder", folderId: folderId.value, query: "" });
      return;
    }
    await navigate({ view: "search", query: trimmed });
  }

  function select(id: string, modifiers: { toggle: boolean; range: boolean }): void {
    const result = applySelection(selection.value, anchor.value, orderIds.value, id, modifiers);
    selection.value = result.selected;
    anchor.value = result.anchor;
  }

  function selectAll(): void {
    selection.value = toggleSelectAll(selection.value, orderIds.value);
  }

  function clearSelection(): void {
    selection.value = [];
    anchor.value = null;
  }

  async function open(entry: FileMeta): Promise<void> {
    if (entry.iconType === "folder") {
      clearSelection();
      await navigate({ view: "folder", folderId: entry.id });
      return;
    }
    await showPreview(entry.id);
  }

  async function showPreview(id: string | null): Promise<void> {
    previewId.value = id;
    preview.value = null;
    previewError.value = null;
    if (!id) return;
    previewLoading.value = true;
    try {
      preview.value = await api.previewPrepare(id);
    } catch (caught) {
      previewError.value = errorMessage(caught);
    } finally {
      previewLoading.value = false;
    }
  }

  async function download(ids?: string[]): Promise<boolean> {
    const targets = ids ?? (selection.value.length ? selectedFiles.value.map((entry) => entry.id) : previewId.value ? [previewId.value] : []);
    if (!targets.length) return false;
    try {
      await api.downloadStart(targets);
      ui.setPrefs({ transferDockOpen: true });
      return succeed("toast.downloadStarted");
    } catch (caught) {
      return fail(caught);
    }
  }

  async function upload(paths: string[]): Promise<boolean> {
    if (!paths.length) return false;
    try {
      await api.uploadStart(folderId.value, paths);
      ui.setPrefs({ transferDockOpen: true });
      session.refresh().catch(() => {});
      return succeed("toast.uploadStarted");
    } catch (caught) {
      return fail(caught);
    }
  }

  async function createFolder(name: string): Promise<boolean> {
    try {
      await api.folderCreate(folderId.value, name);
      treeVersion.value += 1;
      await reload();
      return succeed("toast.created");
    } catch (caught) {
      return fail(caught);
    }
  }

  async function rename(id: string, name: string): Promise<boolean> {
    try {
      const updated = await api.fileRename(id, name);
      entries.value = entries.value.map((entry) => (entry.id === id ? updated : entry));
      if (previewId.value === id) preview.value = null;
      return succeed("toast.renamed");
    } catch (caught) {
      return fail(caught);
    }
  }

  async function duplicate(id: string): Promise<boolean> {
    try {
      await api.fileCopy(id, folderId.value);
      await reload();
      return succeed("toast.copied");
    } catch (caught) {
      return fail(caught);
    }
  }

  async function move(id: string, parent: string): Promise<boolean> {
    try {
      await api.fileMove(id, parent);
      treeVersion.value += 1;
      await reload();
      return succeed("toast.moved");
    } catch (caught) {
      return fail(caught);
    }
  }

  async function trash(ids: string[]): Promise<boolean> {
    try {
      for (const id of ids) await api.fileTrash(id, true);
      selection.value = [];
      anchor.value = null;
      await reload();
      session.refresh().catch(() => {});
      return succeed("toast.trashed");
    } catch (caught) {
      return fail(caught);
    }
  }

  async function restore(ids: string[]): Promise<boolean> {
    try {
      for (const id of ids) await api.fileTrash(id, false);
      selection.value = [];
      anchor.value = null;
      await reload();
      return succeed("toast.restored");
    } catch (caught) {
      return fail(caught);
    }
  }

  async function deleteForever(ids: string[]): Promise<boolean> {
    try {
      for (const id of ids) await api.fileDelete(id);
      selection.value = [];
      anchor.value = null;
      await reload();
      session.refresh().catch(() => {});
      return succeed("toast.trashed");
    } catch (caught) {
      return fail(caught);
    }
  }

  async function setStar(ids: string[], starred: boolean): Promise<boolean> {
    try {
      for (const id of ids) {
        const updated = await api.fileStar(id, starred);
        entries.value = entries.value.map((entry) => (entry.id === id ? updated : entry));
      }
      return succeed(starred ? "toast.starred" : "toast.unstarred");
    } catch (caught) {
      return fail(caught);
    }
  }

  async function copyLink(file: FileMeta): Promise<boolean> {
    const link = file.webViewLink ?? `https://drive.google.com/open?id=${file.id}`;
    if (!(await copyText(link))) return fail(t("toast.failed"));
    return succeed("toast.copiedLink");
  }

  async function copyId(file: FileMeta): Promise<boolean> {
    if (!(await copyText(file.id))) return fail(t("toast.failed"));
    return succeed("toast.copiedId");
  }

  return {
    target,
    crumbs,
    entries,
    nextToken,
    loading,
    loadingMore,
    error,
    selection,
    anchor,
    sort,
    filter,
    history,
    historyIndex,
    treeVersion,
    previewId,
    preview,
    previewLoading,
    previewError,
    view,
    folderId,
    query,
    visibleEntries,
    orderIds,
    selectedFiles,
    selectedCount,
    canGoBack,
    canGoForward,
    canGoUp,
    previewFile,
    focusedFile,
    title,
    navigate,
    reload,
    loadMore,
    goBack,
    goForward,
    goUp,
    setSort,
    setFilter,
    search,
    select,
    selectAll,
    clearSelection,
    open,
    showPreview,
    download,
    upload,
    createFolder,
    rename,
    duplicate,
    move,
    trash,
    restore,
    deleteForever,
    setStar,
    copyLink,
    copyId,
  };
});
