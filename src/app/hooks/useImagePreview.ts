import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { TranslateFn } from "../i18n";
import { api } from "../api";
import {
  previewBytesToObjectUrl,
  previewUrl,
} from "../lib/previewUrl";

const MAX_PREFETCH_CONCURRENT = 4;

let fallbackToastShown = false;

function showFallbackToast(t?: TranslateFn): void {
  if (fallbackToastShown) return;
  fallbackToastShown = true;
  console.warn(t ? t("preview.fallbackWarn") : "Preview protocol failed, using fallback mode");
}

interface Options {
  archiveId: string | null;
  paths: string[];
  index: number;
  enabled?: boolean;
  extraPrefetchPaths?: string[];
  t?: TranslateFn;
}

export function useImagePreview({
  archiveId,
  paths,
  index,
  enabled = true,
  extraPrefetchPaths = [],
  t,
}: Options) {
  const [displaySrc, setDisplaySrc] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const cacheRef = useRef(new Map<string, string>());
  const blobUrlsRef = useRef(new Set<string>());
  const prefetchQueueRef = useRef<string[]>([]);
  const prefetchInflightRef = useRef(0);
  const prefetchQueuedRef = useRef(new Set<string>());
  const currentPath = paths[index] ?? null;

  const resolveSrc = useCallback(
    async (path: string): Promise<string> => {
      const cached = cacheRef.current.get(path);
      if (cached) return cached;

      const protocol = archiveId ? previewUrl(archiveId, path) : "";
      cacheRef.current.set(path, protocol);
      return protocol;
    },
    [archiveId],
  );

  const loadFallback = useCallback(
    async (path: string): Promise<string> => {
      if (!archiveId) throw new Error("no archive");
      const result = await api.readPreviewBytes(archiveId, path);
      const blob = previewBytesToObjectUrl(result);
      blobUrlsRef.current.add(blob);
      cacheRef.current.set(path, blob);
      showFallbackToast(t);
      return blob;
    },
    [archiveId, t],
  );

  const drainPrefetchQueue = useCallback(() => {
    if (!archiveId) return;

    while (
      prefetchInflightRef.current < MAX_PREFETCH_CONCURRENT &&
      prefetchQueueRef.current.length > 0
    ) {
      const path = prefetchQueueRef.current.shift();
      if (!path) break;
      prefetchQueuedRef.current.delete(path);
      if (cacheRef.current.has(path)) continue;

      prefetchInflightRef.current += 1;
      void resolveSrc(path)
        .then((src) => {
          const img = new Image();
          img.src = src;
        })
        .finally(() => {
          prefetchInflightRef.current -= 1;
          drainPrefetchQueue();
        });
    }
  }, [archiveId, resolveSrc]);

  const enqueuePrefetch = useCallback(
    (targetPaths: string[]) => {
      for (const path of targetPaths) {
        if (!path || cacheRef.current.has(path)) continue;
        if (prefetchQueuedRef.current.has(path)) continue;
        prefetchQueuedRef.current.add(path);
        prefetchQueueRef.current.push(path);
      }
      drainPrefetchQueue();
    },
    [drainPrefetchQueue],
  );

  useEffect(() => {
    fallbackToastShown = false;
    cacheRef.current.clear();
    prefetchQueueRef.current = [];
    prefetchQueuedRef.current.clear();
    prefetchInflightRef.current = 0;
    for (const url of blobUrlsRef.current) {
      URL.revokeObjectURL(url);
    }
    blobUrlsRef.current.clear();
    setDisplaySrc(null);
    setError(null);
  }, [archiveId]);

  useEffect(() => {
    if (!enabled || !archiveId || !currentPath) {
      setDisplaySrc(null);
      setIsLoading(false);
      return;
    }

    let active = true;
    setIsLoading(true);
    setError(null);

    void resolveSrc(currentPath)
      .then((src) => {
        if (active) setDisplaySrc(src);
      })
      .catch((e) => {
        if (!active) return;
        setError(String(e));
      })
      .finally(() => {
        if (active) setIsLoading(false);
      });

    return () => {
      active = false;
    };
  }, [archiveId, currentPath, enabled, resolveSrc]);

  const prefetchPaths = useMemo(() => {
    const result: string[] = [];
    if (index > 0) result.push(paths[index - 1]);
    if (index < paths.length - 1) result.push(paths[index + 1]);
    if (index > 1) result.push(paths[index - 2]);
    if (index < paths.length - 2) result.push(paths[index + 2]);
    return result.filter(Boolean);
  }, [index, paths]);

  useEffect(() => {
    if (!enabled || !archiveId) return;
    enqueuePrefetch([...prefetchPaths, ...extraPrefetchPaths]);
  }, [
    archiveId,
    enabled,
    enqueuePrefetch,
    extraPrefetchPaths,
    prefetchPaths,
  ]);

  useEffect(() => {
    return () => {
      for (const url of blobUrlsRef.current) {
        URL.revokeObjectURL(url);
      }
      blobUrlsRef.current.clear();
      cacheRef.current.clear();
    };
  }, []);

  const onImageError = useCallback(() => {
    if (!archiveId || !currentPath) return;
    setIsLoading(true);
    void loadFallback(currentPath)
      .then((src) => setDisplaySrc(src))
      .catch((e) => setError(String(e)))
      .finally(() => setIsLoading(false));
  }, [archiveId, currentPath, loadFallback]);

  const srcForPath = useCallback(
    (path: string) => {
      if (!archiveId) return undefined;
      return cacheRef.current.get(path) ?? previewUrl(archiveId, path);
    },
    [archiveId],
  );

  const onGridImageError = useCallback(
    (path: string) => {
      if (!archiveId) return;
      void loadFallback(path);
    },
    [archiveId, loadFallback],
  );

  return {
    displaySrc,
    isLoading,
    error,
    onImageError,
    onGridImageError,
    srcForPath,
    currentPath,
  };
}