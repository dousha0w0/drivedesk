<script setup lang="ts">
const props = defineProps<{ modelValue: boolean; disabled?: boolean; label?: string }>();
const emit = defineEmits<{ "update:modelValue": [value: boolean] }>();
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="modelValue"
    :aria-label="label"
    :disabled="disabled"
    class="relative inline-flex h-[18px] w-[32px] shrink-0 items-center rounded-full border border-transparent transition-colors disabled:opacity-45"
    :class="modelValue ? 'bg-primary' : 'bg-input'"
    @click="emit('update:modelValue', !props.modelValue)"
  >
    <span
      class="ml-[2px] h-[14px] w-[14px] rounded-full bg-background shadow transition-transform"
      :class="modelValue ? 'translate-x-[14px]' : 'translate-x-0'"
    />
  </button>
</template>
