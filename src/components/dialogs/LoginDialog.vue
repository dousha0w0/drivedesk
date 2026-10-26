<script setup lang="ts">
import { computed, ref } from "vue";
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

const hasBuiltin = computed(() => session.info?.builtinClient === true);

async function finishLogin(): Promise<void> {
  ui.toast(t("toast.loginSuccess", { email: session.activeAccount?.email ?? "" }), "success");
  ui.loginOpen = false;
  await files.navigate({ view: "folder", folderId: "root", query: "" });
}

async function quickLogin(): Promise<void> {
  error.value = null;
  working.value = true;
  try {
    await session.loginQuick();
    await finishLogin();
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    working.value = false;
  }
}

async function importClient(): Promise<void> {
  error.value = null;
  const picked = await openDialog({ multiple: false, filters: [{ name: "OAuth client", extensions: ["json"] }] });
  if (!picked || Array.isArray(picked)) return;
  working.value = true;
  try {
    await session.login(picked);
    await finishLogin();
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    working.value = false;
  }
}

function openExternal(url: string): void {
  void api.systemOpenUrl(url);
}
</script>

<template>
  <AppModal
    :open="ui.loginOpen"
    :title="session.isMock ? $t('login.mockTitle') : $t('login.title')"
    :width="520"
    @close="ui.loginOpen = false"
  >
    <div v-if="session.isMock" class="space-y-3 pb-2">
      <p class="text-[12.5px] leading-relaxed text-muted-foreground">{{ $t("login.mockBody") }}</p>
      <div class="rounded-[8px] border border-border bg-muted/50 p-3 font-mono text-[11px]">DRIVEDESK_MOCK=1</div>
    </div>

    <div v-else class="space-y-3 pb-2">
      <p class="text-[12.5px] leading-relaxed text-muted-foreground">{{ $t("login.body") }}</p>

      <div v-if="session.activeAccount" class="flex items-center gap-2 rounded-[8px] border border-border bg-muted/40 px-3 py-2 text-[12px]">
        <Icon name="check" :size="14" class="text-success" />
        <span class="min-w-0 flex-1 truncate">{{ session.activeAccount.email }}</span>
        <AppButton size="xs" variant="ghost" icon="logout" @click="session.signOut(session.activeAccount.email)">
          {{ $t("settings.signOut") }}
        </AppButton>
      </div>

      <template v-if="hasBuiltin">
        <button
          class="flex h-9 w-full items-center justify-center gap-2 rounded-[7px] bg-primary text-[13px] font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-50"
          :disabled="working"
          data-testid="quick-login"
          @click="quickLogin"
        >
          <Icon name="account" :size="15" />
          {{ $t("login.signIn") }}
        </button>
        <p class="text-[11px] leading-relaxed text-muted-foreground">{{ $t("login.builtinHint") }}</p>
        <p class="text-[11px] leading-relaxed text-muted-foreground">{{ $t("login.testingNote") }}</p>
        <div class="flex items-center gap-2 pt-1">
          <div class="h-px flex-1 bg-border" />
          <span class="text-[10.5px] text-muted-foreground">BYO</span>
          <div class="h-px flex-1 bg-border" />
        </div>
      </template>

      <template v-else>
        <ol class="ml-4 list-decimal space-y-1 text-[12px] text-muted-foreground">
          <li>{{ $t("login.step1") }}</li>
          <li>{{ $t("login.step2") }}</li>
          <li>{{ $t("login.step3") }}</li>
        </ol>
        <p class="rounded-[6px] border border-border bg-muted/40 px-2.5 py-2 text-[11.5px] text-muted-foreground">
          {{ $t("login.noBuiltin") }}
        </p>
      </template>

      <div class="flex flex-wrap gap-2">
        <AppButton size="sm" :variant="hasBuiltin ? 'ghost' : 'primary'" icon="folder" :disabled="working" @click="importClient">
          {{ hasBuiltin ? $t("login.useOwn") : $t("login.import") }}
        </AppButton>
        <AppButton size="sm" variant="ghost" icon="external" @click="openExternal('https://developers.google.com/workspace/guides/create-credentials')">
          {{ $t("login.linkCredentials") }}
        </AppButton>
        <AppButton size="sm" variant="ghost" icon="shield" @click="openExternal('https://support.google.com/cloud/answer/6158849')">
          {{ $t("login.linkConsent") }}
        </AppButton>
      </div>

      <div v-if="error" class="rounded-[6px] border border-destructive/30 bg-destructive/10 px-2.5 py-2 text-[11.5px] leading-relaxed text-destructive">
        {{ error }}
      </div>
      <div v-if="working" class="flex items-center gap-2 text-[11.5px] text-muted-foreground">
        <Icon name="spinner" :size="13" class="spin" />{{ $t("login.working") }}
      </div>
    </div>

    <template #footer>
      <div class="flex-1" />
      <AppButton v-if="session.isMock" size="sm" variant="primary" @click="ui.loginOpen = false">{{ $t("login.close") }}</AppButton>
      <AppButton v-else size="sm" variant="ghost" @click="ui.loginOpen = false">{{ $t("actions.cancel") }}</AppButton>
    </template>
  </AppModal>
</template>
