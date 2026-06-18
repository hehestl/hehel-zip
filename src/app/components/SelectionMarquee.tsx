import type { MarqueeState } from "../hooks/useMarqueeSelection";

interface Props {
  state: MarqueeState | null;
}

export function SelectionMarquee({ state }: Props) {
  if (!state?.active) return null;

  const left = Math.min(state.startX, state.currentX);
  const top = Math.min(state.startY, state.currentY);
  const width = Math.abs(state.currentX - state.startX);
  const height = Math.abs(state.currentY - state.startY);

  return (
    <div
      style={{
        position: "fixed",
        left,
        top,
        width,
        height,
        border: "1px dashed #2659FF",
        backgroundColor: "rgba(38,89,255,0.15)",
        pointerEvents: "none",
        zIndex: 9999,
      }}
    />
  );
}
