<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{ value: number; total: number | null; status?: string; height?: number }>(),
  { status: "running", height: 4 },
);

const percent = computed(() => {
  if (props.total && props.total > 0) return Math.min(100, (props.value / props.total) * 100);
  return props.status === "done" ? 100 : 0;
});
</script>

<template>
  <div class="w-full overflow-hidden rounded-full bg-muted" :style="{ height: `${height}px` }" role="progressbar" :aria-valuenow="Math.round(percent)" aria-valuemin="0" aria-valuemax="100">
    <div
      class="h-full rounded-full transition-[width] duration-150"
      :class="status === 'error' ? 'bg-destructive' : status === 'done' ? 'bg-success' : 'bg-primary'"
      :style="{ width: `${percent}%` }"
    />
  </div>
</template>
