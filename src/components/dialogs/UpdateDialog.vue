<script setup lang="ts">
import { computed } from "vue";
import AppModal from "@/components/ui/AppModal.vue";
import AppButton from "@/components/ui/AppButton.vue";
import Icon from "@/components/ui/Icon.vue";
import ProgressBar from "@/components/ui/ProgressBar.vue";
import { formatBytes, formatDateLong } from "@/lib/format";
import { useUpdaterStore } from "@/stores/updater";

const updater = useUpdaterStore();

const released = computed(() => (updater.publishedAt ? formatDateLong(updater.publishedAt) : null));
const downloadedLabel = computed(() => {
  const progress = updater.progress;
  if (!progress) return "";
  if (!progress.total) return formatBytes(progress.downloaded);
  return `${formatBytes(progress.downloaded)} / ${formatBytes(progress.total)}`;
});
</script>

<template>
  <AppModal :open="updater.dialogOpen" :title="$t('updates.title')" :width="580" @close="updater.dialogOpen = false">
    <div class="space-y-3 pb-2">
      <div class="flex flex-wrap items-center gap-2 text-[12px]">
        <span class="badge">{{ $t("updates.currentVersion") }} {{ updater.currentVersion }}</span>
        <Icon name="chevron-right" :size="12" class="text-muted-foreground" />
        <span class="badge" data-tone="accent">{{ $t("updates.latestVersion") }} {{ updater.availableVersion ?? "—" }}</span>
        <span v-if="released" class="text-[11px] text-muted-foreground">{{ $t("updates.releasedAt") }} {{ released }}</span>
      </div>

      <div class="rounded-[8px] border border-border bg-muted/25">
        <div class="flex items-center gap-1.5 border-b border-border px-3 py-1.5 text-[11px] font-medium text-muted-foreground">
          <Icon name="list" :size="12" />
          {{ $t("updates.notes") }}
        </div>
        <pre class="scroll-thin max-h-[260px] overflow-auto whitespace-pre-wrap px-3 py-2 text-[11.5px] leading-relaxed">{{ updater.notes || $t("updates.noNotes") }}</pre>
      </div>

      <div v-if="updater.installing" class="space-y-1">
        <ProgressBar :value="updater.progress?.downloaded ?? 0" :total="updater.progress?.total ?? null" :height="5" />
        <div class="flex items-center justify-between text-[11px] text-muted-foreground">
          <span class="flex items-center gap-1.5">
            <Icon name="spinner" :size="11" class="spin" />
            {{ updater.percent >= 100 ? $t("updates.restarting") : $t("updates.installing") }}
          </span>
          <span class="tabular-nums">{{ downloadedLabel }}</span>
        </div>
      </div>

      <p v-if="updater.error" class="rounded-[6px] border border-destructive/30 bg-destructive/10 px-2.5 py-2 text-[11.5px] text-destructive">
        {{ updater.error }}
      </p>
      <p class="text-[10.5px] text-muted-foreground">{{ $t("updates.manualHint") }}</p>
    </div>

    <template #footer>
      <AppButton size="sm" variant="ghost" :disabled="updater.installing" @click="updater.ignoreVersion()">
        {{ $t("updates.ignore") }}
      </AppButton>
      <div class="flex-1" />
      <AppButton size="sm" variant="ghost" :disabled="updater.installing" @click="updater.dialogOpen = false">
        {{ $t("updates.later") }}
      </AppButton>
      <AppButton size="sm" variant="primary" icon="download" :disabled="updater.installing || !updater.update" @click="updater.install()">
        {{ $t("updates.install") }}
      </AppButton>
    </template>
  </AppModal>
</template>
