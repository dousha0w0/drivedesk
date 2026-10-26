<script setup lang="ts">
import Icon from "@/components/ui/Icon.vue";
import AppButton from "@/components/ui/AppButton.vue";
import { api } from "@/lib/api";
import TokenLoginForm from "@/components/auth/TokenLoginForm.vue";
import { useGoogleSignIn } from "@/composables/useGoogleSignIn";

const { working, error, hasBuiltin, quick, importClient } = useGoogleSignIn();

function openExternal(url: string): void {
  void api.systemOpenUrl(url);
}
</script>

<template>
  <div class="flex min-h-0 flex-1 items-center justify-center overflow-auto p-8" data-testid="signed-out-state">
    <div class="w-full max-w-[520px] rounded-[var(--dr-lg)] border border-border bg-card p-6 shadow-sm">
      <div class="mb-3 flex items-center gap-2">
        <span class="flex h-9 w-9 items-center justify-center rounded-[8px] bg-muted text-muted-foreground">
          <Icon name="cloud" :size="19" />
        </span>
        <div>
          <div class="text-[14px] font-semibold">{{ $t("login.title") }}</div>
          <div class="text-[11.5px] text-muted-foreground">{{ $t("login.signedOutHint") }}</div>
        </div>
      </div>

      <p class="mb-3 text-[12px] leading-relaxed text-muted-foreground">{{ $t("login.body") }}</p>

      <button
        class="mb-3 flex h-9 w-full items-center justify-center gap-2 rounded-[7px] bg-primary text-[13px] font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-50"
        :disabled="working"
        data-testid="signed-out-primary"
        @click="hasBuiltin ? quick() : importClient()"
      >
        <Icon :name="hasBuiltin ? 'account' : 'folder'" :size="15" />
        {{ hasBuiltin ? $t("login.signIn") : $t("login.import") }}
      </button>

      <p class="mb-2 text-[11px] leading-relaxed text-muted-foreground">
        {{ hasBuiltin ? $t("login.builtinHint") : $t("login.noBuiltin") }}
      </p>
      <p v-if="hasBuiltin" class="mb-2 text-[11px] leading-relaxed text-muted-foreground">{{ $t("login.testingNote") }}</p>

      <div class="flex flex-wrap gap-2">
        <AppButton
          v-if="hasBuiltin"
          size="sm"
          variant="ghost"
          icon="folder"
          :disabled="working"
          @click="importClient"
        >
          {{ $t("login.useOwn") }}
        </AppButton>
        <AppButton
          size="sm"
          variant="ghost"
          icon="external"
          @click="openExternal('https://developers.google.com/workspace/guides/create-credentials')"
        >
          {{ $t("login.linkCredentials") }}
        </AppButton>
        <AppButton size="sm" variant="ghost" icon="shield" @click="openExternal('https://support.google.com/cloud/answer/6158849')">
          {{ $t("login.linkConsent") }}
        </AppButton>
      </div>

      <div class="mt-3"><TokenLoginForm /></div>

      <div v-if="error" class="mt-3 rounded-[6px] border border-destructive/30 bg-destructive/10 px-2.5 py-2 text-[11.5px] leading-relaxed text-destructive">
        {{ error }}
      </div>
      <div v-if="working" class="mt-3 flex items-center gap-2 text-[11.5px] text-muted-foreground">
        <Icon name="spinner" :size="13" class="spin" />{{ $t("login.working") }}
      </div>
    </div>
  </div>
</template>
