import { useCallback, useEffect, useRef, useState } from "react";
import { DRAG_START_THRESHOLD_PX } from "../lib/constants";
import type { ArchiveEntry } from "../types";
import {
  filePathsFromSelection,
  type DragEntriesOptions,
} from "./useArchiveExtract";

export type DragPhase = "idle" | "pending" | "preparing";

export function isDraggableEntry(entry: ArchiveEntry): boolean {
  if (entry.isDir) return false;
  return ["stl", "obj"].includes(entry.extension.toLowerCase());
}

export function resolveDragPaths(
  entryPath: string,
  selected: Set<string>,
  visibleEntries: ArchiveEntry[],
): string[] {
  if (selected.has(entryPath)) {
    return filePathsFromSelection(selected, visibleEntries);
  }
  return filePathsFromSelection(new Set([entryPath]), visibleEntries);
}

interface Options {
  active: boolean;
  disabled: boolean;
  selected: Set<string>;
  visibleEntries: ArchiveEntry[];
  onDragReady: (paths: string[], options: DragEntriesOptions) => Promise<void>;
}

export function useFileDragGesture({
  active,
  disabled,
  selected,
  visibleEntries,
  onDragReady,
}: Options) {
  const [phase, setPhase] = useState<DragPhase>("idle");
  const [pendingPath, setPendingPath] = useState<string | null>(null);
  const [fileCount, setFileCount] = useState(0);
  const [activePaths, setActivePaths] = useState<Set<string>>(new Set());

  const ghostRef = useRef<HTMLDivElement>(null);
  const cancelRef = useRef(false);
  const pathsRef = useRef<string[]>([]);
  const startRef = useRef({ x: 0, y: 0, path: "" });
  const rafRef = useRef<number | null>(null);
  const lastPosRef = useRef({ x: 0, y: 0 });
  const phaseRef = useRef<DragPhase>("idle");

  useEffect(() => {
    phaseRef.current = phase;
  }, [phase]);

  const updateGhostPosition = useCallback((x: number, y: number) => {
    lastPosRef.current = { x, y };
    if (rafRef.current !== null) return;
    rafRef.current = requestAnimationFrame(() => {
      rafRef.current = null;
      const el = ghostRef.current;
      if (!el) return;
      const { x: px, y: py } = lastPosRef.current;
      el.style.transform = `translate(${px + 12}px, ${py + 12}px)`;
    });
  }, []);

  const resetGesture = useCallback(() => {
    setPhase("idle");
    setPendingPath(null);
    setActivePaths(new Set());
    setFileCount(0);
    pathsRef.current = [];
  }, []);

  const cancelDrag = useCallback(() => {
    if (phaseRef.current === "idle") return;
    if (phaseRef.current === "pending") {
      resetGesture();
      return;
    }
    if (phaseRef.current === "preparing") {
      cancelRef.current = true;
      resetGesture();
    }
  }, [resetGesture]);

  const startDrag = useCallback(
    async (paths: string[]) => {
      cancelRef.current = false;
      setPhase("preparing");
      setActivePaths(new Set(paths));
      try {
        await onDragReady(paths, { cancelRef });
      } finally {
        resetGesture();
        cancelRef.current = false;
      }
    },
    [onDragReady, resetGesture],
  );

  const onRowMouseDown = useCallback(
    (e: React.MouseEvent, entry: ArchiveEntry) => {
      if (!active || disabled || !isDraggableEntry(entry) || e.button !== 0) {
        return;
      }
      const paths = resolveDragPaths(entry.path, selected, visibleEntries);
      if (paths.length === 0) return;

      startRef.current = { x: e.clientX, y: e.clientY, path: entry.path };
      pathsRef.current = paths;
      setPendingPath(entry.path);
      setFileCount(paths.length);
      setPhase("pending");
      updateGhostPosition(e.clientX, e.clientY);
    },
    [active, disabled, selected, visibleEntries, updateGhostPosition],
  );

  const onRowMouseMove = useCallback(
    (e: React.MouseEvent, entry: ArchiveEntry) => {
      if (
        phaseRef.current !== "pending" ||
        entry.path !== startRef.current.path ||
        disabled
      ) {
        return;
      }
      const dx = e.clientX - startRef.current.x;
      const dy = e.clientY - startRef.current.y;
      if (Math.hypot(dx, dy) < DRAG_START_THRESHOLD_PX) return;
      void startDrag(pathsRef.current);
    },
    [disabled, startDrag],
  );

  const onRowMouseUp = useCallback(() => {
    if (phaseRef.current === "pending") {
      resetGesture();
    }
  }, [resetGesture]);

  useEffect(() => {
    if (phase === "idle") return;

    const onWindowMove = (e: MouseEvent) => {
      updateGhostPosition(e.clientX, e.clientY);
    };
    const onWindowUp = () => {
      if (phaseRef.current === "pending") {
        resetGesture();
      }
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        cancelDrag();
      }
    };

    window.addEventListener("mousemove", onWindowMove);
    window.addEventListener("mouseup", onWindowUp);
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("mousemove", onWindowMove);
      window.removeEventListener("mouseup", onWindowUp);
      window.removeEventListener("keydown", onKeyDown);
      if (rafRef.current !== null) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
    };
  }, [phase, cancelDrag, resetGesture, updateGhostPosition]);

  return {
    phase,
    pendingPath,
    fileCount,
    activePaths,
    ghostRef,
    cancelDrag,
    onRowMouseDown,
    onRowMouseMove,
    onRowMouseUp,
  };
}
