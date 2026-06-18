import {
  useEffect,
  useRef,
  useState,
  type RefObject,
} from "react";

export interface MarqueeState {
  startX: number;
  startY: number;
  currentX: number;
  currentY: number;
  active: boolean;
}

interface DragRef {
  startX: number;
  startY: number;
  currentX: number;
  currentY: number;
  active: boolean;
  ctrlKey: boolean;
  drawing: boolean;
}

export function rectsIntersect(r1: DOMRect, r2: DOMRect): boolean {
  return !(
    r2.left > r1.right ||
    r2.right < r1.left ||
    r2.top > r1.bottom ||
    r2.bottom < r1.top
  );
}

export function indicesInRect(rows: HTMLElement[], rect: DOMRect): number[] {
  return rows
    .filter((el) => rectsIntersect(el.getBoundingClientRect(), rect))
    .map((el) => Number(el.dataset.entryIndex))
    .filter((idx) => !Number.isNaN(idx));
}

const MARQUEE_THRESHOLD_PX = 3;

export function useMarqueeSelection(
  containerRef: RefObject<HTMLElement | null>,
  entries: { path: string }[],
  prevSelected: Set<string>,
  setSelected: (s: Set<string>) => void,
  setAnchorIndex: (i: number | null) => void,
  disabled: boolean,
): MarqueeState | null {
  const [marquee, setMarquee] = useState<MarqueeState | null>(null);
  const dragRef = useRef<DragRef | null>(null);
  const prevSelectedRef = useRef(prevSelected);
  prevSelectedRef.current = prevSelected;

  useEffect(() => {
    if (disabled) {
      dragRef.current = null;
      setMarquee(null);
      return;
    }

    const container = containerRef.current;
    if (!container) return;

    const onPointerDown = (e: PointerEvent) => {
      if (e.target !== container || e.button !== 0) return;

      dragRef.current = {
        startX: e.clientX,
        startY: e.clientY,
        currentX: e.clientX,
        currentY: e.clientY,
        active: false,
        ctrlKey: e.ctrlKey || e.metaKey,
        drawing: true,
      };
      setMarquee({
        startX: e.clientX,
        startY: e.clientY,
        currentX: e.clientX,
        currentY: e.clientY,
        active: false,
      });
    };

    const onPointerMove = (e: PointerEvent) => {
      const drag = dragRef.current;
      if (!drag?.drawing) return;

      drag.currentX = e.clientX;
      drag.currentY = e.clientY;

      const distance = Math.hypot(
        e.clientX - drag.startX,
        e.clientY - drag.startY,
      );
      if (!drag.active && distance > MARQUEE_THRESHOLD_PX) {
        drag.active = true;
      }

      if (drag.active) {
        setMarquee({
          startX: drag.startX,
          startY: drag.startY,
          currentX: drag.currentX,
          currentY: drag.currentY,
          active: true,
        });
      }
    };

    const onPointerUp = () => {
      const drag = dragRef.current;
      if (!drag?.drawing) return;

      if (drag.active) {
        const marqueeRect = new DOMRect(
          Math.min(drag.startX, drag.currentX),
          Math.min(drag.startY, drag.currentY),
          Math.abs(drag.currentX - drag.startX),
          Math.abs(drag.currentY - drag.startY),
        );

        const rows = Array.from(
          container.querySelectorAll<HTMLElement>("[data-entry-index]"),
        );
        const hitIndices = indicesInRect(rows, marqueeRect);

        const next = drag.ctrlKey
          ? new Set(prevSelectedRef.current)
          : new Set<string>();
        for (const idx of hitIndices) {
          const entry = entries[idx];
          if (entry) next.add(entry.path);
        }

        setSelected(next);
        setAnchorIndex(hitIndices.length > 0 ? hitIndices[0] : null);
      } else {
        setSelected(new Set());
        setAnchorIndex(null);
      }

      dragRef.current = null;
      setMarquee(null);
    };

    container.addEventListener("pointerdown", onPointerDown);
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);

    return () => {
      container.removeEventListener("pointerdown", onPointerDown);
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
    };
  }, [containerRef, disabled, entries, setAnchorIndex, setSelected]);

  return marquee;
}
