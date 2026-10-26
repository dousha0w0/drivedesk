<script setup lang="ts">
import { computed } from "vue";
import Icon from "@/components/ui/Icon.vue";
import ProgressBar from "@/components/ui/ProgressBar.vue";
import { api } from "@/lib/api";
import { eta, formatBytes, formatSpeed } from "@/lib/format";
import { useTransfersStore } from "@/stores/transfers";
import { useUiStore } from "@/stores/ui";
import { i18n } from "@/i18n";
import type { Transfer } from "@/types/drive";

const transfers = useTransfersStore();
const ui = useUiStore();
const t = (key: string, params?: Record<string, unknown>) => i18n.global.t(key, params ?? {});

const items = computed(() => transfers.items);
const statusLabel = (transfer: Transfer) =>
  transfer.status === "done"
    ? t("transfers.done")
    : transfer.status === "error"
      ? t("transfers.error")
      : transfer.status === "canceled"
        ? t("transfers.canceled")
        : transfer.status === "queued"
          ? t("transfers.queued")
          : t("transfers.running");

function progressText(transfer: Transfer): string {
  const received = formatBytes(transfer.received);
  if (transfer.total) return `${received} / ${formatBytes(transfer.total)}`;
  return received;
}

async function openFolder(transfer: Transfer): Promise<void> {
  if (!transfer.destDir) return;
  await api.systemOpenPath(transfer.destDir);
}
</script>

<template>
  <section class="relative flex shrink-0 flex-col border-t border-border bg-background" :style="{ height: `${ui.prefs.dockHeight}px` }" data-testid="transfer-dock">
    <div class="flex h-8 shrink-0 items-center gap-2 border-b border-border px-3 text-[11.5px]">
      <Icon name="arrow-up" :size="13" class="text-muted-foreground" />
      <span class="font-medium">{{ $t("transfers.title") }}</span>
      <span v-if="transfers.activeCount" class="badge" data-tone="accent">{{ transfers.activeCount }}</span>
      <span v-if="transfers.runningCount" class="text-muted-foreground">{{ formatSpeed(transfers.totalSpeed) }}</span>
      <div class="flex-1" />
      <button
        v-if="transfers.finishedCount"
        class="rounded px-1.5 py-0.5 text-muted-foreground hover:bg-muted hover:text-foreground"
        @click="transfers.clearFinished()"
      >
        {{ $t("transfers.clearFinished") }}
      </button>
      <button class="rounded p-1 text-muted-foreground hover:bg-muted" :title="$t('actions.close')" @click="ui.setPrefs({ transferDockOpen: false })">
        <Icon name="chevron-down" :size="13" />
      </button>
    </div>

    <div class="scroll-thin min-h-0 flex-1 overflow-auto px-2 py-1">
      <div v-if="!items.length" class="flex h-full flex-col items-center justify-center gap-1 text-center">
        <span class="text-[12px] text-muted-foreground">{{ $t("transfers.empty") }}</span>
        <span class="text-[11px] text-muted-foreground/80">{{ $t("transfers.emptyHint") }}</span>
      </div>

      <div
        v-for="transfer in items"
        :key="transfer.id"
        class="flex items-center gap-2 rounded px-1.5 py-1 hover:bg-muted/60"
        :data-testid="`transfer-${transfer.name}`"
      >
        <Icon :name="transfer.kind === 'upload' ? 'upload' : 'download'" :size="13" class="text-muted-foreground" />
        <div class="min-w-0 flex-1">
          <div class="flex items-baseline gap-2">
            <span class="truncate text-[11.5px]" :title="transfer.name">{{ transfer.name }}</span>
            <span v-if="transfer.expandedCount" class="badge shrink-0">{{ $t("transfers.expanded", { count: transfer.expandedCount }) }}</span>
            <span class="ml-auto shrink-0 text-[10.5px] text-muted-foreground">{{ progressText(transfer) }}</span>
            <span v-if="transfer.status === 'running' && transfer.speedBps" class="shrink-0 text-[10.5px] text-muted-foreground">
              {{ formatSpeed(transfer.speedBps) }} · {{ eta(transfer) }}
            </span>
            <span
              class="w-[52px] shrink-0 text-right text-[10.5px]"
              :class="transfer.status === 'error' ? 'text-destructive' : transfer.status === 'done' ? 'text-success' : 'text-muted-foreground'"
            >
              {{ statusLabel(transfer) }}
            </span>
          </div>
          <ProgressBar
            :value="transfer.received"
            :total="transfer.total"
            :status="transfer.status"
            :height="3"
            class="mt-1"
          />
          <div v-if="transfer.error" class="mt-0.5 truncate text-[10.5px] text-destructive" :title="transfer.error">{{ transfer.error }}</div>
        </div>
        <div class="flex shrink-0 items-center gap-0.5">
          <button
            v-if="transfer.status === 'running' || transfer.status === 'queued'"
            class="rounded p-1 text-muted-foreground hover:bg-accent"
            :title="$t('transfers.cancel')"
            @click="transfers.cancel(transfer.id)"
          >
            <Icon name="x" :size="12" />
          </button>
          <button
            v-if="transfer.status === 'error' || transfer.status === 'canceled'"
            class="rounded p-1 text-muted-foreground hover:bg-accent"
            :title="$t('transfers.retry')"
            @click="transfers.retry(transfer.id)"
          >
            <Icon name="refresh" :size="12" />
          </button>
          <button
            v-if="transfer.status === 'done' && transfer.kind === 'download'"
            class="rounded p-1 text-muted-foreground hover:bg-accent"
            :title="$t('transfers.openFolder')"
            @click="openFolder(transfer)"
          >
            <Icon name="folder-open" :size="12" />
          </button>
        </div>
      </div>
    </div>
  </section>
</template>
