import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import LoginDialog from "@/components/dialogs/LoginDialog.vue";
import { i18n, setLocale } from "@/i18n";
import { useSessionStore } from "@/stores/session";
import { useUiStore } from "@/stores/ui";
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
    platform: "darwin",
    ...overrides,
  };
}

function mountDialog(overrides: Partial<AppInfo>) {
  const pinia = createPinia();
  setActivePinia(pinia);
  const session = useSessionStore();
  session.info = info(overrides);
  const ui = useUiStore();
  ui.loginOpen = true;
  return mount(LoginDialog, {
    global: { plugins: [pinia, i18n], stubs: { teleport: true } },
  });
}

describe("LoginDialog", () => {
  beforeEach(() => setLocale("zh-CN"));

  it("offers one-click sign-in when the build carries a client", () => {
    const wrapper = mountDialog({ builtinClient: true });
    const quick = wrapper.find('[data-testid="quick-login"]');
    expect(quick.exists()).toBe(true);
    expect(quick.text()).toBe(i18n.global.t("login.signIn"));
    expect(wrapper.text()).toContain(i18n.global.t("login.builtinHint"));
    expect(wrapper.text()).toContain(i18n.global.t("login.testingNote"));
    expect(wrapper.text()).toContain(i18n.global.t("login.useOwn"));
  });

  it("asks for a client JSON when this build has none", () => {
    const wrapper = mountDialog({ builtinClient: false });
    expect(wrapper.find('[data-testid="quick-login"]').exists()).toBe(false);
    expect(wrapper.text()).toContain(i18n.global.t("login.noBuiltin"));
    expect(wrapper.text()).toContain(i18n.global.t("login.step1"));
    expect(wrapper.text()).toContain(i18n.global.t("login.import"));
  });

  it("keeps the demo copy in mock mode", () => {
    const wrapper = mountDialog({ builtinClient: false, mock: true });
    expect(wrapper.text()).toContain(i18n.global.t("login.mockBody"));
    expect(wrapper.find('[data-testid="quick-login"]').exists()).toBe(false);
  });
});
