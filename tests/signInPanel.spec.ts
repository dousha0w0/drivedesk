import { beforeEach, describe, expect, it } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import SignInPanel from "@/components/auth/SignInPanel.vue";
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

function mountPanel(builtinClient: boolean | null) {
  const pinia = createPinia();
  setActivePinia(pinia);
  const session = useSessionStore();
  session.info = builtinClient === null ? null : info({ builtinClient });
  return { wrapper: mount(SignInPanel, { global: { plugins: [pinia, i18n] } }), session };
}

describe("SignInPanel", () => {
  beforeEach(() => {
    setLocale("zh-CN");
    window.localStorage.clear();
  });

  it("opens on the Google tab when the build carries a client", () => {
    const { wrapper } = mountPanel(true);
    expect(wrapper.find('[data-testid="signin-method-google"]').attributes("aria-selected")).toBe("true");
    expect(wrapper.find('[data-testid="signin-google-submit"]').text()).toBe(i18n.global.t("login.signIn"));
    expect(wrapper.text()).toContain(i18n.global.t("login.testingNote"));
  });

  it("opens on the token tab when there is no client, and still offers the JSON tab", async () => {
    const { wrapper } = mountPanel(false);
    expect(wrapper.find('[data-testid="signin-method-google"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="signin-method-token"]').attributes("aria-selected")).toBe("true");
    expect(wrapper.find('[data-testid="token-input"]').exists()).toBe(true);

    await wrapper.find('[data-testid="signin-method-json"]').trigger("click");
    expect(wrapper.find('[data-testid="json-submit"]').exists()).toBe(true);
    expect(wrapper.text()).toContain(i18n.global.t("login.noBuiltin"));
  });

  it("keeps the token submit disabled until something is pasted", async () => {
    const { wrapper } = mountPanel(false);
    const submit = () => wrapper.find('[data-testid="token-submit"]');
    expect(submit().attributes("disabled")).toBeDefined();

    await wrapper.find('[data-testid="token-input"]').setValue("ya29.some-token");
    expect(submit().attributes("disabled")).toBeUndefined();
    expect(wrapper.find('[data-testid="token-input"]').attributes("type")).toBe("password");
  });

  it("remembers an explicit method and does not override it when capabilities arrive", async () => {
    const { wrapper, session } = mountPanel(null);
    await wrapper.find('[data-testid="signin-method-json"]').trigger("click");
    expect(window.localStorage.getItem("drivedesk.signInMethod")).toBe("json");

    session.info = info({ builtinClient: true });
    await flushPromises();
    expect(wrapper.find('[data-testid="signin-method-json"]').attributes("aria-selected")).toBe("true");
    expect(wrapper.find('[data-testid="signin-google-submit"]').exists()).toBe(false);
  });
});
