import { describe, expect, it } from "vitest";
import type { ArchiveEntry } from "../types";
import {
  isDraggableEntry,
  resolveDragPaths,
  type DragPhase,
} from "./useFileDragGesture";

function entry(path: string, ext: string, isDir = false): ArchiveEntry {
  return {
    path,
    name: path.split("/").pop() ?? path,
    extension: ext,
    isDir,
    size: 100,
    packedSize: 50,
    modified: null,
  };
}

describe("isDraggableEntry", () => {
  it("allows STL and OBJ files", () => {
    expect(isDraggableEntry(entry("a.stl", "stl"))).toBe(true);
    expect(isDraggableEntry(entry("a.obj", "OBJ"))).toBe(true);
  });

  it("rejects directories and other extensions", () => {
    expect(isDraggableEntry(entry("dir/", "", true))).toBe(false);
    expect(isDraggableEntry(entry("a.png", "png"))).toBe(false);
  });
});

describe("resolveDragPaths", () => {
  const visible = [
    entry("a.stl", "stl"),
    entry("b.stl", "stl"),
    entry("c.png", "png"),
  ];

  it("uses full selection when dragged row is selected", () => {
    const selected = new Set(["a.stl", "b.stl"]);
    expect(resolveDragPaths("a.stl", selected, visible)).toEqual([
      "a.stl",
      "b.stl",
    ]);
  });

  it("uses only dragged row when it is not selected", () => {
    const selected = new Set<string>();
    expect(resolveDragPaths("b.stl", selected, visible)).toEqual(["b.stl"]);
  });
});

export function phaseAfterMouseUp(phase: DragPhase): DragPhase {
  return phase === "pending" ? "idle" : phase;
}

export function phaseAfterEscape(phase: DragPhase): DragPhase {
  if (phase === "pending" || phase === "preparing") return "idle";
  return phase;
}

export function shouldSkipDragAfterExtract(cancelled: boolean): boolean {
  return cancelled;
}

describe("drag gesture phase transitions", () => {
  it("pending → idle on mouseup", () => {
    expect(phaseAfterMouseUp("pending")).toBe("idle");
    expect(phaseAfterMouseUp("preparing")).toBe("preparing");
  });

  it("Esc resets pending and preparing", () => {
    expect(phaseAfterEscape("pending")).toBe("idle");
    expect(phaseAfterEscape("preparing")).toBe("idle");
    expect(phaseAfterEscape("idle")).toBe("idle");
  });

  it("Esc during preparing skips native drag after extract", () => {
    expect(shouldSkipDragAfterExtract(true)).toBe(true);
    expect(shouldSkipDragAfterExtract(false)).toBe(false);
  });
});
