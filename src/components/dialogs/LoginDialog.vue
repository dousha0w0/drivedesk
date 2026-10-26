<script setup lang="ts">
import { ref } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import AppModal from "@/components/ui/AppModal.vue";
import AppButton from "@/components/ui/AppButton.vue";
import Icon from "@/components/ui/Icon.vue";
import { api } from "@/lib/api";
import { errorMessage } from "@/lib/errors";
import { useFilesStore } from "@/stores/files";
import { useSessionStore } from "@/stores/session";
import { useUiStore } from "@/stores/ui";
import { i18n } from "@/i18n";

const ui = useUiStore();
const session = useSessionStore();
const files = useFilesStore();
const t = (key: string, params?: Record<string, unknown>) => i18n.global.t(key, params ?? {});

const working = ref(false);
const error = ref<string | null>(null);

async function importClient(): Promise<void> {
  error.value = null;
  const picked = await openDialog({ multiple: false, filters: [{ name: "OAuth client", extensions: ["json"] }] });
  if (!picked || Array.isArray(picked)) return;
  working.value = true;
  try {
    await session.login(picked);
    ui.toast(t("toast.loginSuccess", { email: session.activeAccount?.email ?? "" }), "success");
    ui.loginOpen = false;
    await files.navigate({ view: "folder", folderId: "root", query: "" });
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    working.value = false;
  }
}

function openDocs(): void {
  void api.systemOpenUrl("https://developers.google.com/drive/api/quickstart");
}
</script>

<template>
  <AppModal :open="ui.loginOpen" :title="session.isMock ? $t('login.mockTitle') : $t('login.title')" :width="520" @close="ui.loginOpen = false">
    <div v-if="session.isMock" class="space-y-3 pb-2">
      <p class="text-[12.5px] leading-relaxed text-muted-foreground">{{ $t("login.mockBody") }}</p>
      <div class="rounded-[8px] border border-border bg-muted/50 p-3 font-mono text-[11px]">DRIVEDESK_MOCK=1</div>
    </div>

    <div v-else class="space-y-2 pb-2">
      <p class="text-[12.5px] leading-relaxed text-muted-foreground">{{ $t("login.body") }}</p>
      <ol class="ml-4 list-decimal space-y-1 text-[12px] text-muted-foreground">
        <li>{{ $t("login.step1") }}</li>
        <li>{{ $t("login.step2") }}</li>
        <li>{{ $t("login.step3") }}</li>
      </ol>
      <div v-if="session.activeAccount" class="flex items-center gap-2 rounded-[8px] border border-border bg-muted/40 px-3 py-2 text-[12px]">
        <Icon name="check" :size="14" class="text-success" />
        <span class="flex-1 truncate">{{ session.activeAccount.email }}</span>
      </div>
      <div v-if="error" class="rounded-[6px] border border-destructive/30 bg-destructive/10 px-2.5 py-2 text-[11.5px] text-destructive">
        {{ error }}
      </div>
      <div v-if="working" class="flex items-center gap-2 text-[11.5px] text-muted-foreground">
        <Icon name="spinner" :size="13" class="spin" />{{ $t("login.working") }}
      </div>
    </div>

    <template #footer>
      <AppButton size="sm" variant="ghost" icon="external" @click="openDocs">{{ $t("login.docs") }}</AppButton>
      <div class="flex-1" />
      <AppButton v-if="session.isMock" size="sm" variant="primary" @click="ui.loginOpen = false">{{ $t("login.close") }}</AppButton>
      <template v-else>
        <AppButton size="sm" variant="ghost" @click="ui.loginOpen = false">{{ $t("actions.cancel") }}</AppButton>
        <AppButton size="sm" variant="primary" icon="account" :disabled="working" @click="importClient">{{ $t("login.import") }}</AppButton>
      </template>
    </template>
  </AppModal>
</template>
