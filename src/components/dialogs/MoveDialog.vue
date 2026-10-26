<script setup lang="ts">
import { ref } from "vue";
import AppModal from "@/components/ui/AppModal.vue";
import AppButton from "@/components/ui/AppButton.vue";
import Icon from "@/components/ui/Icon.vue";
import FolderTree from "@/components/layout/FolderTree.vue";
import { useDialogsStore } from "@/stores/dialogs";
import { useFilesStore } from "@/stores/files";

const dialogs = useDialogsStore();
const files = useFilesStore();
const target = ref<string>("root");
const targetName = ref<string>("root");
const submitting = ref(false);

function choose(folder: { id: string; name: string }): void {
  target.value = folder.id;
  targetName.value = folder.name;
}

async function submit(): Promise<void> {
  const state = dialogs.move;
  if (!state) return;
  submitting.value = true;
  const ok = await files.move(state.fileId, target.value);
  submitting.value = false;
  if (ok) dialogs.closeMove();
}
</script>

<template>
  <AppModal
    :open="dialogs.move !== null"
    :title="$t('dialog.moveTitle', { name: dialogs.move?.name ?? '' })"
    :width="460"
    @close="dialogs.closeMove()"
  >
    <div class="mb-1 text-[11.5px] text-muted-foreground">{{ $t("dialog.moveHint") }}</div>
    <div class="max-h-[46vh] overflow-auto rounded-[8px] border border-border bg-background p-1">
      <button
        class="tree-row w-full"
        :data-active="target === 'root' || undefined"
        @click="choose({ id: 'root', name: $t('nav.myDrive') })"
      >
        <span class="flex h-4 w-4 items-center justify-center text-muted-foreground"><Icon name="drive" :size="13" /></span>
        <span class="truncate">{{ $t("dialog.moveRoot") }}</span>
      </button>
      <FolderTree parent-id="root" :depth="0" select-mode @select="choose" />
    </div>
    <div class="mt-2 flex items-center gap-1.5 text-[11.5px] text-muted-foreground">
      <Icon name="folder" :size="12" />
      <span class="truncate">{{ targetName }}</span>
    </div>
    <template #footer>
      <AppButton size="sm" variant="ghost" @click="dialogs.closeMove()">{{ $t("actions.cancel") }}</AppButton>
      <AppButton size="sm" variant="primary" :disabled="submitting" @click="submit">{{ $t("dialog.move") }}</AppButton>
    </template>
  </AppModal>
</template>
