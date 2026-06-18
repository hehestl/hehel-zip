import { useI18n } from "../i18n";
import { formatBytes } from "../lib/archiveView";
import { ProgressBar } from "./ProgressBar";

export type ExtractOverlayMode = "extract" | "copy" | "drag";

interface Props {
  open: boolean;
  fileCount: number;
  totalBytes: number;
  mode?: ExtractOverlayMode;
  cancelHint?: boolean;
  cancelling?: boolean;
}

export function ExtractProgressOverlay({
  open,
  fileCount,
  totalBytes,
  mode = "extract",
  cancelHint = false,
  cancelling = false,
}: Props) {
  const { t } = useI18n();

  if (!open) return null;

  const ariaLabels: Record<ExtractOverlayMode, string> = {
    extract: t("extract.extract"),
    copy: t("extract.copy"),
    drag: t("extract.drag"),
  };

  return (
    <div className="dialog-overlay">
      <div className="card min-w-[280px] text-center">
        <ProgressBar
          ariaLabel={cancelling ? t("extract.cancelling") : ariaLabels[mode]}
          className="mb-3"
        />
        <p className="text-muted text-xs">
          {t("extract.summary", { count: fileCount, size: formatBytes(totalBytes) })}
        </p>
        {cancelHint && !cancelling && (
          <p className="text-muted mt-2 text-xs">{t("extract.cancelHint")}</p>
        )}
      </div>
    </div>
  );
}