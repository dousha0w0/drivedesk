import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import SignedOutState from "@/components/layout/SignedOutState.vue";
import { i18n, setLocale } from "@/i18n";
import { useSessionStore } from "@/stores/session";
import type { AppInfo } from "@/types/drive";

function info(overrides: Partial<AppInfo> = {}): AppInfo {
  return {
    version: "0.1.0",
    mock: false,
    dataDir: "/tmp/drivedesk-test",
    cacheDir: "/tmp/drivedesk-test-cache",
    defaultDownloadDir: "/tmp/downloads",
    secretStorage: "test",
    accounts: [],
    builtinClient: false,
    gdirectFound: false,
    gdirectPath: null,
    gdirectVersion: null,
    platform: "macos",
    ...overrides,
  };
}

function mountState(overrides: Partial<AppInfo>) {
  const pinia = createPinia();
  setActivePinia(pinia);
  useSessionStore().info = info(overrides);
  return mount(SignedOutState, { global: { plugins: [pinia, i18n] } });
}

describe("SignedOutState", () => {
  beforeEach(() => {
    setLocale("zh-CN");
    window.localStorage.clear();
  });

  it("is reachable and offers one-click sign-in when the build has a client", () => {
    const wrapper = mountState({ builtinClient: true });
    const submit = wrapper.find('[data-testid="signin-google-submit"]');
    expect(submit.exists()).toBe(true);
    expect(submit.text()).toBe(i18n.global.t("login.signIn"));
    expect(wrapper.text()).toContain(i18n.global.t("login.builtinHint"));
  });

  it("falls back to the token tab when the build has no client", () => {
    const wrapper = mountState({ builtinClient: false });
    expect(wrapper.find('[data-testid="signin-google-submit"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="token-input"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="signin-method-json"]').exists()).toBe(true);
  });
});
