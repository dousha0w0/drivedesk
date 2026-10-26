import { defineStore } from "pinia";
import { ref } from "vue";
import type { FileMeta } from "@/types/drive";

export type PromptMode = "new-folder" | "rename";

export interface PromptState {
  mode: PromptMode;
  targetId: string | null;
  title: string;
  label: string;
  value: string;
}

export interface ConfirmState {
  title: string;
  body: string;
  confirmLabel: string;
  danger: boolean;
  action: () => void | Promise<void>;
}

export interface MoveState {
  fileId: string;
  name: string;
}

/**
 * One modal at a time, driven by a small state machine instead of per-dialog
 * booleans: the shell renders `PromptDialog`/`ConfirmDialog`/`MoveDialog` and
 * each closes itself by clearing this store.
 */
export const useDialogsStore = defineStore("dialogs", () => {
  const prompt = ref<PromptState | null>(null);
  const confirm = ref<ConfirmState | null>(null);
  const move = ref<MoveState | null>(null);

  function newFolder(): void {
    prompt.value = {
      mode: "new-folder",
      targetId: null,
      title: "dialog.newFolderTitle",
      label: "dialog.newFolderLabel",
      value: "",
    };
  }

  function rename(file: FileMeta): void {
    prompt.value = {
      mode: "rename",
      targetId: file.id,
      title: "dialog.renameTitle",
      label: "dialog.renameLabel",
      value: file.name,
    };
  }

  function ask(state: ConfirmState): void {
    confirm.value = state;
  }

  function moveTo(file: FileMeta): void {
    move.value = { fileId: file.id, name: file.name };
  }

  function closePrompt(): void {
    prompt.value = null;
  }

  function closeConfirm(): void {
    confirm.value = null;
  }

  function closeMove(): void {
    move.value = null;
  }

  return { prompt, confirm, move, newFolder, rename, ask, moveTo, closePrompt, closeConfirm, closeMove };
});
