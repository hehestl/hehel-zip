import { useEffect, useMemo, useRef, useState } from "react";
import type { EditorMode } from "../../types";
import { useI18n } from "../../i18n";
import { editorModes } from "./AreaHeader";

interface Props {
  mode: EditorMode;
  disabledReasons: Partial<Record<EditorMode, string>>;
  onModeChange: (mode: EditorMode) => void;
}

export function AreaModePicker({
  mode,
  disabledReasons,
  onModeChange,
}: Props) {
  const { t } = useI18n();
  const modes = useMemo(() => editorModes(t), [t]);
  const modeIcons: Record<EditorMode, string> = useMemo(
    () => ({
      archive: t("editor.iconArchive"),
      images: t("editor.iconImages"),
      metadata: t("editor.iconMetadata"),
      history: t("editor.iconHistory"),
    }),
    [t],
  );
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onMouseDown = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("mousedown", onMouseDown);
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("mousedown", onMouseDown);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  const currentLabel = modes.find((m) => m.id === mode)?.label ?? mode;
  const disabledHint = disabledReasons[mode];

  return (
    <div ref={rootRef} className="relative shrink-0">
      <button
        type="button"
        className="btn btn-ghost flex h-5 w-5 items-center justify-center p-0 text-[10px] font-medium leading-none"
        title={
          disabledHint
            ? t("editor.disabledHint", { mode: currentLabel, reason: disabledHint })
            : currentLabel
        }
        aria-label={t("editor.modeLabel", { mode: currentLabel })}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        {modeIcons[mode]}
      </button>
      {open && (
        <div
          className="menu absolute left-0 top-full z-50 mt-0.5 min-w-[140px] text-sm"
          role="menu"
        >
          {modes.map((m) => {
            const disabled = Boolean(disabledReasons[m.id]);
            const hint = disabledReasons[m.id];
            return (
              <button
                key={m.id}
                type="button"
                role="menuitem"
                disabled={disabled}
                title={hint}
                className={`menu-item items-center gap-2 text-xs ${
                  m.id === mode ? "menu-item-active" : ""
                }`}
                onClick={() => {
                  if (disabled) return;
                  onModeChange(m.id);
                  setOpen(false);
                }}
              >
                <span className="w-4 text-center">{modeIcons[m.id]}</span>
                <span>{m.label}</span>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}