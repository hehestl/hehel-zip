import { describe, expect, test } from "vitest";
import { resolveCloseDirection } from "./areaGripClose";

describe("resolveCloseDirection", () => {
  test("resolves horizontal right swipe dominantly", () => {
    expect(resolveCloseDirection(12, -3)).toBe("right");
    expect(resolveCloseDirection(50, 49)).toBe("right");
  });

  test("resolves horizontal left swipe dominantly", () => {
    expect(resolveCloseDirection(-20, 5)).toBe("left");
  });

  test("resolves vertical up swipe dominantly", () => {
    expect(resolveCloseDirection(2, -15)).toBe("up");
  });

  test("resolves vertical down swipe dominantly", () => {
    expect(resolveCloseDirection(-10, 30)).toBe("down");
  });
});
