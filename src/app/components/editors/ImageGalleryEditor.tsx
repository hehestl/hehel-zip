import { useEffect, useMemo, useState } from "react";
import type { ArchiveEntry } from "../../types";
import { useI18n } from "../../i18n";
import { isImageEntry } from "../../lib/previewUrl";
import { ProgressBar } from "../ProgressBar";
import { useImagePreview } from "../../hooks/useImagePreview";
import { ImageGridView } from "./ImageGridView";
import { ImageFullscreenOverlay } from "./ImageFullscreenOverlay";

type ViewMode = "single" | "grid";

interface Props {
  archiveId: string | null;
  entries: ArchiveEntry[];
}

export function ImageGalleryEditor({ archiveId, entries }: Props) {
  const { t } = useI18n();
  const [index, setIndex] = useState(0);
  const [viewMode, setViewMode] = useState<ViewMode>("single");
  const [fullscreen, setFullscreen] = useState(false);
  const [visibleGridPaths, setVisibleGridPaths] = useState<string[]>([]);

  const images = useMemo(
    () => entries.filter((e) => !e.isDir && isImageEntry(e.extension)),
    [entries],
  );
  const paths = useMemo(() => images.map((e) => e.path), [images]);
  const current = images[Math.min(index, Math.max(0, images.length - 1))];

  const preview = useImagePreview({
    archiveId,
    paths,
    index,
    enabled: viewMode === "single" || fullscreen || viewMode === "grid",
    extraPrefetchPaths: viewMode === "grid" ? visibleGridPaths : [],
    t,
  });

  useEffect(() => {
    setIndex(0);
    setFullscreen(false);
  }, [archiveId, entries]);

  useEffect(() => {
    if (viewMode !== "single" && !fullscreen) return;
    const onKey = (e: KeyboardEvent) => {
      if (fullscreen) return;
      if (e.key === "ArrowLeft") setIndex((i) => Math.max(0, i - 1));
      if (e.key === "ArrowRight") {
        setIndex((i) => Math.min(images.length - 1, i + 1));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [fullscreen, images.length, viewMode]);

  if (!archiveId) {
    return (
      <div className="editor-panel flex flex-1 items-center justify-center p-2.5 text-sm text-muted">
        {t("images.openArchive")}
      </div>
    );
  }

  if (images.length === 0) {
    return (
      <div className="editor-panel flex flex-1 items-center justify-center p-2.5 text-sm text-muted">
        {t("images.noImages")}
      </div>
    );
  }

  const goPrev = () => setIndex((i) => Math.max(0, i - 1));
  const goNext = () => setIndex((i) => Math.min(images.length - 1, i + 1));

  return (
    <div className="editor-panel relative flex min-h-0 flex-1 flex-col bg-[var(--editor-bg)]">
      <div className="panel flex items-center justify-between border-b px-2.5 py-2 text-xs">
        <div className="flex gap-2">
          <button
            type="button"
            className={`btn btn-ghost ${viewMode === "single" ? "menu-item-active" : ""}`}
            onClick={() => setViewMode("single")}
          >
            {t("images.single")}
          </button>
          <button
            type="button"
            className={`btn btn-ghost ${viewMode === "grid" ? "menu-item-active" : ""}`}
            onClick={() => setViewMode("grid")}
          >
            {t("images.grid")}
          </button>
        </div>
        <span>{t("images.counter", { current: index + 1, total: images.length })}</span>
      </div>

      {viewMode === "grid" ? (
        <div className="flex min-h-0 flex-1 flex-col p-2.5">
          <ImageGridView
            images={images}
            archiveId={archiveId}
            srcForPath={preview.srcForPath}
            onVisiblePathsChange={setVisibleGridPaths}
            onSelect={(i) => {
              setIndex(i);
              setFullscreen(true);
            }}
            onImageError={preview.onGridImageError}
          />
        </div>
      ) : (
        <>
          <div
            className="flex flex-1 cursor-zoom-in items-center justify-center overflow-auto p-2.5"
            onClick={() => setFullscreen(true)}
          >
            {preview.isLoading && !preview.displaySrc ? (
              <ProgressBar
                className="w-[200px]"
                ariaLabel={t("images.loading")}
              />
            ) : preview.error ? (
              <div className="text-sm text-hh-danger">{preview.error}</div>
            ) : (
              <img
                src={preview.displaySrc ?? undefined}
                alt={current?.name}
                className="max-h-full max-w-full object-contain"
                onError={preview.onImageError}
              />
            )}
          </div>
          <div className="panel flex items-center justify-between border-t px-2.5 py-2.5 text-xs">
            <button
              type="button"
              className="btn btn-ghost"
              disabled={index <= 0}
              onClick={goPrev}
            >
              ←
            </button>
            <span>{current?.name}</span>
            <button
              type="button"
              className="btn btn-ghost"
              disabled={index >= images.length - 1}
              onClick={goNext}
            >
              →
            </button>
          </div>
        </>
      )}

      <ImageFullscreenOverlay
        open={fullscreen}
        src={preview.displaySrc}
        name={current?.name ?? ""}
        index={index}
        total={images.length}
        onClose={() => setFullscreen(false)}
        onPrev={goPrev}
        onNext={goNext}
        onImageError={preview.onImageError}
      />
    </div>
  );
}