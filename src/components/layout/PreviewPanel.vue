<script setup lang="ts">
import { computed, ref, watch } from "vue";
import Icon from "@/components/ui/Icon.vue";
import AppButton from "@/components/ui/AppButton.vue";
import FileIcon from "./FileIcon.vue";
import { api, assetUrl } from "@/lib/api";
import { copyText } from "@/lib/clipboard";
import { formatBytes, formatDateLong } from "@/lib/format";
import { errorMessage } from "@/lib/errors";
import { useFilesStore } from "@/stores/files";
import { useUiStore } from "@/stores/ui";
import { i18n } from "@/i18n";
import type { FileMeta } from "@/types/drive";

const files = useFilesStore();
const ui = useUiStore();
const t = (key: string, params?: Record<string, unknown>) => i18n.global.t(key, params ?? {});
const openError = ref<string | null>(null);

const file = computed<FileMeta | null>(() => files.focusedFile);
const preview = computed(() => files.preview);
const url = computed(() => (files.preview?.path ? assetUrl(files.preview.path) : null));
const location = computed(() => files.crumbs.map((crumb) => crumb.name).join(" / "));

watch(
  () => files.previewId,
  () => {
    openError.value = null;
  },
);

async function openExternal(): Promise<void> {
  if (!files.preview?.path) return;
  try {
    await api.systemOpenPath(files.preview.path);
  } catch (caught) {
    openError.value = errorMessage(caught);
  }
}

async function openInBrowser(): Promise<void> {
  if (!file.value?.webViewLink) return;
  try {
    await api.systemOpenUrl(file.value.webViewLink);
  } catch (caught) {
    openError.value = errorMessage(caught);
  }
}

async function copy(value: string | null | undefined, toastKey: string): Promise<void> {
  if (!value) return;
  const copied = await copyText(value);
  ui.toast(copied ? t(toastKey) : t("toast.failed"), copied ? "success" : "error", 2000);
}

function tab(next: "preview" | "details"): void {
  ui.setPrefs({ rightPanelTab: next, rightPanelOpen: true });
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="flex h-9 shrink-0 items-center gap-1 border-b border-border px-2">
      <button
        class="h-6 rounded px-2 text-[11.5px]"
        :class="ui.prefs.rightPanelTab === 'preview' ? 'bg-accent font-medium text-foreground' : 'text-muted-foreground hover:bg-muted'"
        @click="tab('preview')"
      >
        {{ $t("preview.title") }}
      </button>
      <button
        class="h-6 rounded px-2 text-[11.5px]"
        :class="ui.prefs.rightPanelTab === 'details' ? 'bg-accent font-medium text-foreground' : 'text-muted-foreground hover:bg-muted'"
        @click="tab('details')"
      >
        {{ $t("details.title") }}
      </button>
      <div class="flex-1" />
      <AppButton
        v-if="file && ui.prefs.rightPanelTab === 'details'"
        size="icon-sm"
        variant="ghost"
        icon="external"
        :title="$t('actions.openInDrive')"
        :disabled="!file.webViewLink"
        @click="openInBrowser"
      />
      <AppButton size="icon-sm" variant="ghost" icon="x" :title="$t('actions.close')" @click="ui.setPrefs({ rightPanelOpen: false })" />
    </div>

    <template v-if="ui.prefs.rightPanelTab === 'preview'">
      <div v-if="!file && !files.previewLoading" class="flex flex-1 flex-col items-center justify-center gap-2 px-6 text-center">
        <Icon name="eye" :size="26" class="text-muted-foreground/70" />
        <div class="text-[12.5px] font-medium">{{ $t("preview.emptyTitle") }}</div>
        <div class="text-[11.5px] leading-relaxed text-muted-foreground">{{ $t("preview.emptyHint") }}</div>
      </div>

      <div v-else-if="files.previewLoading" class="flex flex-1 items-center justify-center gap-2 text-[12px] text-muted-foreground">
        <Icon name="spinner" :size="14" class="spin" />
        {{ $t("preview.loading") }}
      </div>

      <div v-else-if="files.previewError" class="flex flex-1 flex-col items-center justify-center gap-2 px-6 text-center">
        <Icon name="alert" :size="22" class="text-destructive" />
        <div class="text-[12.5px] font-medium">{{ $t("preview.failed") }}</div>
        <div class="max-w-[320px] break-words text-[11.5px] text-muted-foreground">{{ files.previewError }}</div>
        <AppButton size="sm" variant="secondary" icon="refresh" @click="files.showPreview(files.previewId)">{{ $t("actions.retry") }}</AppButton>
      </div>

      <template v-else-if="preview">
        <div v-if="preview.kind === 'text'" class="preview-scroll scroll-thin">
          <div v-if="preview.truncated" class="sticky top-0 flex items-center gap-1 border-b border-border bg-muted px-3 py-1 text-[11px] text-muted-foreground">
            <Icon name="info" :size="11" />{{ $t("preview.truncated") }}
          </div>
          <pre class="preview-text">{{ preview.text }}</pre>
        </div>

        <div v-else-if="preview.kind === 'image' && url" class="preview-scroll scroll-thin">
          <div class="preview-image"><img :src="url" :alt="file?.name" /></div>
        </div>

        <iframe v-else-if="preview.kind === 'pdf' && url" class="preview-frame" :src="url" :title="file?.name" />

        <div v-else-if="preview.kind === 'video' && url" class="flex flex-1 items-center justify-center bg-black/85 p-2">
          <video :src="url" controls class="max-h-full max-w-full" />
        </div>

        <div v-else-if="preview.kind === 'audio' && url" class="flex flex-1 flex-col items-center justify-center gap-3 p-6">
          <Icon name="audio" :size="34" class="text-muted-foreground" />
          <audio :src="url" controls class="w-full max-w-[320px]" />
        </div>

        <div v-else class="flex flex-1 flex-col items-center justify-center gap-2 px-6 text-center">
          <FileIcon :kind="file?.iconType ?? 'file'" :size="30" />
          <div class="max-w-[320px] text-[11.5px] leading-relaxed text-muted-foreground">
            {{ preview.message || (preview.kind === "office" ? $t("preview.office") : $t("preview.failed")) }}
          </div>
          <div class="flex gap-2">
            <AppButton v-if="preview.path" size="sm" variant="secondary" icon="external" @click="openExternal">{{ $t("preview.openExternal") }}</AppButton>
            <AppButton v-if="file?.webViewLink" size="sm" variant="ghost" icon="link" @click="openInBrowser">{{ $t("actions.openInDrive") }}</AppButton>
          </div>
          <div v-if="openError" class="text-[11px] text-destructive">{{ openError }}</div>
        </div>

        <div v-if="file" class="flex shrink-0 items-center gap-2 border-t border-border px-3 py-1.5 text-[11px] text-muted-foreground">
          <span class="truncate" :title="file.name">{{ file.name }}</span>
          <div class="flex-1" />
          <span>{{ formatBytes(file.size) }}</span>
          <button class="rounded px-1 hover:bg-muted" @click="files.download([file.id])"><Icon name="download" :size="12" /></button>
        </div>
      </template>
    </template>

    <div v-else class="preview-scroll scroll-thin px-3 py-2">
      <div v-if="!file" class="flex h-full flex-col items-center justify-center gap-2 text-center text-[11.5px] text-muted-foreground">
        <Icon name="info" :size="22" class="text-muted-foreground/70" />
        {{ $t("details.notSelected") }}
      </div>
      <template v-else>
        <div class="mb-3 flex items-center gap-2">
          <FileIcon :kind="file.iconType" :size="26" />
          <div class="min-w-0">
            <div class="truncate text-[12.5px] font-medium" :title="file.name">{{ file.name }}</div>
            <div class="text-[11px] text-muted-foreground">{{ $t(`kind.${file.iconType}`) }}</div>
          </div>
        </div>

        <dl class="details-grid">
          <dt>{{ $t("details.size") }}</dt>
          <dd>{{ file.iconType === "folder" ? "—" : formatBytes(file.size) }}</dd>
          <dt>{{ $t("details.owner") }}</dt>
          <dd class="truncate">{{ file.owner || file.ownerEmail || "—" }}</dd>
          <dt>{{ $t("details.modified") }}</dt>
          <dd>{{ formatDateLong(file.modifiedTime) }}</dd>
          <dt>{{ $t("details.created") }}</dt>
          <dd>{{ formatDateLong(file.createdTime) }}</dd>
          <dt>{{ $t("details.location") }}</dt>
          <dd class="truncate" :title="location">{{ location || "—" }}</dd>
          <dt>{{ $t("details.shared") }}</dt>
          <dd>{{ file.shared ? "✓" : "—" }}</dd>
          <dt>{{ $t("details.md5") }}</dt>
          <dd class="flex items-center gap-1">
            <span class="truncate font-mono text-[10.5px]" :title="file.md5 || ''">{{ file.md5 || "—" }}</span>
            <button v-if="file.md5" class="rounded p-0.5 hover:bg-muted" :title="$t('details.copy')" @click="copy(file.md5, 'toast.copiedId')">
              <Icon name="copy" :size="11" />
            </button>
          </dd>
          <dt>{{ $t("details.id") }}</dt>
          <dd class="flex items-center gap-1">
            <span class="truncate font-mono text-[10.5px]" :title="file.id">{{ file.id }}</span>
            <button class="rounded p-0.5 hover:bg-muted" :title="$t('details.copy')" @click="copy(file.id, 'toast.copiedId')">
              <Icon name="copy" :size="11" />
            </button>
          </dd>
        </dl>

        <div class="mt-3 flex flex-wrap gap-2">
          <AppButton v-if="file.webViewLink" size="sm" variant="secondary" icon="external" @click="openInBrowser">{{ $t("actions.openInDrive") }}</AppButton>
          <AppButton v-if="file.canDownload" size="sm" variant="ghost" icon="download" @click="files.download([file.id])">{{ $t("actions.download") }}</AppButton>
          <AppButton size="sm" variant="ghost" icon="star" @click="files.setStar([file.id], !file.starred)">
            {{ file.starred ? $t("actions.unstar") : $t("actions.star") }}
          </AppButton>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.details-grid {
  display: grid;
  grid-template-columns: 74px minmax(0, 1fr);
  row-gap: 7px;
  column-gap: 8px;
  font-size: 11.5px;
}
.details-grid dt {
  color: var(--muted-foreground);
}
.details-grid dd {
  margin: 0;
  min-width: 0;
}
</style>
