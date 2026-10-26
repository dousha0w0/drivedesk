import { ref } from "vue";

export interface ResizeOptions {
  axis: "x" | "y";
  /** +1 when dragging right/down grows the panel, -1 when it shrinks it. */
  direction: 1 | -1;
  current: () => number;
  min: number;
  max: () => number;
  onChange: (value: number) => void;
  onCommit: (value: number) => void;
}

/**
 * Pointer-driven panel resize with a single document-level listener pair.
 * Moves only update the caller's live value; the persisted value is committed
 * once on pointerup, so localStorage is not written on every frame.
 */
export function useResizeHandle(options: ResizeOptions) {
  const dragging = ref(false);

  function onPointerDown(event: PointerEvent): void {
    event.preventDefault();
    event.stopPropagation();
    const startPosition = options.axis === "x" ? event.clientX : event.clientY;
    const startValue = options.current();
    let latest = startValue;
    dragging.value = true;
    document.body.classList.add("is-resizing");

    const onMove = (moveEvent: PointerEvent) => {
      const delta = (options.axis === "x" ? moveEvent.clientX : moveEvent.clientY) - startPosition;
      latest = Math.min(options.max(), Math.max(options.min, Math.round(startValue + delta * options.direction)));
      options.onChange(latest);
    };
    const onUp = () => {
      dragging.value = false;
      document.body.classList.remove("is-resizing");
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      options.onCommit(latest);
    };

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  }

  return { dragging, onPointerDown };
}
