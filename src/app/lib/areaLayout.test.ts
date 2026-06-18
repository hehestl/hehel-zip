import { describe, expect, it } from "vitest";
import {
  canJoin,
  canRemoveLeaf,
  clampRatio,
  collectLeaves,
  defaultLayout,
  joinLeaves,
  parseAreaLayout,
  removeLeaf,
  resizeSplit,
  setMode,
  splitLeaf,
  swapModes,
} from "./areaLayout";

describe("areaLayout", () => {
  it("split keeps original leaf id", () => {
    const root = defaultLayout();
    const id = root.kind === "leaf" ? root.id : "";
    const split = splitLeaf(root, id, "row");
    expect(split.kind).toBe("split");
    if (split.kind === "split") {
      expect(split.a.kind).toBe("leaf");
      if (split.a.kind === "leaf") expect(split.a.id).toBe(id);
    }
  });

  it("rejects join for non-siblings", () => {
    let root = defaultLayout();
    const id = root.kind === "leaf" ? root.id : "";
    root = splitLeaf(root, id, "row");
    if (root.kind !== "split") throw new Error("expected split");
    const left = root.a.kind === "leaf" ? root.a.id : "";
    root = splitLeaf(root, left, "col");
    const leaves = root.kind === "split" ? [root.a, root.b] : [];
    const deepLeaf =
      leaves[0]?.kind === "split" && leaves[0].a.kind === "leaf"
        ? leaves[0].a.id
        : "";
    const sibling =
      leaves[0]?.kind === "split" && leaves[0].b.kind === "leaf"
        ? leaves[0].b.id
        : "";
    const far =
      root.kind === "split" && root.b.kind === "leaf" ? root.b.id : "";
    expect(canJoin(root, deepLeaf, sibling)).toBe(true);
    expect(canJoin(root, deepLeaf, far)).toBe(false);
    expect(joinLeaves(root, deepLeaf, far)).toBe(root);
  });

  it("swap modes", () => {
    let root = defaultLayout();
    const id = root.kind === "leaf" ? root.id : "";
    root = splitLeaf(root, id, "row");
    if (root.kind !== "split") throw new Error("split");
    const a = root.a.kind === "leaf" ? root.a.id : "";
    const b = root.b.kind === "leaf" ? root.b.id : "";
    root = setMode(root, a, "images");
    root = swapModes(root, a, b);
    if (root.kind !== "split") throw new Error("split");
    expect(root.a.kind === "leaf" && root.a.mode).toBe("archive");
    expect(root.b.kind === "leaf" && root.b.mode).toBe("images");
  });

  it("resize extremes", () => {
    let root = defaultLayout();
    const id = root.kind === "leaf" ? root.id : "";
    root = splitLeaf(root, id, "row");
    if (root.kind !== "split") throw new Error("split");
    const splitId = root.id;
    root = resizeSplit(root, splitId, 0.01);
    if (root.kind === "split") expect(root.ratio).toBe(0.05);
    root = resizeSplit(root, splitId, 0.99);
    if (root.kind === "split") expect(root.ratio).toBe(0.95);
    expect(clampRatio(2)).toBe(0.95);
  });

  it("parseAreaLayout invalid fallback", () => {
    const parsed = parseAreaLayout({ kind: "leaf", id: "", mode: "nope" });
    expect(parsed.kind).toBe("leaf");
    expect(parsed.kind === "leaf" && parsed.mode).toBe("archive");
  });

  it("canRemoveLeaf false for single leaf", () => {
    const root = defaultLayout();
    expect(canRemoveLeaf(root)).toBe(false);
  });

  it("removeLeaf drops direct sibling", () => {
    let root = defaultLayout();
    const id = root.kind === "leaf" ? root.id : "";
    root = splitLeaf(root, id, "row");
    if (root.kind !== "split") throw new Error("expected split");
    const keepId = root.b.kind === "leaf" ? root.b.id : "";
    const removed = removeLeaf(root, id);
    expect(removed.kind).toBe("leaf");
    if (removed.kind === "leaf") expect(removed.id).toBe(keepId);
    expect(collectLeaves(removed)).toHaveLength(1);
  });

  it("removeLeaf from nested split promotes sibling subtree", () => {
    let root = defaultLayout();
    const rootId = root.kind === "leaf" ? root.id : "";
    root = splitLeaf(root, rootId, "row");
    if (root.kind !== "split") throw new Error("expected split");
    const leftId = root.a.kind === "leaf" ? root.a.id : "";
    root = splitLeaf(root, leftId, "col");
    if (root.kind !== "split") throw new Error("expected split");
    const inner =
      root.a.kind === "split"
        ? root.a
        : (() => {
            throw new Error("expected inner split");
          })();
    const deepLeaf = inner.a.kind === "leaf" ? inner.a.id : "";
    const deepSibling = inner.b.kind === "leaf" ? inner.b.id : "";
    const result = removeLeaf(root, deepLeaf);
    expect(canRemoveLeaf(result)).toBe(true);
    const leaves = collectLeaves(result);
    expect(leaves.some((l) => l.id === deepSibling)).toBe(true);
    expect(leaves.some((l) => l.id === deepLeaf)).toBe(false);
  });
});
