<script setup lang="ts">
import { computed } from "vue";
import { KIND_ICON } from "@/lib/icons";
import Icon from "@/components/ui/Icon.vue";

const props = withDefaults(defineProps<{ kind: string; size?: number }>(), { size: 16 });
const icon = computed(() => KIND_ICON[props.kind] ?? KIND_ICON.file);
</script>

<template>
  <span class="inline-flex shrink-0 items-center justify-center" :class="`kind-tint-${kind}`">
    <Icon :name="icon" :size="size" />
  </span>
</template>
