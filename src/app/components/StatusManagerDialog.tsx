import type { WorkflowStatus } from "../types";
import { useI18n } from "../i18n";

interface Props {
  open: boolean;
  statuses: WorkflowStatus[];
  onClose: () => void;
  onCreate: (label: string, color: string) => Promise<void>;
  onUpdate: (
    id: string,
    label: string,
    color: string,
    sortOrder: number,
  ) => Promise<void>;
  onDelete: (id: string) => Promise<void>;
}

export function StatusManagerDialog({
  open,
  statuses,
  onClose,
  onCreate,
  onUpdate,
  onDelete,
}: Props) {
  const { t } = useI18n();

  if (!open) return null;

  return (
    <div className="dialog-overlay">
      <div className="dialog">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-sm font-medium">{t("statusManager.title")}</h2>
          <button type="button" onClick={onClose} className="btn btn-ghost px-2.5 py-1">
            X
          </button>
        </div>

        <form
          className="mb-4 flex gap-2.5"
          onSubmit={async (e) => {
            e.preventDefault();
            const fd = new FormData(e.currentTarget);
            const label = String(fd.get("label") ?? "").trim();
            const color = String(fd.get("color") ?? "#64748b");
            if (!label) return;
            await onCreate(label, color);
            e.currentTarget.reset();
          }}
        >
          <input
            name="label"
            placeholder={t("statusManager.newPlaceholder")}
            className="input flex-1"
          />
          <input
            name="color"
            type="color"
            defaultValue="#64748b"
            className="h-8 w-12 rounded-hh-sm border border-hh-border bg-hh-surface"
          />
          <button type="submit" className="btn btn-primary">
            {t("dialog.add")}
          </button>
        </form>

        <ul className="space-y-2.5">
          {statuses.map((status) => (
            <li key={status.id} className="card flex items-center gap-2.5">
              <input
                type="color"
                defaultValue={status.color}
                id={`color-${status.id}`}
                className="h-8 w-12 rounded-hh-sm border-none bg-transparent"
              />
              <input
                defaultValue={status.label}
                id={`label-${status.id}`}
                className="input flex-1"
              />
              <button
                type="button"
                className="btn btn-ghost"
                onClick={async () => {
                  const label = (
                    document.getElementById(
                      `label-${status.id}`,
                    ) as HTMLInputElement
                  ).value;
                  const color = (
                    document.getElementById(
                      `color-${status.id}`,
                    ) as HTMLInputElement
                  ).value;
                  await onUpdate(status.id, label, color, status.sortOrder);
                }}
              >
                {t("dialog.save")}
              </button>
              {!status.isDefault && (
                <button
                  type="button"
                  className="btn btn-danger"
                  onClick={() => onDelete(status.id)}
                >
                  {t("dialog.delete")}
                </button>
              )}
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}