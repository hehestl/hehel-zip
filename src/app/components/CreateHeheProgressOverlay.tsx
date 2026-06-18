import { useI18n } from "../i18n";
import { ProgressBar } from "./ProgressBar";

export function CreateHeheProgressOverlay({ open }: { open: boolean }) {
  const { t } = useI18n();

  if (!open) return null;

  return (
    <div className="dialog-overlay">
      <div className="card min-w-[280px] text-center">
        <ProgressBar ariaLabel={t("createHehe.progress")} />
      </div>
    </div>
  );
}