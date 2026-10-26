<script setup lang="ts">
import { computed } from "vue";
import Icon from "./Icon.vue";

const props = withDefaults(
  defineProps<{
    variant?: "primary" | "secondary" | "ghost" | "danger";
    size?: "xs" | "sm" | "md" | "icon" | "icon-sm";
    disabled?: boolean;
    icon?: string;
    type?: "button" | "submit";
    title?: string;
  }>(),
  { variant: "secondary", size: "sm", disabled: false, type: "button" },
);

const emit = defineEmits<{ click: [event: MouseEvent] }>();

const VARIANTS = {
  primary: "bg-primary text-primary-foreground hover:opacity-90 active:opacity-80 border border-transparent",
  secondary: "bg-card text-foreground border border-border hover:bg-muted active:bg-accent",
  ghost: "bg-transparent text-foreground border border-transparent hover:bg-muted active:bg-accent",
  danger: "bg-destructive/10 text-destructive border border-destructive/25 hover:bg-destructive/20",
} as const;

const SIZES = {
  xs: "h-6 gap-1 px-1.5 text-[11px] rounded-[5px]",
  sm: "h-7 gap-1.5 px-2 text-[12px] rounded-[6px]",
  md: "h-8 gap-1.5 px-2.5 text-[12.5px] rounded-[7px]",
  icon: "h-8 w-8 justify-center rounded-[6px]",
  "icon-sm": "h-7 w-7 justify-center rounded-[6px]",
} as const;

const classes = computed(() => [VARIANTS[props.variant], SIZES[props.size]]);
</script>

<template>
  <button
    :type="type"
    :disabled="disabled"
    :title="title"
    :class="['inline-flex select-none items-center whitespace-nowrap transition-colors disabled:pointer-events-none disabled:opacity-45', classes]"
    @click="emit('click', $event)"
  >
    <Icon v-if="icon" :name="icon" :size="size === 'xs' ? 12 : 14" />
    <slot />
  </button>
</template>
