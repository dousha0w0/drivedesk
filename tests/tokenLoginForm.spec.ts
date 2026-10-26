import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import TokenLoginForm from "@/components/auth/TokenLoginForm.vue";
import { i18n, setLocale } from "@/i18n";

function mountForm() {
  const pinia = createPinia();
  setActivePinia(pinia);
  return mount(TokenLoginForm, { global: { plugins: [pinia, i18n] } });
}

describe("TokenLoginForm", () => {
  beforeEach(() => setLocale("zh-CN"));

  it("stays collapsed until the developer asks for it", () => {
    const wrapper = mountForm();
    expect(wrapper.find('[data-testid="token-input"]').exists()).toBe(false);
    expect(wrapper.text()).toContain(i18n.global.t("login.tokenTitle"));
  });

  it("requires a non-empty token before submitting", async () => {
    const wrapper = mountForm();
    await wrapper.find('[data-testid="token-toggle"]').trigger("click");
    const input = wrapper.find('[data-testid="token-input"]');
    expect(input.exists()).toBe(true);
    expect(input.attributes("type")).toBe("password");

    const submit = wrapper.find('[data-testid="token-submit"]');
    expect(submit.attributes("disabled")).toBeDefined();

    await input.setValue("ya29.test-token");
    expect(wrapper.find('[data-testid="token-submit"]').attributes("disabled")).toBeUndefined();
  });
});
