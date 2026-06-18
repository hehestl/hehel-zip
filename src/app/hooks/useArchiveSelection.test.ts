import { describe, expect, it } from "vitest";
import { applyShiftRange } from "./useArchiveSelection";

describe("applyShiftRange", () => {
  const entries = [
    { path: "1.txt" },
    { path: "2.txt" },
    { path: "3.txt" },
    { path: "4.txt" },
  ];

  it("creates range with given anchor", () => {
    const result = applyShiftRange(entries, 0, 2, new Set(), false);
    expect([...result]).toEqual(["1.txt", "2.txt", "3.txt"]);
  });

  it("uses fallback anchor from prevSelected when anchor is null", () => {
    const prevSelected = new Set(["2.txt"]);
    const result = applyShiftRange(entries, null, 3, prevSelected, false);
    expect([...result]).toEqual(["2.txt", "3.txt", "4.txt"]);
  });

  it("unions range when Ctrl is pressed", () => {
    const prevSelected = new Set(["1.txt"]);
    const result = applyShiftRange(entries, 2, 3, prevSelected, true);
    expect(result.has("1.txt")).toBe(true);
    expect(result.has("3.txt")).toBe(true);
    expect(result.has("4.txt")).toBe(true);
  });
});
