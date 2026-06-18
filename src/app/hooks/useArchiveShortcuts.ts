import { useEffect } from "react";
import { isModifiedKey } from "../lib/keyboardShortcut";
import type { ArchiveEntry, WorkflowStatus } from "../types";

interface Options {
  disabled: boolean;
  statuses: WorkflowStatus[];
  selected: Set<string>;
  visibleEntries: ArchiveEntry[];
  onSelectAll: () => void;
  onCopy: () => void;
  onCreateArchive: () => void;
  onApplyStatus: (paths: string[], statusId: string | null) => void;
  onOpenFolder: (path: string) => void;
}

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA";
}

export function useArchiveShortcuts({
  disabled,
  statuses,
  selected,
  visibleEntries,
  onSelectAll,
  onCopy,
  onCreateArchive,
  onApplyStatus,
  onOpenFolder,
}: Options) {
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (disabled || isEditableTarget(e.target)) return;

      if (isModifiedKey(e, "KeyA")) {
        e.preventDefault();
        onSelectAll();
        return;
      }

      if (isModifiedKey(e, "KeyC")) {
        e.preventDefault();
        onCopy();
        return;
      }

      if (isModifiedKey(e, "KeyV")) {
        e.preventDefault();
        onCreateArchive();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && /^[1-6]$/.test(e.key)) {
        e.preventDefault();
        const status = statuses[Number(e.key) - 1];
        if (status && selected.size > 0) {
          void onApplyStatus([...selected], status.id);
        }
        return;
      }

      if (e.key === "Delete" && selected.size > 0) {
        e.preventDefault();
        void onApplyStatus([...selected], null);
        return;
      }

      if (e.key === "Enter" && selected.size === 1) {
        const path = [...selected][0];
        const entry = visibleEntries.find((en) => en.path === path);
        if (entry?.isDir) {
          e.preventDefault();
          onOpenFolder(entry.path.replace(/\/$/, ""));
        }
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [
    disabled,
    onApplyStatus,
    onCopy,
    onCreateArchive,
    onOpenFolder,
    onSelectAll,
    selected,
    statuses,
    visibleEntries,
  ]);
}
