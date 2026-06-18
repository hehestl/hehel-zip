import { useCallback, useEffect, useRef, useState, type MutableRefObject } from "react";
import { api } from "../api";
import type { ExtractOverlayMode } from "../components/ExtractProgressOverlay";
import type { ArchiveEntry } from "../types";

const WARM_DEBOUNCE_MS = 150;

function totalBytesForPaths(paths: string[], entries: ArchiveEntry[]): number {
  const byPath = new Map(entries.map((e) => [e.path, e]));
  return paths.reduce((sum, p) => sum + (byPath.get(p)?.size ?? 0), 0);
}

export function filePathsFromSelection(
  selected: Set<string>,
  visibleEntries: ArchiveEntry[],
): string[] {
  return visibleEntries
    .filter((e) => selected.has(e.path) && !e.isDir)
    .map((e) => e.path);
}

export interface DragEntriesOptions {
  cancelRef?: MutableRefObject<boolean>;
}

export function useArchiveExtract(
  archivePath: string | null,
  visibleEntries: ArchiveEntry[],
  preservePaths: boolean,
  cacheDir: string | null,
  onPulledOut?: (entryPaths: string[]) => void,
) {
  const [extracting, setExtracting] = useState(false);
  const [overlayMode, setOverlayMode] = useState<ExtractOverlayMode>("extract");
  const [cancelling, setCancelling] = useState(false);
  const [progress, setProgress] = useState({ fileCount: 0, totalBytes: 0 });
  const cancelRef = useRef(false);
  const warmTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (warmTimerRef.current) clearTimeout(warmTimerRef.current);
    };
  }, []);

  const warmExtract = useCallback(
    (entryPaths: string[]) => {
      if (!archivePath || entryPaths.length === 0) return;
      if (warmTimerRef.current) clearTimeout(warmTimerRef.current);
      warmTimerRef.current = setTimeout(() => {
        void api.warmExtractCache(
          archivePath,
          entryPaths,
          preservePaths,
          cacheDir,
        );
      }, WARM_DEBOUNCE_MS);
    },
    [archivePath, cacheDir, preservePaths],
  );

  const runWithProgress = useCallback(
    async (
      entryPaths: string[],
      mode: ExtractOverlayMode,
      action: (extracted: string[], sessionId: string) => Promise<void>,
      options?: DragEntriesOptions,
    ) => {
      if (!archivePath || entryPaths.length === 0) return;
      const signal = options?.cancelRef ?? cancelRef;
      signal.current = false;
      setOverlayMode(mode);
      setCancelling(false);
      setProgress({
        fileCount: entryPaths.length,
        totalBytes: totalBytesForPaths(entryPaths, visibleEntries),
      });
      setExtracting(true);
      try {
        const { sessionId, paths: extracted } = await api.extractToSession(
          archivePath,
          entryPaths,
          preservePaths,
          cacheDir,
        );
        if (signal.current) {
          setCancelling(true);
          await api.dropExtractSession(sessionId);
          return;
        }
        await action(extracted, sessionId);
        onPulledOut?.(entryPaths);
      } finally {
        setExtracting(false);
        setCancelling(false);
        setOverlayMode("extract");
        setProgress({ fileCount: 0, totalBytes: 0 });
        signal.current = false;
      }
    },
    [archivePath, cacheDir, onPulledOut, preservePaths, visibleEntries],
  );

  const copyEntriesToClipboard = useCallback(
    async (paths: string[]) => {
      await runWithProgress(paths, "copy", (extracted, sessionId) =>
        api.copyFilesToClipboard(extracted, sessionId),
      );
    },
    [runWithProgress],
  );

  const dragEntries = useCallback(
    async (paths: string[], options?: DragEntriesOptions) => {
      await runWithProgress(
        paths,
        "drag",
        (extracted, sessionId) => api.startFileDrag(extracted, sessionId),
        options,
      );
    },
    [runWithProgress],
  );

  return {
    extracting,
    showProgressOverlay: extracting,
    overlayMode,
    cancelling,
    progress,
    copyEntriesToClipboard,
    dragEntries,
    warmExtract,
  };
}
