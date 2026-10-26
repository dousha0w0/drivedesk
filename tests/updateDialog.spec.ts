import { beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import UpdateDialog from "@/components/dialogs/UpdateDialog.vue";
import { i18n, setLocale } from "@/i18n";
import { useUpdaterStore } from "@/stores/updater";

function mountDialog() {
  const pinia = createPinia();
  setActivePinia(pinia);
  const updater = useUpdaterStore();
  updater.currentVersion = "0.1.0";
  updater.update = {
    version: "0.2.0",
    date: "2026-11-01T10:00:00Z",
    body: "## 0.2.0\n\n- 修了 A\n- 加了 B",
    downloadAndInstall: vi.fn(),
  } as never;
  updater.dialogOpen = true;
  return { wrapper: mount(UpdateDialog, { global: { plugins: [pinia, i18n], stubs: { teleport: true } } }), updater };
}

describe("UpdateDialog", () => {
  beforeEach(() => {
    setLocale("zh-CN");
    window.localStorage.clear();
  });

  it("shows both versions and the release notes", () => {
    const { wrapper } = mountDialog();
    expect(wrapper.text()).toContain("0.1.0");
    expect(wrapper.text()).toContain("0.2.0");
    expect(wrapper.text()).toContain("修了 A");
    expect(wrapper.text()).toContain(i18n.global.t("updates.install"));
  });

  it("records an ignored version and closes", () => {
    const { wrapper, updater } = mountDialog();
    const ignore = wrapper.findAll("button").find((button) => button.text() === i18n.global.t("updates.ignore"));
    expect(ignore).toBeTruthy();
    void ignore!.trigger("click");
    expect(updater.prefs.ignoredVersion).toBe("0.2.0");
    expect(updater.update).toBeNull();
    expect(updater.dialogOpen).toBe(false);
  });
});
