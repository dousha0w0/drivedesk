<script setup lang="ts">
import { ref, watch } from "vue";
import AppModal from "@/components/ui/AppModal.vue";
import AppButton from "@/components/ui/AppButton.vue";
import { useDialogsStore } from "@/stores/dialogs";
import { useFilesStore } from "@/stores/files";
import { i18n } from "@/i18n";

const dialogs = useDialogsStore();
const files = useFilesStore();
const t = (key: string) => i18n.global.t(key);
const value = ref("");
const error = ref<string | null>(null);

watch(
  () => dialogs.prompt,
  (state) => {
    value.value = state?.value ?? "";
    error.value = null;
  },
);

async function submit(): Promise<void> {
  const state = dialogs.prompt;
  if (!state) return;
  const name = value.value.trim();
  if (!name) {
    error.value = t("dialog.emptyName");
    return;
  }
  const ok =
    state.mode === "new-folder"
      ? await files.createFolder(name)
      : state.targetId
        ? await files.rename(state.targetId, name)
        : false;
  if (ok) dialogs.closePrompt();
}
</script>

<template>
  <AppModal
    :open="dialogs.prompt !== null"
    :title="dialogs.prompt ? $t(dialogs.prompt.title) : ''"
    :width="420"
    @close="dialogs.closePrompt()"
  >
    <label class="mb-1 block text-[11.5px] text-muted-foreground">{{ dialogs.prompt ? $t(dialogs.prompt.label) : "" }}</label>
    <input
      v-model="value"
      class="h-8 w-full rounded-[6px] border border-input bg-background px-2 text-[12.5px] outline-none focus:border-ring"
      autofocus
      data-testid="prompt-input"
      @keydown.enter.prevent="submit"
    />
    <div v-if="error" class="mt-1.5 text-[11.5px] text-destructive">{{ error }}</div>
    <template #footer>
      <AppButton size="sm" variant="ghost" @click="dialogs.closePrompt()">{{ $t("actions.cancel") }}</AppButton>
      <AppButton size="sm" variant="primary" data-testid="prompt-submit" @click="submit">
        {{ dialogs.prompt?.mode === "rename" ? $t("dialog.rename") : $t("dialog.create") }}
      </AppButton>
    </template>
  </AppModal>
</template>
