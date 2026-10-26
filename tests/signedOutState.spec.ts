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
  beforeEach(() => setLocale("zh-CN"));

  it("is reachable and offers one-click sign-in when the build has a client", () => {
    const wrapper = mountState({ builtinClient: true });
    const primary = wrapper.find('[data-testid="signed-out-primary"]');
    expect(primary.exists()).toBe(true);
    expect(primary.text()).toBe(i18n.global.t("login.signIn"));
    expect(wrapper.text()).toContain(i18n.global.t("login.builtinHint"));
    expect(wrapper.text()).toContain(i18n.global.t("login.useOwn"));
  });

  it("falls back to importing a client JSON when the build has none", () => {
    const wrapper = mountState({ builtinClient: false });
    const primary = wrapper.find('[data-testid="signed-out-primary"]');
    expect(primary.text()).toBe(i18n.global.t("login.import"));
    expect(wrapper.text()).toContain(i18n.global.t("login.noBuiltin"));
    expect(wrapper.text()).not.toContain(i18n.global.t("login.useOwn"));
  });
});
