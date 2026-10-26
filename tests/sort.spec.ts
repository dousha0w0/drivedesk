import { describe, expect, it } from "vitest";
import { matchesFilter, sortEntries, toggleSort } from "@/lib/sort";
import type { FileMeta } from "@/types/drive";

function file(name: string, overrides: Partial<FileMeta> = {}): FileMeta {
  return {
    id: name,
    name,
    mimeType: "image/png",
    size: 100,
    modifiedTime: "2026-01-01T00:00:00.000Z",
    createdTime: "2026-01-01T00:00:00.000Z",
    owner: null,
    ownerEmail: null,
    parents: [],
    starred: false,
    trashed: false,
    md5: null,
    webViewLink: null,
    thumbnailLink: null,
    iconType: "image",
    shared: false,
    canEdit: true,
    canDownload: true,
    ...overrides,
  };
}

describe("sortEntries", () => {
  it("keeps folders first and sorts names naturally", () => {
    const folder = file("zulu", { iconType: "folder", mimeType: "application/vnd.google-apps.folder" });
    const sorted = sortEntries([file("img10.png"), folder, file("img2.png")], "name", "asc");
    expect(sorted.map((entry) => entry.name)).toEqual(["zulu", "img2.png", "img10.png"]);
  });

  it("sorts by size descending and reverses name order", () => {
    const bySize = sortEntries([file("a", { size: 1 }), file("b", { size: 900 }), file("c", { size: 40 })], "size", "desc");
    expect(bySize.map((entry) => entry.name)).toEqual(["b", "c", "a"]);

    const byName = sortEntries([file("a"), file("c"), file("b")], "name", "desc");
    expect(byName.map((entry) => entry.name)).toEqual(["c", "b", "a"]);
  });

  it("orders by modified time and breaks ties by name", () => {
    const sorted = sortEntries(
      [
        file("old", { modifiedTime: "2025-01-01T00:00:00.000Z" }),
        file("new-b", { modifiedTime: "2026-05-01T00:00:00.000Z" }),
        file("new-a", { modifiedTime: "2026-05-01T00:00:00.000Z" }),
      ],
      "modified",
      "desc",
    );
    expect(sorted.map((entry) => entry.name)).toEqual(["new-a", "new-b", "old"]);
  });

  it("does not mutate the input array", () => {
    const input = [file("b"), file("a")];
    sortEntries(input, "name", "asc");
    expect(input.map((entry) => entry.name)).toEqual(["b", "a"]);
  });
});

describe("toggleSort", () => {
  it("cycles the current column and starts new columns descending", () => {
    expect(toggleSort({ key: "name", direction: "asc" }, "size")).toEqual({ key: "size", direction: "desc" });
    expect(toggleSort({ key: "size", direction: "desc" }, "size")).toEqual({ key: "size", direction: "asc" });
    expect(toggleSort({ key: "size", direction: "asc" }, "name")).toEqual({ key: "name", direction: "asc" });
  });
});

describe("matchesFilter", () => {
  it("matches names and mime types case-insensitively", () => {
    const entry = file("Budget.XLSX", { mimeType: "application/vnd.ms-excel" });
    expect(matchesFilter(entry, "budget")).toBe(true);
    expect(matchesFilter(entry, "ms-excel")).toBe(true);
    expect(matchesFilter(entry, "photo")).toBe(false);
    expect(matchesFilter(entry, "   ")).toBe(true);
  });
});
