<script setup lang="ts">
import { ref } from "vue";
import AppModal from "@/components/ui/AppModal.vue";
import AppButton from "@/components/ui/AppButton.vue";
import { useDialogsStore } from "@/stores/dialogs";

const dialogs = useDialogsStore();
const working = ref(false);

async function confirm(): Promise<void> {
  const state = dialogs.confirm;
  if (!state) return;
  working.value = true;
  try {
    await state.action();
    dialogs.closeConfirm();
  } finally {
    working.value = false;
  }
}
</script>

<template>
  <AppModal :open="dialogs.confirm !== null" :title="dialogs.confirm?.title ?? ''" :width="440" @close="dialogs.closeConfirm()">
    <p class="pb-2 text-[12.5px] leading-relaxed text-muted-foreground">{{ dialogs.confirm?.body }}</p>
    <template #footer>
      <AppButton size="sm" variant="ghost" @click="dialogs.closeConfirm()">{{ $t("actions.cancel") }}</AppButton>
      <AppButton
        size="sm"
        :variant="dialogs.confirm?.danger ? 'danger' : 'primary'"
        :disabled="working"
        data-testid="confirm-submit"
        @click="confirm"
      >
        {{ dialogs.confirm?.confirmLabel }}
      </AppButton>
    </template>
  </AppModal>
</template>
