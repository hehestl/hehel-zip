import { useEffect, useRef, type ReactNode } from "react";
import { useI18n } from "../i18n";
import type { WorkflowStatus } from "../types";

export interface ContextMenuState {
  x: number;
  y: number;
  entryPath: string;
  entryIndex: number;
  isDir: boolean;
}

interface Props {
  menu: ContextMenuState | null;
  statuses: WorkflowStatus[];
  hasSelection: boolean;
  onClose: () => void;
  onExtract: () => void;
  onCopy: () => void;
  onSelectAll: () => void;
  onOpenFolder: () => void;
  onSetStatus: (statusId: string | null) => void;
}

export function ArchiveContextMenu({
  menu,
  statuses,
  hasSelection,
  onClose,
  onExtract,
  onCopy,
  onSelectAll,
  onOpenFolder,
  onSetStatus,
}: Props) {
  const { t } = useI18n();
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!menu) return;
    const onMouseDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", onMouseDown);
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("mousedown", onMouseDown);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [menu, onClose]);

  if (!menu) return null;

  const statusItems = statuses.slice(0, 6);

  return (
    <div
      ref={ref}
      className="menu fixed z-50 min-w-[200px] text-sm"
      style={{ left: menu.x, top: menu.y }}
      role="menu"
    >
      <MenuButton onClick={() => { onExtract(); onClose(); }} disabled={!hasSelection}>
        {t("contextMenu.extractTo")}
      </MenuButton>
      <MenuButton onClick={() => { onCopy(); onClose(); }} disabled={!hasSelection}>
        {t("contextMenu.copy")}
      </MenuButton>
      {menu.isDir && (
        <MenuButton onClick={() => { onOpenFolder(); onClose(); }}>{t("contextMenu.open")}</MenuButton>
      )}
      <div className="my-1 border-t border-hh-border" />
      <div className="text-muted px-2.5 py-1 text-xs">{t("contextMenu.statusSection")}</div>
      {statusItems.map((s, i) => (
        <MenuButton
          key={s.id}
          onClick={() => {
            onSetStatus(s.id);
            onClose();
          }}
          disabled={!hasSelection}
        >
          <span
            className="mr-2 inline-block h-2 w-2 rounded-full"
            style={{ backgroundColor: s.color }}
          />
          {s.label}
          <span className="text-muted ml-auto text-xs">
            {t("contextMenu.shortcutHint", { number: i + 1 })}
          </span>
        </MenuButton>
      ))}
      <MenuButton
        onClick={() => {
          onSetStatus(null);
          onClose();
        }}
        disabled={!hasSelection}
      >
        {t("contextMenu.clearStatus")}
      </MenuButton>
      <div className="my-1 border-t border-hh-border" />
      <MenuButton onClick={() => { onSelectAll(); onClose(); }}>{t("contextMenu.selectAll")}</MenuButton>
    </div>
  );
}

function MenuButton({
  children,
  onClick,
  disabled,
}: {
  children: ReactNode;
  onClick: () => void;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      disabled={disabled}
      className="menu-item items-center"
      onClick={onClick}
    >
      {children}
    </button>
  );
}