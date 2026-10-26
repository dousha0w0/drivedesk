const UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

export function formatBytes(value: number | null | undefined, digits = 1): string {
  if (value === null || value === undefined || Number.isNaN(value)) return "—";
  if (value < 1024) return `${value} B`;
  let amount = value;
  let unit = 0;
  while (amount >= 1024 && unit < UNITS.length - 1) {
    amount /= 1024;
    unit += 1;
  }
  const decimals = amount >= 100 ? 0 : digits;
  return `${amount.toFixed(decimals)} ${UNITS[unit]}`;
}

export function formatSpeed(bytesPerSecond: number): string {
  if (!bytesPerSecond || bytesPerSecond < 1) return "";
  return `${formatBytes(bytesPerSecond)}/s`;
}

const DAY = 86_400_000;

export function formatDate(iso: string | null | undefined, now = new Date()): string {
  if (!iso) return "—";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "—";
  const diff = now.getTime() - date.getTime();
  const time = `${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
  if (diff < 0) return date.toLocaleDateString();
  if (diff < 60_000) return "刚刚";
  if (date.toDateString() === now.toDateString()) return time;
  const yesterday = new Date(now.getTime() - DAY);
  if (date.toDateString() === yesterday.toDateString()) return `昨天 ${time}`;
  const year = date.getFullYear() === now.getFullYear() ? "" : `${date.getFullYear()}/`;
  return `${year}${date.getMonth() + 1}/${date.getDate()} ${time}`;
}

export function formatDateLong(iso: string | null | undefined): string {
  if (!iso) return "—";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "—";
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function formatDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return "—";
  const seconds = Math.round(ms / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${seconds % 60}s`;
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
}

/** Remaining time from the current rate; empty when it cannot be known. */
export function eta(transfer: { received: number; total: number | null; speedBps: number }): string {
  if (!transfer.total || !transfer.speedBps) return "";
  const remaining = transfer.total - transfer.received;
  if (remaining <= 0) return "0s";
  return formatDuration((remaining / transfer.speedBps) * 1000);
}

export function extensionOf(name: string): string {
  const index = name.lastIndexOf(".");
  if (index <= 0 || index === name.length - 1) return "";
  return name.slice(index + 1).toLowerCase();
}

export function stemOf(name: string): string {
  const index = name.lastIndexOf(".");
  return index <= 0 ? name : name.slice(0, index);
}

export function quotaPercent(usage: number | null | undefined, limit: number | null | undefined): number {
  if (!usage || !limit || limit <= 0) return 0;
  return Math.min(100, Math.max(0, (usage / limit) * 100));
}
