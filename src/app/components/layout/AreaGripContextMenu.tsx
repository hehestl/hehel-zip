import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { useI18n } from "../../i18n";

interface Props {
  x: number;
  y: number;
  canPopOut: boolean;
  canClosePanel: boolean;
  canCloseWindow: boolean;
  onPopOut: () => void;
  onClosePanel: () => void;
  onCloseWindow: () => void;
  onClose: () => void;
}

export function AreaGripContextMenu({
  x,
  y,
  canPopOut,
  canClosePanel,
  canCloseWindow,
  onPopOut,
  onClosePanel,
  onCloseWindow,
  onClose,
}: Props) {
  const { t } = useI18n();
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    const onMouseDown = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("mousedown", onMouseDown);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("mousedown", onMouseDown);
    };
  }, [onClose]);

  const menu = (
    <div
      ref={menuRef}
      className="menu fixed z-[9999] min-w-[220px] text-sm"
      style={{ left: x, top: y }}
      role="menu"
    >
      <button
        type="button"
        role="menuitem"
        disabled={!canPopOut}
        className="menu-item"
        onClick={() => {
          onPopOut();
          onClose();
        }}
      >
        {t("layout.popOut")}
      </button>
      <button
        type="button"
        role="menuitem"
        disabled={!canClosePanel}
        className="menu-item"
        onClick={() => {
          onClosePanel();
          onClose();
        }}
      >
        {t("layout.closePanel")}
      </button>
      <button
        type="button"
        role="menuitem"
        disabled={!canCloseWindow}
        className="menu-item"
        onClick={() => {
          onCloseWindow();
          onClose();
        }}
      >
        {t("layout.closeWindow")}
      </button>
    </div>
  );

  return createPortal(menu, document.body);
}