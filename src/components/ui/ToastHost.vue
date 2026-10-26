<script setup lang="ts">
import { useUiStore } from "@/stores/ui";
import Icon from "./Icon.vue";

const ui = useUiStore();
const ICON_BY_KIND = { info: "info", success: "check", error: "alert" } as const;
</script>

<template>
  <div class="toast-stack" aria-live="polite">
    <div
      v-for="toast in ui.toasts"
      :key="toast.id"
      class="toast"
      :data-kind="toast.kind"
      role="status"
      @click="ui.dismissToast(toast.id)"
    >
      <Icon
        :name="ICON_BY_KIND[toast.kind]"
        :size="14"
        :class="toast.kind === 'error' ? 'text-destructive' : toast.kind === 'success' ? 'text-success' : 'text-muted-foreground'"
        class="mt-0.5"
      />
      <span class="flex-1 break-words">{{ toast.message }}</span>
    </div>
  </div>
</template>
