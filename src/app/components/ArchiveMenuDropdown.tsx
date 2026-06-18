import { useEffect, useRef, useState } from "react";

export interface MenuItem {
  id: string;
  label: string;
  disabled?: boolean;
  separator?: boolean;
  checked?: boolean;
  onClick?: () => void;
}

interface Props {
  label: string;
  items: MenuItem[];
}

export function ArchiveMenuDropdown({ label, items }: Props) {
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

  return (
    <div
      ref={rootRef}
      className="app-menubar-item relative"
    >
      <button
        type="button"
        className="menubar-item"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        {label} ▾
      </button>
      {open ? (
        <div className="menu absolute left-0 top-full z-50 mt-1 min-w-[200px]" role="menu">
          {items.map((item) =>
            item.separator ? (
              <div key={item.id} className="my-1 border-t border-hh-border" />
            ) : (
              <button
                key={item.id}
                type="button"
                role="menuitem"
                className="menu-item w-full text-left"
                disabled={item.disabled}
                onClick={() => {
                  item.onClick?.();
                  setOpen(false);
                }}
              >
                {item.checked !== undefined ? (
                  <span className="flex items-center gap-2">
                    <input type="checkbox" readOnly checked={item.checked} />
                    {item.label}
                  </span>
                ) : (
                  item.label
                )}
              </button>
            ),
          )}
        </div>
      ) : null}
    </div>
  );
}
