import { describe, expect, it } from "vitest";
import { indicesInRect, rectsIntersect } from "./useMarqueeSelection";

function rect(x: number, y: number, w: number, h: number): DOMRect {
  return {
    x,
    y,
    width: w,
    height: h,
    left: x,
    top: y,
    right: x + w,
    bottom: y + h,
    toJSON: () => ({}),
  } as DOMRect;
}

describe("Marquee pure functions", () => {
  it("rectsIntersect detects overlap", () => {
    expect(rectsIntersect(rect(0, 0, 100, 100), rect(50, 50, 100, 100))).toBe(
      true,
    );
    expect(rectsIntersect(rect(0, 0, 100, 100), rect(200, 200, 50, 50))).toBe(
      false,
    );
  });

  it("indicesInRect reads data-entry-index", () => {
    const createMockRow = (index: number, r: DOMRect) =>
      ({
        dataset: { entryIndex: String(index) },
        getBoundingClientRect: () => r,
      }) as unknown as HTMLElement;

    const rows = [
      createMockRow(0, rect(0, 0, 100, 20)),
      createMockRow(1, rect(0, 20, 100, 20)),
      createMockRow(2, rect(0, 40, 100, 20)),
    ];

    expect(indicesInRect(rows, rect(0, 15, 100, 10))).toEqual([0, 1]);
  });
});
