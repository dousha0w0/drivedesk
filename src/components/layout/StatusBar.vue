<script setup lang="ts">
import { computed } from "vue";
import Icon from "@/components/ui/Icon.vue";
import { formatBytes, quotaPercent } from "@/lib/format";
import { useFilesStore } from "@/stores/files";
import { useSessionStore } from "@/stores/session";
import { useTransfersStore } from "@/stores/transfers";

const files = useFilesStore();
const session = useSessionStore();
const transfers = useTransfersStore();

const storage = computed(() => {
  const percent = quotaPercent(session.quota?.usage, session.quota?.limit);
  return `${formatBytes(session.quota?.usage)} / ${formatBytes(session.quota?.limit)} (${Math.round(percent)}%)`;
});
</script>

<template>
  <footer class="flex h-[26px] shrink-0 items-center gap-3 border-t border-border bg-chrome px-3 text-[10.5px] text-muted-foreground" data-testid="status-bar">
    <span class="flex items-center gap-1">
      <Icon name="drive" :size="11" />
      <span class="max-w-[280px] truncate">{{ files.title }}</span>
    </span>
    <span>{{ $t("status.items", { count: files.visibleEntries.length }) }}</span>
    <span v-if="files.selectedCount">{{ $t("status.selected", { count: files.selectedCount }) }}</span>
    <span v-if="transfers.activeCount" class="flex items-center gap-1">
      <Icon name="spinner" :size="10" class="spin" />
      {{ $t("transfers.title") }} · {{ transfers.activeCount }}
    </span>
    <span v-if="files.error" class="truncate text-destructive" :title="files.error">{{ files.error }}</span>
    <div class="flex-1" />
    <span v-if="session.isMock" class="badge" data-tone="warning">{{ $t("status.mock") }}</span>
    <span v-if="session.gdirect?.found && session.settings?.gdirectEnabled" class="badge" data-tone="accent">{{ $t("status.gdirect") }}</span>
    <span>{{ storage }}</span>
    <span class="text-muted-foreground/60">v{{ session.info?.version ?? "0.1.0" }}</span>
  </footer>
</template>
