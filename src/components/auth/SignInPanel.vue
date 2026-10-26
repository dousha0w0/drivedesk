<script setup lang="ts">
import { computed, ref, watch } from "vue";
import AppButton from "@/components/ui/AppButton.vue";
import Icon from "@/components/ui/Icon.vue";
import { api } from "@/lib/api";
import { useGoogleSignIn } from "@/composables/useGoogleSignIn";

type Method = "google" | "token" | "json";

const { working, error, hasBuiltin, quick, token, importClient } = useGoogleSignIn();

const STORAGE_KEY = "drivedesk.signInMethod";
const stored = (() => {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    return value === "google" || value === "token" || value === "json" ? value : null;
  } catch {
    return null;
  }
})();

const chosen = ref<Method | null>(stored);
const method = ref<Method>(stored ?? (hasBuiltin.value ? "google" : "token"));
const tokenValue = ref("");
const showSteps = ref(false);

// The build's capabilities arrive with app_info after mount; only follow them
// until the user picks a method themselves.
watch(hasBuiltin, (value) => {
  if (!chosen.value) method.value = value ? "google" : "token";
});

const methods = computed<{ id: Method; label: string; icon: string }[]>(() =>
  [
    ...(hasBuiltin.value ? [{ id: "google" as Method, label: "login.methodGoogle", icon: "account" }] : []),
    { id: "token", label: "login.methodToken", icon: "keyboard" },
    { id: "json", label: "login.methodJson", icon: "file" },
  ],
);

function choose(next: Method): void {
  chosen.value = next;
  method.value = next;
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    // storage unavailable: the choice simply does not persist
  }
}

function submitToken(): void {
  if (!tokenValue.value.trim() || working.value) return;
  void token(tokenValue.value);
}

function openExternal(url: string): void {
  void api.systemOpenUrl(url);
}
</script>

<template>
  <div>
    <div class="mb-3 flex gap-1 rounded-[7px] border border-border bg-muted p-1" role="tablist">
      <button
        v-for="item in methods"
        :key="item.id"
        class="flex h-7 flex-1 items-center justify-center gap-1.5 rounded-[5px] text-[11.5px] text-muted-foreground transition-colors"
        :class="method === item.id ? 'bg-card text-foreground shadow-sm' : 'hover:text-foreground'"
        :data-testid="`signin-method-${item.id}`"
        role="tab"
        :aria-selected="method === item.id"
        @click="choose(item.id)"
      >
        <Icon :name="item.icon" :size="13" />
        {{ $t(item.label) }}
      </button>
    </div>

    <div v-if="method === 'google'" class="space-y-2">
      <button
        class="flex h-9 w-full items-center justify-center gap-2 rounded-[7px] bg-primary text-[13px] font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-50"
        :disabled="working"
        data-testid="signin-google-submit"
        @click="quick"
      >
        <Icon name="account" :size="15" />
        {{ $t("login.signIn") }}
      </button>
      <p class="text-[11px] leading-relaxed text-muted-foreground">{{ $t("login.builtinHint") }}</p>
      <p class="text-[11px] leading-relaxed text-muted-foreground">{{ $t("login.testingNote") }}</p>
      <button class="text-[11px] text-muted-foreground underline-offset-2 hover:text-foreground hover:underline" @click="choose('json')">
        {{ $t("login.useOwn") }}
      </button>
    </div>

    <div v-else-if="method === 'token'" class="space-y-2">
      <p class="text-[11px] leading-relaxed text-muted-foreground">{{ $t("login.tokenHint") }}</p>
      <input
        v-model="tokenValue"
        type="password"
        autocomplete="off"
        spellcheck="false"
        class="h-8 w-full rounded-[6px] border border-input bg-background px-2 font-mono text-[11.5px] outline-none focus:border-ring"
        :placeholder="$t('login.tokenPlaceholder')"
        data-testid="token-input"
        @keydown.enter.prevent="submitToken"
      />
      <div class="flex flex-wrap items-center gap-2">
        <AppButton size="sm" variant="primary" icon="account" :disabled="!tokenValue.trim() || working" data-testid="token-submit" @click="submitToken">
          {{ $t("login.tokenButton") }}
        </AppButton>
        <AppButton size="sm" variant="ghost" icon="external" @click="openExternal('https://developers.google.com/oauthplayground')">
          {{ $t("login.tokenHowTo") }}
        </AppButton>
      </div>
      <p class="font-mono text-[10px] text-muted-foreground">{{ $t("login.tokenNeedsScope") }}</p>
    </div>

    <div v-else class="space-y-2">
      <p v-if="!hasBuiltin" class="text-[11px] leading-relaxed text-muted-foreground">{{ $t("login.noBuiltin") }}</p>
      <AppButton size="sm" variant="primary" icon="folder" :disabled="working" data-testid="json-submit" @click="importClient">
        {{ $t("login.import") }}
      </AppButton>
      <button class="block text-[11px] text-muted-foreground underline-offset-2 hover:text-foreground hover:underline" @click="showSteps = !showSteps">
        {{ $t("login.howToCreate") }}
      </button>
      <ol v-if="showSteps" class="ml-4 list-decimal space-y-1 text-[11px] leading-relaxed text-muted-foreground">
        <li>{{ $t("login.step1") }}</li>
        <li>{{ $t("login.step2") }}</li>
        <li>{{ $t("login.step3") }}</li>
      </ol>
      <div class="flex flex-wrap gap-2 pt-0.5">
        <AppButton size="xs" variant="ghost" icon="external" @click="openExternal('https://developers.google.com/workspace/guides/create-credentials')">
          {{ $t("login.linkCredentials") }}
        </AppButton>
        <AppButton size="xs" variant="ghost" icon="shield" @click="openExternal('https://support.google.com/cloud/answer/6158849')">
          {{ $t("login.linkConsent") }}
        </AppButton>
      </div>
    </div>

    <p v-if="error" class="mt-3 rounded-[6px] border border-destructive/30 bg-destructive/10 px-2.5 py-2 text-[11.5px] leading-relaxed text-destructive">
      {{ error }}
    </p>
    <p v-if="working" class="mt-3 flex items-center gap-1.5 text-[11.5px] text-muted-foreground">
      <Icon name="spinner" :size="12" class="spin" />{{ $t("login.working") }}
    </p>
  </div>
</template>
