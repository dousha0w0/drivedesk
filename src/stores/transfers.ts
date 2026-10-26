import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { api, onTransfer } from "@/lib/api";
import { errorMessage } from "@/lib/errors";
import type { Transfer } from "@/types/drive";

export const useTransfersStore = defineStore("transfers", () => {
  const items = ref<Transfer[]>([]);
  const initialized = ref(false);
  const error = ref<string | null>(null);
  /** Bumped whenever a transfer finishes so the file list can refresh. */
  const completionTick = ref(0);
  let unlisten: (() => void) | null = null;

  const active = computed(() => items.value.filter((item) => item.status === "running" || item.status === "queued"));
  const activeCount = computed(() => active.value.length);
  const runningCount = computed(() => items.value.filter((item) => item.status === "running").length);
  const totalSpeed = computed(() => items.value.reduce((sum, item) => sum + (item.status === "running" ? item.speedBps : 0), 0));
  const finishedCount = computed(
    () => items.value.filter((item) => item.status === "done" || item.status === "error" || item.status === "canceled").length,
  );

  function upsert(transfer: Transfer): void {
    const index = items.value.findIndex((item) => item.id === transfer.id);
    const previous = index === -1 ? null : items.value[index];
    if (index === -1) {
      items.value = [transfer, ...items.value];
    } else {
      const next = [...items.value];
      next[index] = transfer;
      items.value = next;
    }
    if (transfer.status === "done" && previous?.status !== "done") {
      completionTick.value += 1;
    }
  }

  async function init(): Promise<void> {
    if (initialized.value) return;
    initialized.value = true;
    try {
      items.value = await api.transfersList();
      unlisten = await onTransfer(upsert);
    } catch (caught) {
      error.value = errorMessage(caught);
    }
  }

  async function cancel(id: string): Promise<void> {
    await api.transferCancel(id);
  }

  async function retry(id: string): Promise<void> {
    await api.transferRetry(id);
  }

  async function clearFinished(): Promise<void> {
    await api.transfersClearFinished();
    items.value = items.value.filter((item) => !["done", "error", "canceled"].includes(item.status));
  }

  function dispose(): void {
    unlisten?.();
    unlisten = null;
  }

  return {
    items,
    initialized,
    error,
    completionTick,
    active,
    activeCount,
    runningCount,
    totalSpeed,
    finishedCount,
    init,
    upsert,
    cancel,
    retry,
    clearFinished,
    dispose,
  };
});
