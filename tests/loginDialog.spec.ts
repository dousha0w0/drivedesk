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
  beforeEach(() => {
    setLocale("zh-CN");
    window.localStorage.clear();
  });

  it("offers one-click sign-in when the build carries a client", () => {
    const wrapper = mountDialog({ builtinClient: true });
    const submit = wrapper.find('[data-testid="signin-google-submit"]');
    expect(submit.exists()).toBe(true);
    expect(submit.text()).toBe(i18n.global.t("login.signIn"));
    expect(wrapper.text()).toContain(i18n.global.t("login.builtinHint"));
    expect(wrapper.text()).toContain(i18n.global.t("login.testingNote"));
  });

  it("opens the token tab when this build has no client, with the JSON tab beside it", async () => {
    const wrapper = mountDialog({ builtinClient: false });
    expect(wrapper.find('[data-testid="signin-google-submit"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="token-input"]').exists()).toBe(true);

    await wrapper.find('[data-testid="signin-method-json"]').trigger("click");
    expect(wrapper.text()).toContain(i18n.global.t("login.noBuiltin"));
    expect(wrapper.find('[data-testid="json-submit"]').text()).toBe(i18n.global.t("login.import"));
  });

  it("keeps the demo copy in mock mode", () => {
    const wrapper = mountDialog({ builtinClient: false, mock: true });
    expect(wrapper.text()).toContain(i18n.global.t("login.mockBody"));
    expect(wrapper.find('[data-testid="signin-method-token"]').exists()).toBe(false);
  });
});
