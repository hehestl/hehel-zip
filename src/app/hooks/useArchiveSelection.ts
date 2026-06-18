import { useCallback, useState } from "react";
import type { ArchiveEntry } from "../types";

export interface SelectionClick {
  index: number;
  path: string;
  ctrlKey: boolean;
  shiftKey: boolean;
}

export function applyShiftRange(
  entries: { path: string }[],
  anchorIndex: number | null,
  currentIndex: number,
  prevSelected: Set<string>,
  ctrlKey: boolean,
): Set<string> {
  let startAnchor = anchorIndex;

  if (startAnchor === null && prevSelected.size > 0) {
    startAnchor = entries.findIndex((e) => prevSelected.has(e.path));
    if (startAnchor === -1) startAnchor = currentIndex;
  } else if (startAnchor === null) {
    startAnchor = currentIndex;
  }

  const start = Math.min(startAnchor, currentIndex);
  const end = Math.max(startAnchor, currentIndex);
  const next = ctrlKey ? new Set(prevSelected) : new Set<string>();

  for (let i = start; i <= end; i++) {
    next.add(entries[i].path);
  }
  return next;
}

export function useArchiveSelection(entries: ArchiveEntry[]) {
  const [selected, setSelected] = useState<Set<string>>(() => new Set());
  const [anchorIndex, setAnchorIndex] = useState<number | null>(null);

  const clearSelection = useCallback(() => {
    setSelected(new Set());
    setAnchorIndex(null);
  }, []);

  const selectAll = useCallback(() => {
    setSelected(new Set(entries.map((e) => e.path)));
    setAnchorIndex(entries.length > 0 ? 0 : null);
  }, [entries]);

  const handleRowClick = useCallback(
    ({ index, path, ctrlKey, shiftKey }: SelectionClick) => {
      if (shiftKey) {
        setSelected((prev) =>
          applyShiftRange(entries, anchorIndex, index, prev, ctrlKey),
        );
        setAnchorIndex(index);
        return;
      }

      if (ctrlKey) {
        setSelected((prev) => {
          const next = new Set(prev);
          if (next.has(path)) next.delete(path);
          else next.add(path);
          return next;
        });
        setAnchorIndex(index);
        return;
      }

      setSelected(new Set([path]));
      setAnchorIndex(index);
    },
    [anchorIndex, entries],
  );

  const selectSingle = useCallback((path: string, index: number) => {
    setSelected(new Set([path]));
    setAnchorIndex(index);
  }, []);

  return {
    selected,
    setSelected,
    anchorIndex,
    setAnchorIndex,
    clearSelection,
    selectAll,
    handleRowClick,
    selectSingle,
  };
}
