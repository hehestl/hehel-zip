import type { EditorMode } from "../../types";
import type { TranslateFn } from "../../i18n";
import { AreaModePicker } from "./AreaModePicker";

export const EDITOR_MODE_IDS: EditorMode[] = [
  "archive",
  "images",
  "metadata",
  "history",
];

export function editorModes(t: TranslateFn): { id: EditorMode; label: string }[] {
  return [
    { id: "archive", label: t("editor.archive") },
    { id: "images", label: t("editor.images") },
    { id: "metadata", label: t("editor.metadata") },
    { id: "history", label: t("editor.history") },
  ];
}

interface Props {
  mode: EditorMode;
  disabledReasons: Partial<Record<EditorMode, string>>;
  onModeChange: (mode: EditorMode) => void;
  onFocus: () => void;
  children: React.ReactNode;
}

export function AreaHeader({
  mode,
  disabledReasons,
  onModeChange,
  onFocus,
  children,
}: Props) {
  return (
    <div
      className="panel flex items-center gap-2 border-b px-2 py-1 text-xs"
      onFocus={onFocus}
    >
      {children}
      <AreaModePicker
        mode={mode}
        disabledReasons={disabledReasons}
        onModeChange={onModeChange}
      />
    </div>
  );
}