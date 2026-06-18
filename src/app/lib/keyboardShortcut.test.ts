import { describe, expect, it } from "vitest";
import { isModifiedKey } from "./keyboardShortcut";

function keyEvent(
  code: string,
  opts: { key?: string; ctrlKey?: boolean; metaKey?: boolean; shiftKey?: boolean },
): KeyboardEvent {
  return {
    code,
    key: opts.key ?? code,
    ctrlKey: opts.ctrlKey ?? false,
    metaKey: opts.metaKey ?? false,
    shiftKey: opts.shiftKey ?? false,
    altKey: false,
  } as KeyboardEvent;
}

describe("isModifiedKey", () => {
  it("matches Ctrl+A by physical key (Russian layout key is ф)", () => {
    expect(isModifiedKey(keyEvent("KeyA", { key: "ф", ctrlKey: true }), "KeyA")).toBe(
      true,
    );
  });

  it("rejects Shift+Ctrl+A", () => {
    expect(
      isModifiedKey(keyEvent("KeyA", { key: "a", ctrlKey: true, shiftKey: true }), "KeyA"),
    ).toBe(false);
  });

  it("matches Meta+A", () => {
    expect(isModifiedKey(keyEvent("KeyA", { key: "a", metaKey: true }), "KeyA")).toBe(
      true,
    );
  });
});
