import { useEffect, useMemo, useState } from "react";
import { api } from "../../api";
import { useI18n } from "../../i18n";
import type { ActionLogEntry, WorkflowStatus } from "../../types";

interface Props {
  archiveId: string | null;
  statuses: WorkflowStatus[];
}

function label(statuses: WorkflowStatus[], id: string | null): string {
  if (!id) return "—";
  return statuses.find((s) => s.id === id)?.label ?? id.slice(0, 8);
}

export function ActionHistoryEditor({ archiveId, statuses }: Props) {
  const { t, locale } = useI18n();
  const [entries, setEntries] = useState<ActionLogEntry[]>([]);
  const [typeFilter, setTypeFilter] = useState("");
  const [entryFilter, setEntryFilter] = useState("");

  useEffect(() => {
    if (!archiveId) {
      setEntries([]);
      return;
    }
    void api.getActionLog(archiveId).then(setEntries);
  }, [archiveId]);

  const filtered = useMemo(() => {
    return entries.filter((e) => {
      if (typeFilter && e.actionType !== typeFilter) return false;
      if (entryFilter && !(e.entryPath ?? "").includes(entryFilter)) return false;
      return true;
    });
  }, [entries, typeFilter, entryFilter]);

  if (!archiveId) {
    return (
      <div className="editor-panel flex flex-1 items-center justify-center p-2.5 text-sm text-muted">
        {t("history.openArchive")}
      </div>
    );
  }

  return (
    <div className="editor-panel flex min-h-0 flex-1 flex-col bg-[var(--editor-bg)] text-[var(--editor-text)]">
      <div className="panel flex flex-wrap gap-2.5 border-b px-2.5 py-2.5 text-xs">
        <select
          className="select"
          value={typeFilter}
          onChange={(e) => setTypeFilter(e.target.value)}
        >
          <option value="">{t("history.allTypes")}</option>
          <option value="open">{t("history.typeOpen")}</option>
          <option value="status_change">{t("history.typeStatusChange")}</option>
          <option value="extract">{t("history.typeExtract")}</option>
          <option value="create">{t("history.typeCreate")}</option>
        </select>
        <input
          className="input max-w-xs"
          placeholder={t("history.entryPlaceholder")}
          value={entryFilter}
          onChange={(e) => setEntryFilter(e.target.value)}
        />
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        <table className="data-grid w-full text-xs">
          <thead>
            <tr>
              <th>{t("history.date")}</th>
              <th>{t("history.action")}</th>
              <th>{t("history.entry")}</th>
              <th>{t("history.change")}</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((e) => (
              <tr key={e.id}>
                <td>{new Date(e.createdAt).toLocaleString(locale)}</td>
                <td>{e.actionType}</td>
                <td>{e.entryPath ?? ""}</td>
                <td>
                  {e.actionType === "status_change"
                    ? t("history.statusChange", {
                        from: label(statuses, e.fromStatusId),
                        to: label(statuses, e.toStatusId),
                      })
                    : (e.detail ?? "")}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {filtered.length === 0 ? (
          <div className="p-2.5 text-center text-muted">{t("history.empty")}</div>
        ) : null}
      </div>
    </div>
  );
}