<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { api } from "@/lib/api";
import { errorMessage } from "@/lib/errors";
import { useFilesStore } from "@/stores/files";
import FileIcon from "./FileIcon.vue";
import Icon from "@/components/ui/Icon.vue";
import type { FileMeta } from "@/types/drive";

const props = withDefaults(defineProps<{ parentId: string; depth: number; selectMode?: boolean }>(), { selectMode: false });
const emit = defineEmits<{ select: [folder: FileMeta] }>();

const files = useFilesStore();
const children = ref<FileMeta[]>([]);
const expanded = ref<string[]>([]);
const childCache = ref<Record<string, FileMeta[]>>({});
const loading = ref(false);
const error = ref<string | null>(null);

async function load(parentId = props.parentId): Promise<void> {
  loading.value = true;
  error.value = null;
  try {
    const response = await api.filesList({
      view: "folder",
      folderId: parentId,
      sortBy: "name",
      sortDir: "asc",
      pageSize: 200,
    });
    const folders = response.files.filter((entry) => entry.iconType === "folder");
    if (parentId === props.parentId) children.value = folders;
    else childCache.value = { ...childCache.value, [parentId]: folders };
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    loading.value = false;
  }
}

function foldersOf(parentId: string): FileMeta[] {
  return parentId === props.parentId ? children.value : childCache.value[parentId] ?? [];
}

async function toggle(folder: FileMeta): Promise<void> {
  const index = expanded.value.indexOf(folder.id);
  if (index !== -1) {
    expanded.value = expanded.value.filter((id) => id !== folder.id);
    return;
  }
  expanded.value = [...expanded.value, folder.id];
  if (!childCache.value[folder.id]) await load(folder.id);
}

function activate(folder: FileMeta): void {
  if (props.selectMode) {
    emit("select", folder);
    return;
  }
  void files.navigate({ view: "folder", folderId: folder.id });
}

onMounted(() => void load());
watch(
  () => props.parentId,
  () => void load(),
);
watch(
  () => files.treeVersion,
  () => {
    void load();
    for (const id of expanded.value) void load(id);
  },
);
</script>

<template>
  <div class="flex flex-col">
    <div v-if="loading && !children.length" class="px-2 py-1 text-[11px] text-muted-foreground">{{ $t("table.loading") }}</div>
    <div v-else-if="error" class="px-2 py-1 text-[11px] text-destructive">{{ error }}</div>
    <template v-for="folder in children" :key="folder.id">
      <div
        class="tree-row group"
        :data-active="!selectMode && files.view === 'folder' && files.folderId === folder.id"
        :style="{ paddingLeft: `${8 + depth * 12}px` }"
        :data-testid="`tree-${folder.name}`"
        @click="activate(folder)"
      >
        <button
          class="flex h-4 w-4 items-center justify-center rounded text-muted-foreground hover:bg-accent"
          :aria-label="$t('actions.open')"
          @click.stop="toggle(folder)"
        >
          <Icon :name="expanded.includes(folder.id) ? 'chevron-down' : 'chevron-right'" :size="12" />
        </button>
        <FileIcon kind="folder" :size="15" />
        <span class="truncate">{{ folder.name }}</span>
      </div>
      <FolderTree
        v-if="expanded.includes(folder.id)"
        :parent-id="folder.id"
        :depth="depth + 1"
        :select-mode="selectMode"
        @select="emit('select', $event)"
      />
    </template>
  </div>
</template>

<script lang="ts">
export default { name: "FolderTree" };
</script>
