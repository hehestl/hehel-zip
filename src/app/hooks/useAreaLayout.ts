import { useCallback, useState } from "react";
import type { AreaNode, EditorMode } from "../types";
import {
  canRemoveLeaf,
  collectLeaves,
  defaultLayout,
  joinLeaves,
  parseAreaLayout,
  removeLeaf,
  resizeSplit,
  setMode,
  splitLeaf,
  swapModes,
} from "../lib/areaLayout";

export function useAreaLayout(
  stored: AreaNode | undefined,
  onPersist: (layout: AreaNode) => void,
) {
  const [layout, setLayout] = useState<AreaNode>(() =>
    parseAreaLayout(stored ?? defaultLayout()),
  );
  const [focusedLeafId, setFocusedLeafId] = useState(() => {
    const l = parseAreaLayout(stored ?? defaultLayout());
    return collectLeaves(l)[0]?.id ?? "";
  });

  const commit = useCallback(
    (next: AreaNode | ((prev: AreaNode) => AreaNode)) => {
      setLayout((prev) => {
        const resolved = typeof next === "function" ? next(prev) : next;
        onPersist(resolved);
        return resolved;
      });
    },
    [onPersist],
  );

  const split = useCallback(
    (leafId: string, direction: "row" | "col") =>
      commit((prev) => splitLeaf(prev, leafId, direction)),
    [commit],
  );

  const join = useCallback(
    (sourceId: string, targetId: string) =>
      commit((prev) => joinLeaves(prev, sourceId, targetId)),
    [commit],
  );

  const swap = useCallback(
    (aId: string, bId: string) => commit((prev) => swapModes(prev, aId, bId)),
    [commit],
  );

  const resize = useCallback(
    (splitId: string, ratio: number) =>
      commit((prev) => resizeSplit(prev, splitId, ratio)),
    [commit],
  );

  const setLeafMode = useCallback(
    (leafId: string, mode: EditorMode) =>
      commit((prev) => setMode(prev, leafId, mode)),
    [commit],
  );

  const setFocusedMode = useCallback(
    (mode: EditorMode) => {
      if (!focusedLeafId) return;
      commit((prev) => setMode(prev, focusedLeafId, mode));
    },
    [commit, focusedLeafId],
  );

  const removePanel = useCallback(
    (leafId: string) => {
      commit((prev) => {
        if (!canRemoveLeaf(prev)) return prev;
        const next = removeLeaf(prev, leafId);
        setFocusedLeafId((current) => {
          if (current !== leafId) return current;
          return collectLeaves(next)[0]?.id ?? "";
        });
        return next;
      });
    },
    [commit],
  );

  return {
    layout,
    focusedLeafId,
    setFocusedLeafId,
    split,
    join,
    swap,
    resize,
    setLeafMode,
    setFocusedMode,
    removePanel,
    canRemovePanel: canRemoveLeaf(layout),
  };
}
