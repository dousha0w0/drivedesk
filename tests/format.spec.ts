import { describe, expect, it } from "vitest";
import { formatBytes, formatDate, formatDuration, extensionOf, quotaPercent, eta } from "@/lib/format";

describe("formatBytes", () => {
  it("uses binary units and sensible precision", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(999)).toBe("999 B");
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(1024 * 1024 * 3.5)).toBe("3.5 MB");
    expect(formatBytes(1024 ** 3 * 120)).toBe("120 GB");
    expect(formatBytes(null)).toBe("—");
  });
});

describe("formatDate", () => {
  const now = new Date("2026-10-26T12:00:00");

  it("summarises today, yesterday and older dates", () => {
    expect(formatDate("2026-10-26T09:30:00", now)).toBe("09:30");
    expect(formatDate("2026-10-25T21:05:00", now)).toBe("昨天 21:05");
    expect(formatDate("2025-03-04T08:00:00", now)).toBe("2025/3/4 08:00");
    expect(formatDate(null, now)).toBe("—");
  });
});

describe("quotaPercent", () => {
  it("clamps to 0..100 and survives missing values", () => {
    expect(quotaPercent(50, 100)).toBe(50);
    expect(quotaPercent(200, 100)).toBe(100);
    expect(quotaPercent(null, 100)).toBe(0);
    expect(quotaPercent(50, 0)).toBe(0);
  });
});

describe("eta", () => {
  it("derives a duration from the current rate", () => {
    expect(eta({ received: 5_000, total: 10_000, speedBps: 2_500 })).toBe("2s");
    expect(eta({ received: 10_000, total: 10_000, speedBps: 0 })).toBe("");
    expect(eta({ received: 0, total: null, speedBps: 10 })).toBe("");
  });
});

describe("formatDuration", () => {
  it("scales from seconds to hours", () => {
    expect(formatDuration(1_500)).toBe("2s");
    expect(formatDuration(90_000)).toBe("1m 30s");
    expect(formatDuration(3_720_000)).toBe("1h 2m");
  });
});

describe("extensionOf", () => {
  it("returns a lowercase extension or empty when there is none", () => {
    expect(extensionOf("Report.PDF")).toBe("pdf");
    expect(extensionOf("archive.tar.gz")).toBe("gz");
    expect(extensionOf("README")).toBe("");
    expect(extensionOf(".env")).toBe("");
  });
});
