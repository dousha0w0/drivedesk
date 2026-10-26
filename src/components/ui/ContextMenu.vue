<script setup lang="ts">
import { onBeforeUnmount, onMounted } from "vue";
import { useUiStore } from "@/stores/ui";
import Icon from "./Icon.vue";

const ui = useUiStore();

function close(): void {
  ui.closeMenu();
}

function onPointerDown(event: MouseEvent): void {
  if (!ui.menu) return;
  const target = event.target;
  if (target instanceof Element && target.closest(".menu-surface")) return;
  close();
}

function onKey(event: KeyboardEvent): void {
  if (event.key === "Escape" && ui.menu) {
    event.stopPropagation();
    close();
  }
}

function run(action?: () => void): void {
  close();
  action?.();
}

onMounted(() => {
  window.addEventListener("mousedown", onPointerDown, true);
  window.addEventListener("resize", close);
  window.addEventListener("keydown", onKey, true);
});
onBeforeUnmount(() => {
  window.removeEventListener("mousedown", onPointerDown, true);
  window.removeEventListener("resize", close);
  window.removeEventListener("keydown", onKey, true);
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="ui.menu"
      class="menu-surface fade-in"
      role="menu"
      :style="{ left: `${ui.menu.x}px`, top: `${ui.menu.y}px` }"
      @contextmenu.prevent
    >
      <template v-for="(item, index) in ui.menu.items" :key="`${item.label}-${index}`">
        <div v-if="item.separator" class="menu-separator" role="separator" />
        <div
          v-else
          class="menu-item"
          role="menuitem"
          :data-danger="item.danger || undefined"
          :data-disabled="item.disabled || undefined"
          @click="run(item.action)"
        >
          <Icon v-if="item.icon" :name="item.icon" :size="14" class="text-muted-foreground" />
          <span class="flex-1">{{ item.label }}</span>
          <span v-if="item.shortcut" class="menu-shortcut">{{ item.shortcut }}</span>
        </div>
      </template>
    </div>
  </Teleport>
</template>
