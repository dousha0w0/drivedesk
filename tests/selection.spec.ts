import { describe, expect, it } from "vitest";
import { applySelection, pruneSelection, toggleSelectAll } from "@/lib/selection";

const ORDER = ["a", "b", "c", "d", "e"];

describe("applySelection", () => {
  it("replaces the selection on a plain click", () => {
    expect(applySelection(["a", "b"], "a", ORDER, "c", { toggle: false, range: false })).toEqual({
      selected: ["c"],
      anchor: "c",
    });
  });

  it("toggles one row with cmd/ctrl and keeps the anchor movable", () => {
    const first = applySelection(["a"], "a", ORDER, "c", { toggle: true, range: false });
    expect(first).toEqual({ selected: ["a", "c"], anchor: "c" });
    expect(applySelection(first.selected, first.anchor, ORDER, "a", { toggle: true, range: false })).toEqual({
      selected: ["c"],
      anchor: "a",
    });
  });

  it("selects an inclusive range in both directions from the anchor", () => {
    expect(applySelection([], "b", ORDER, "d", { toggle: false, range: true }).selected).toEqual(["b", "c", "d"]);
    expect(applySelection([], "d", ORDER, "b", { toggle: false, range: true }).selected).toEqual(["b", "c", "d"]);
  });

  it("falls back to a single click when shift has no anchor", () => {
    expect(applySelection([], null, ORDER, "c", { toggle: false, range: true })).toEqual({
      selected: ["c"],
      anchor: "c",
    });
  });

  it("ignores ids that are not in the current order", () => {
    expect(applySelection(["a"], "missing", ORDER, "c", { toggle: false, range: true }).selected).toEqual(["c"]);
  });
});

describe("pruneSelection", () => {
  it("drops ids that disappeared and preserves the surviving order", () => {
    expect(pruneSelection(["c", "a", "z"], ["a", "b", "c"])).toEqual(["c", "a"]);
  });
});

describe("toggleSelectAll", () => {
  it("selects everything, then clears when everything is already selected", () => {
    expect(toggleSelectAll([], ORDER)).toEqual(ORDER);
    expect(toggleSelectAll(ORDER, ORDER)).toEqual([]);
    expect(toggleSelectAll(["a"], ORDER)).toEqual(ORDER);
  });
});
