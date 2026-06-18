export type CloseDirection = "left" | "right" | "up" | "down";

export function resolveCloseDirection(dx: number, dy: number): CloseDirection {
  if (Math.abs(dx) >= Math.abs(dy)) {
    return dx > 0 ? "right" : "left";
  }
  return dy > 0 ? "down" : "up";
}
