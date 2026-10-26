<script setup lang="ts">
import AppModal from "@/components/ui/AppModal.vue";
import AppButton from "@/components/ui/AppButton.vue";
import Icon from "@/components/ui/Icon.vue";
import SignInPanel from "@/components/auth/SignInPanel.vue";
import { useSessionStore } from "@/stores/session";
import { useUiStore } from "@/stores/ui";

const ui = useUiStore();
const session = useSessionStore();
</script>

<template>
  <AppModal
    :open="ui.loginOpen"
    :title="session.isMock ? $t('login.mockTitle') : $t('login.title')"
    :width="540"
    @close="ui.loginOpen = false"
  >
    <div v-if="session.isMock" class="space-y-3 pb-2">
      <p class="text-[12.5px] leading-relaxed text-muted-foreground">{{ $t("login.mockBody") }}</p>
      <div class="rounded-[8px] border border-border bg-muted/50 p-3 font-mono text-[11px]">DRIVEDESK_MOCK=1</div>
    </div>

    <div v-else class="space-y-3 pb-2">
      <div v-if="session.activeAccount" class="flex items-center gap-2 rounded-[8px] border border-border bg-muted/40 px-3 py-2 text-[12px]">
        <Icon name="check" :size="14" class="text-success" />
        <span class="min-w-0 flex-1 truncate">{{ session.activeAccount.email }}</span>
        <span v-if="session.activeAccount.authKind === 'token'" class="badge" data-tone="warning">token</span>
        <AppButton size="xs" variant="ghost" icon="logout" @click="session.signOut(session.activeAccount.email)">
          {{ $t("settings.signOut") }}
        </AppButton>
      </div>
      <SignInPanel />
    </div>

    <template #footer>
      <div class="flex-1" />
      <AppButton v-if="session.isMock" size="sm" variant="primary" @click="ui.loginOpen = false">{{ $t("login.close") }}</AppButton>
      <AppButton v-else size="sm" variant="ghost" @click="ui.loginOpen = false">{{ $t("actions.cancel") }}</AppButton>
    </template>
  </AppModal>
</template>
