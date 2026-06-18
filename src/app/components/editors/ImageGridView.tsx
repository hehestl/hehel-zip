import { useEffect, useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { formatBytes } from "../../lib/archiveView";
import type { ArchiveEntry } from "../../types";

const ITEM_MIN_WIDTH = 120;
const ROW_HEIGHT = 168;
const GRID_GAP = 10;

interface Props {
  images: ArchiveEntry[];
  archiveId: string | null;
  srcForPath: (path: string) => string | undefined;
  onSelect: (index: number) => void;
  onImageError: (path: string) => void;
  onVisiblePathsChange?: (paths: string[]) => void;
}

export function ImageGridView({
  images,
  archiveId,
  srcForPath,
  onSelect,
  onImageError,
  onVisiblePathsChange,
}: Props) {
  const parentRef = useRef<HTMLDivElement>(null);
  const [columnCount, setColumnCount] = useState(4);

  useEffect(() => {
    const el = parentRef.current;
    if (!el) return;

    const updateColumns = () => {
      const width = el.clientWidth;
      const cols = Math.max(
        1,
        Math.floor((width + GRID_GAP) / (ITEM_MIN_WIDTH + GRID_GAP)),
      );
      setColumnCount(cols);
    };

    updateColumns();
    const observer = new ResizeObserver(updateColumns);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const rowCount = Math.ceil(images.length / columnCount);

  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 2,
  });

  const virtualRows = rowVirtualizer.getVirtualItems();

  const visiblePaths = useMemo(() => {
    const paths: string[] = [];
    for (const row of virtualRows) {
      const start = row.index * columnCount;
      const end = Math.min(start + columnCount, images.length);
      for (let i = start; i < end; i += 1) {
        paths.push(images[i].path);
      }
    }
    return paths;
  }, [virtualRows, columnCount, images]);

  useEffect(() => {
    onVisiblePathsChange?.(visiblePaths);
  }, [visiblePaths, onVisiblePathsChange]);

  if (!archiveId) return null;

  const paddingTop = virtualRows[0]?.start ?? 0;
  const paddingBottom =
    rowVirtualizer.getTotalSize() - (virtualRows[virtualRows.length - 1]?.end ?? 0);

  return (
    <div ref={parentRef} className="image-grid-scroll min-h-0 flex-1 overflow-auto">
      <div
        className="image-grid-virtual"
        style={{ height: rowVirtualizer.getTotalSize(), position: "relative" }}
      >
        <div style={{ paddingTop, paddingBottom }}>
          {virtualRows.map((row) => {
            const start = row.index * columnCount;
            const rowImages = images.slice(start, start + columnCount);
            return (
              <div
                key={row.key}
                className="image-grid"
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  width: "100%",
                  transform: `translateY(${row.start}px)`,
                }}
              >
                {rowImages.map((entry, colIndex) => {
                  const index = start + colIndex;
                  return (
                    <button
                      key={entry.path}
                      type="button"
                      className="image-grid__item"
                      onClick={() => onSelect(index)}
                    >
                      <img
                        src={srcForPath(entry.path)}
                        alt={entry.name}
                        loading="lazy"
                        className="image-grid__thumb"
                        onError={() => onImageError(entry.path)}
                      />
                      <span className="image-grid__caption">
                        {entry.name}
                        <span className="text-muted">
                          {" "}
                          · {formatBytes(entry.size)}
                        </span>
                      </span>
                    </button>
                  );
                })}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}