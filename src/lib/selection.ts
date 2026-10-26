/**
 * Selection is kept as a list of ids in table order, not a Set, so shift-range
 * math and the status bar stay deterministic. Pure functions only: the store
 * holds the result, tests can call these directly.
 */
export interface ClickModifiers {
  toggle: boolean;
  range: boolean;
}

export function applySelection(
  selected: string[],
  anchor: string | null,
  order: string[],
  id: string,
  modifiers: ClickModifiers,
): { selected: string[]; anchor: string } {
  if (modifiers.range && anchor) {
    const from = order.indexOf(anchor);
    const to = order.indexOf(id);
    if (from !== -1 && to !== -1) {
      const [start, end] = from <= to ? [from, to] : [to, from];
      const range = order.slice(start, end + 1);
      return { selected: range, anchor };
    }
  }
  if (modifiers.toggle) {
    return {
      selected: selected.includes(id) ? selected.filter((value) => value !== id) : [...selected, id],
      anchor: id,
    };
  }
  return { selected: [id], anchor: id };
}

/** Drop ids that disappeared after a reload; keep the surviving order. */
export function pruneSelection(selected: string[], available: string[]): string[] {
  const present = new Set(available);
  return selected.filter((id) => present.has(id));
}

export function toggleSelectAll(selected: string[], order: string[]): string[] {
  return selected.length === order.length && order.every((id) => selected.includes(id)) ? [] : [...order];
}
