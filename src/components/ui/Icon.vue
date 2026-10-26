<script setup lang="ts">
import { computed } from "vue";
import { ICONS } from "@/lib/icons";

const props = withDefaults(
  defineProps<{ name: string; size?: number; strokeWidth?: number; filled?: boolean }>(),
  { size: 16, strokeWidth: 1.8, filled: false },
);

const path = computed(() => ICONS[props.name] ?? ICONS.file);
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    :stroke-width="strokeWidth"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    class="shrink-0"
  >
    <path :d="path" :fill="filled ? 'currentColor' : 'none'" />
  </svg>
</template>
