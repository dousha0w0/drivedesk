import { computed, ref } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { errorMessage } from "@/lib/errors";
import { useFilesStore } from "@/stores/files";
import { useSessionStore } from "@/stores/session";
import { useUiStore } from "@/stores/ui";
import { i18n } from "@/i18n";

/**
 * Sign-in behaviour shared by the login dialog and the signed-out landing
 * page: one implementation, so both entry points always offer the same paths
 * and show the same errors.
 */
export function useGoogleSignIn() {
  const session = useSessionStore();
  const ui = useUiStore();
  const files = useFilesStore();

  const working = ref(false);
  const error = ref<string | null>(null);
  const hasBuiltin = computed(() => session.info?.builtinClient === true);

  async function run(action: () => Promise<void>): Promise<void> {
    error.value = null;
    working.value = true;
    try {
      await action();
      ui.toast(i18n.global.t("toast.loginSuccess", { email: session.activeAccount?.email ?? "" }), "success");
      ui.loginOpen = false;
      await files.navigate({ view: "folder", folderId: "root", query: "" });
    } catch (caught) {
      error.value = errorMessage(caught);
    } finally {
      working.value = false;
    }
  }

  function quick(): Promise<void> {
    return run(() => session.loginQuick());
  }

  async function importClient(): Promise<void> {
    error.value = null;
    const picked = await openDialog({ multiple: false, filters: [{ name: "OAuth client", extensions: ["json"] }] });
    if (!picked || Array.isArray(picked)) return;
    await run(() => session.login(picked));
  }

  return { working, error, hasBuiltin, quick, importClient };
}
