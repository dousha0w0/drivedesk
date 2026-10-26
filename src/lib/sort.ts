import type { FileMeta, SortDirection, SortKey } from "@/types/drive";

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });

/**
 * Sort with folders pinned first. The same ordering applies to the remote
 * response, so re-sorting locally never disagrees with a reload.
 */
export function sortEntries(files: FileMeta[], key: SortKey, direction: SortDirection): FileMeta[] {
  const sign = direction === "desc" ? -1 : 1;
  return [...files].sort((left, right) => {
    if (left.iconType === "folder" !== (right.iconType === "folder")) {
      return left.iconType === "folder" ? -1 : 1;
    }
    let result = 0;
    switch (key) {
      case "size":
        result = (left.size ?? -1) - (right.size ?? -1);
        break;
      case "modified":
        result = (left.modifiedTime ?? "").localeCompare(right.modifiedTime ?? "");
        break;
      case "type":
        result = collator.compare(left.mimeType, right.mimeType);
        break;
      default:
        result = collator.compare(left.name, right.name);
    }
    return result === 0 ? collator.compare(left.name, right.name) : result * sign;
  });
}

/** Cycle a column: ascending -> descending -> back to ascending. */
export function toggleSort(current: { key: SortKey; direction: SortDirection }, key: SortKey): {
  key: SortKey;
  direction: SortDirection;
} {
  if (current.key !== key) return { key, direction: key === "name" ? "asc" : "desc" };
  return { key, direction: current.direction === "asc" ? "desc" : "asc" };
}

export function matchesFilter(file: FileMeta, filter: string): boolean {
  const needle = filter.trim().toLowerCase();
  if (!needle) return true;
  return file.name.toLowerCase().includes(needle) || file.mimeType.toLowerCase().includes(needle);
}
