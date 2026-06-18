import { useI18n } from "../i18n";
import { formatBytes } from "../lib/archiveView";
import type { CreateHeheResult } from "../types";

interface Props {
  result: CreateHeheResult | null;
  onOpen: () => void;
  onStay: () => void;
}

export function CreateHeheResultDialog({ result, onOpen, onStay }: Props) {
  const { t } = useI18n();

  if (!result) return null;

  return (
    <div className="dialog-overlay">
      <div className="card min-w-[320px]">
        <p className="text-sm font-medium">{t("createHehe.resultTitle")}</p>
        <p className="text-muted mt-1 text-xs">
          {t("createHehe.resultSummary", {
            count: result.entryCount,
            size: formatBytes(result.totalBytes),
          })}
        </p>
        <p className="text-muted mt-2 truncate text-xs" title={result.outputPath}>
          {result.outputPath}
        </p>
        <div className="mt-4 flex flex-wrap justify-end gap-2">
          <button type="button" className="btn btn-ghost" onClick={onStay}>
            {t("createHehe.stay")}
          </button>
          <button type="button" className="btn btn-primary" onClick={onOpen}>
            {t("createHehe.openNew")}
          </button>
        </div>
      </div>
    </div>
  );
}