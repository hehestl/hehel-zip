import type { AreaNode, EditorMode } from "../types";

export const MIN_LEAF_PX = 120;

export function defaultLayout(): AreaNode {
  return { kind: "leaf", id: crypto.randomUUID(), mode: "archive" };
}

export function parseAreaLayout(raw: unknown): AreaNode {
  if (!raw || typeof raw !== "object") return defaultLayout();
  return validateNode(raw as AreaNode) ?? defaultLayout();
}

function validateNode(node: AreaNode): AreaNode | null {
  if (node.kind === "leaf") {
    if (!node.id || !isMode(node.mode)) return null;
    return node;
  }
  if (node.kind === "split") {
    if (!node.id || (node.direction !== "row" && node.direction !== "col")) {
      return null;
    }
    const a = validateNode(node.a);
    const b = validateNode(node.b);
    if (!a || !b) return null;
    return {
      ...node,
      ratio: clampRatio(node.ratio),
      a,
      b,
    };
  }
  return null;
}

function isMode(v: string): v is EditorMode {
  return ["archive", "images", "metadata", "history"].includes(v);
}

export function clampRatio(ratio: number): number {
  return Math.min(0.95, Math.max(0.05, ratio));
}

export function findLeaf(root: AreaNode, id: string): AreaNode | null {
  if (root.kind === "leaf") return root.id === id ? root : null;
  return findLeaf(root.a, id) ?? findLeaf(root.b, id);
}

function findParent(
  root: AreaNode,
  leafId: string,
): { parent: Extract<AreaNode, { kind: "split" }>; side: "a" | "b" } | null {
  if (root.kind === "leaf") return null;
  if (root.a.kind === "leaf" && root.a.id === leafId) {
    return { parent: root, side: "a" };
  }
  if (root.b.kind === "leaf" && root.b.id === leafId) {
    return { parent: root, side: "b" };
  }
  return findParent(root.a, leafId) ?? findParent(root.b, leafId);
}

export function canJoin(root: AreaNode, aId: string, bId: string): boolean {
  if (aId === bId) return false;
  const pa = findParent(root, aId);
  const pb = findParent(root, bId);
  if (!pa || !pb) return false;
  return pa.parent.id === pb.parent.id && pa.side !== pb.side;
}

export function splitLeaf(
  root: AreaNode,
  leafId: string,
  direction: "row" | "col",
): AreaNode {
  const replace = (node: AreaNode): AreaNode => {
    if (node.kind === "leaf" && node.id === leafId) {
      return {
        kind: "split",
        id: crypto.randomUUID(),
        direction,
        ratio: 0.5,
        a: node,
        b: { kind: "leaf", id: crypto.randomUUID(), mode: node.mode },
      };
    }
    if (node.kind === "split") {
      return { ...node, a: replace(node.a), b: replace(node.b) };
    }
    return node;
  };
  return replace(root);
}

export function joinLeaves(
  root: AreaNode,
  sourceId: string,
  targetId: string,
): AreaNode {
  if (!canJoin(root, sourceId, targetId)) return root;
  const replace = (node: AreaNode): AreaNode => {
    if (node.kind === "split") {
      const aLeaf = node.a.kind === "leaf" ? node.a : null;
      const bLeaf = node.b.kind === "leaf" ? node.b : null;
      if (aLeaf && bLeaf) {
        const ids = [aLeaf.id, bLeaf.id];
        if (ids.includes(sourceId) && ids.includes(targetId)) {
          return targetId === aLeaf.id ? aLeaf : bLeaf;
        }
      }
      return { ...node, a: replace(node.a), b: replace(node.b) };
    }
    return node;
  };
  return replace(root);
}

export function swapModes(root: AreaNode, aId: string, bId: string): AreaNode {
  const a = findLeaf(root, aId);
  const b = findLeaf(root, bId);
  if (!a || !b || a.kind !== "leaf" || b.kind !== "leaf") return root;
  return mapLeaves(root, (leaf) => {
    if (leaf.id === aId) return { ...leaf, mode: b.mode };
    if (leaf.id === bId) return { ...leaf, mode: a.mode };
    return leaf;
  });
}

export function setMode(root: AreaNode, leafId: string, mode: EditorMode): AreaNode {
  return mapLeaves(root, (leaf) =>
    leaf.id === leafId ? { ...leaf, mode } : leaf,
  );
}

export function resizeSplit(
  root: AreaNode,
  splitId: string,
  ratio: number,
): AreaNode {
  if (root.kind === "split" && root.id === splitId) {
    return { ...root, ratio: clampRatio(ratio) };
  }
  if (root.kind === "split") {
    return {
      ...root,
      a: resizeSplit(root.a, splitId, ratio),
      b: resizeSplit(root.b, splitId, ratio),
    };
  }
  return root;
}

function mapLeaves(
  node: AreaNode,
  fn: (leaf: Extract<AreaNode, { kind: "leaf" }>) => Extract<
    AreaNode,
    { kind: "leaf" }
  >,
): AreaNode {
  if (node.kind === "leaf") return fn(node);
  return { ...node, a: mapLeaves(node.a, fn), b: mapLeaves(node.b, fn) };
}

export function collectLeaves(root: AreaNode): Extract<AreaNode, { kind: "leaf" }>[] {
  if (root.kind === "leaf") return [root];
  return [...collectLeaves(root.a), ...collectLeaves(root.b)];
}

export function canRemoveLeaf(root: AreaNode): boolean {
  return collectLeaves(root).length > 1;
}

export function removeLeaf(node: AreaNode, leafId: string): AreaNode {
  if (node.kind === "leaf") {
    return node;
  }

  if (node.a.kind === "leaf" && node.a.id === leafId) return node.b;
  if (node.b.kind === "leaf" && node.b.id === leafId) return node.a;

  const a = removeLeaf(node.a, leafId);
  const b = removeLeaf(node.b, leafId);
  if (a === node.a && b === node.b) return node;
  return { ...node, a, b };
}
