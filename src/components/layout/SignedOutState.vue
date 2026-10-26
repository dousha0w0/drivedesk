<script setup lang="ts">
import Icon from "@/components/ui/Icon.vue";
import SignInPanel from "@/components/auth/SignInPanel.vue";
</script>

<template>
  <div class="flex min-h-0 flex-1 items-center justify-center overflow-auto p-8" data-testid="signed-out-state">
    <div class="w-full max-w-[540px] rounded-[var(--dr-lg)] border border-border bg-card p-5 shadow-sm">
      <div class="mb-4 flex items-center gap-2.5">
        <span class="flex h-9 w-9 items-center justify-center rounded-[8px] bg-muted text-muted-foreground">
          <Icon name="cloud" :size="19" />
        </span>
        <div>
          <div class="text-[14px] font-semibold">{{ $t("login.title") }}</div>
          <div class="text-[11.5px] text-muted-foreground">{{ $t("login.signedOutHint") }}</div>
        </div>
      </div>
      <SignInPanel />
    </div>
  </div>
</template>
