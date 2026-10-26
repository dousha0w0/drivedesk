<script setup lang="ts">
import { onBeforeUnmount, onMounted, watch } from "vue";
import Icon from "./Icon.vue";

const props = withDefaults(
  defineProps<{ open: boolean; title?: string; width?: number; closeOnBackdrop?: boolean }>(),
  { width: 560, closeOnBackdrop: true },
);
const emit = defineEmits<{ close: [] }>();

function onKey(event: KeyboardEvent): void {
  if (event.key === "Escape" && props.open) {
    event.stopPropagation();
    emit("close");
  }
}

onMounted(() => window.addEventListener("keydown", onKey, true));
onBeforeUnmount(() => window.removeEventListener("keydown", onKey, true));

watch(
  () => props.open,
  (open) => {
    document.body.style.overflow = open ? "hidden" : "";
  },
  { immediate: true },
);
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="modal-backdrop" @mousedown.self="closeOnBackdrop && emit('close')">
      <div class="modal-card fade-in" :style="{ width: `min(${width}px, 100%)` }" role="dialog" aria-modal="true">
        <div class="modal-head">
          <span class="flex-1 truncate">{{ title }}</span>
          <button class="rounded p-1 text-muted-foreground hover:bg-muted" :aria-label="$t('actions.close')" @click="emit('close')">
            <Icon name="x" :size="15" />
          </button>
        </div>
        <div class="modal-body scroll-thin">
          <slot />
        </div>
        <div v-if="$slots.footer" class="modal-foot">
          <slot name="footer" />
        </div>
      </div>
    </div>
  </Teleport>
</template>
