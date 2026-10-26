<script setup lang="ts">
import { ref } from "vue";
import AppButton from "@/components/ui/AppButton.vue";
import Icon from "@/components/ui/Icon.vue";
import { api } from "@/lib/api";
import { useGoogleSignIn } from "@/composables/useGoogleSignIn";

const { working, error, token } = useGoogleSignIn();
const expanded = ref(false);
const value = ref("");

function submit(): void {
  if (!value.value.trim() || working.value) return;
  void token(value.value);
}
</script>

<template>
  <div class="rounded-[7px] border border-border bg-muted/25">
    <button
      class="flex h-8 w-full items-center gap-1.5 px-2.5 text-[11.5px] text-muted-foreground hover:text-foreground"
      :aria-expanded="expanded"
      data-testid="token-toggle"
      @click="expanded = !expanded"
    >
      <Icon :name="expanded ? 'chevron-down' : 'chevron-right'" :size="12" />
      <Icon name="keyboard" :size="13" />
      <span>{{ $t("login.tokenTitle") }}</span>
    </button>

    <div v-if="expanded" class="space-y-2 border-t border-border px-2.5 py-2">
      <p class="text-[11px] leading-relaxed text-muted-foreground">{{ $t("login.tokenHint") }}</p>
      <input
        v-model="value"
        type="password"
        autocomplete="off"
        spellcheck="false"
        class="h-8 w-full rounded-[6px] border border-input bg-background px-2 font-mono text-[11.5px] outline-none focus:border-ring"
        :placeholder="$t('login.tokenPlaceholder')"
        data-testid="token-input"
        @keydown.enter.prevent="submit"
      />
      <div class="flex flex-wrap items-center gap-2">
        <AppButton size="sm" variant="secondary" icon="account" :disabled="!value.trim() || working" data-testid="token-submit" @click="submit">
          {{ $t("login.tokenButton") }}
        </AppButton>
        <AppButton size="sm" variant="ghost" icon="external" @click="api.systemOpenUrl('https://developers.google.com/oauthplayground')">
          {{ $t("login.tokenHowTo") }}
        </AppButton>
        <span class="w-full font-mono text-[10px] text-muted-foreground">{{ $t("login.tokenNeedsScope") }}</span>
      </div>
      <p v-if="error" class="text-[11px] leading-relaxed text-destructive">{{ error }}</p>
      <p v-if="working" class="flex items-center gap-1.5 text-[11px] text-muted-foreground">
        <Icon name="spinner" :size="12" class="spin" />{{ $t("login.working") }}
      </p>
    </div>
  </div>
</template>
