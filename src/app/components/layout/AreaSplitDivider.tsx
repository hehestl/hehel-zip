interface Props {
  direction: "row" | "col";
  onDrag: (deltaPx: number, containerSize: number) => void;
}

export function AreaSplitDivider({ direction, onDrag }: Props) {
  return (
    <div
      className={`shrink-0 bg-hh-border hover:bg-hh-accent/40 ${
        direction === "row"
          ? "w-1 cursor-col-resize"
          : "h-1 cursor-row-resize"
      }`}
      onPointerDown={(e) => {
        const start = direction === "row" ? e.clientX : e.clientY;
        const parent = e.currentTarget.parentElement;
        if (!parent) return;
        const size =
          direction === "row" ? parent.clientWidth : parent.clientHeight;
        const move = (ev: PointerEvent) => {
          const cur = direction === "row" ? ev.clientX : ev.clientY;
          onDrag(cur - start, size);
        };
        const up = () => {
          window.removeEventListener("pointermove", move);
          window.removeEventListener("pointerup", up);
        };
        window.addEventListener("pointermove", move);
        window.addEventListener("pointerup", up);
      }}
    />
  );
}
