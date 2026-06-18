import { translateStatusLabel, useI18n } from "../i18n";
import type { WorkflowStatus } from "../types";

interface Props {
  status?: WorkflowStatus;
}

export function StatusBadge({ status }: Props) {
  const { t, locale } = useI18n();

  if (!status) {
    return <span className="text-muted">—</span>;
  }

  return (
    <span
      className="inline-block rounded-hh-sm px-1.5 py-0.5 text-[11px] text-white"
      style={{ backgroundColor: status.color }}
    >
      {translateStatusLabel(status.label, locale, t)}
    </span>
  );
}