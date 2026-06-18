import { forwardRef } from "react";
import { HeheArchiveIcon } from "./ArchiveFileTable";

interface Props {
  visible: boolean;
  fileCount: number;
}

export const FileDragGhost = forwardRef<HTMLDivElement, Props>(
  function FileDragGhost({ visible, fileCount }, ref) {
    if (!visible) return null;

    return (
      <div
        ref={ref}
        className="file-drag-ghost"
        style={{ transform: "translate(-9999px, -9999px)" }}
        aria-hidden
      >
        <HeheArchiveIcon className="file-drag-ghost__icon opacity-90" />
        <span className="file-drag-ghost__badge">{fileCount} STL</span>
      </div>
    );
  },
);
