import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { useI18n } from "../i18n";

export interface ArchivePathBarProps {
  archivePath: string | null;
  locationText: string;
  canNavigateUp: boolean;
  onNavigateUp: () => void;
  onNavigateToPath: (raw: string) => boolean;
  emptyHint?: string;
}

export function ArchivePathBar({
  archivePath,
  locationText,
  canNavigateUp,
  onNavigateUp,
  onNavigateToPath,
  emptyHint,
}: ArchivePathBarProps) {
  const { t } = useI18n();
  const hint = emptyHint ?? t("pathbar.emptyHint");
  const [draft, setDraft] = useState("");
  const [invalid, setInvalid] = useState(false);
  const isFocusedRef = useRef(false);
  const inputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    if (isFocusedRef.current) return;
    setDraft(archivePath ? locationText : "");
    setInvalid(false);
  }, [locationText, archivePath]);

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      const success = onNavigateToPath(draft);
      setInvalid(!success);
    } else if (e.key === "Escape") {
      e.preventDefault();
      setDraft(locationText);
      setInvalid(false);
      e.currentTarget.blur();
    }
  };

  const isReadOnly = !archivePath;
  const upEnabled = canNavigateUp && !isReadOnly;
  const goEnabled = !!archivePath && draft.trim().length > 0;

  const copyPath = async () => {
    if (!navigator.clipboard) return;
    try {
      await navigator.clipboard.writeText(draft || locationText);
    } catch {
      // ignore clipboard failures
    }
  };

  const pastePath = async () => {
    if (!navigator.clipboard || !inputRef.current) return;
    try {
      const clipboardText = await navigator.clipboard.readText();
      if (!clipboardText) return;
      setDraft(clipboardText);
      setInvalid(false);
      inputRef.current.focus();
      inputRef.current.select();
    } catch {
      // ignore clipboard failures
    }
  };

  const applyPath = () => {
    const success = onNavigateToPath(draft);
    setInvalid(!success);
    if (success) {
      inputRef.current?.blur();
    }
  };

  return (
    <div className="app-pathbar">
      <button
        type="button"
        onClick={onNavigateUp}
        disabled={!upEnabled}
        className="menubar-item px-2"
        title={t("pathbar.navigateUp")}
      >
        ↑
      </button>

      <div className="path-field flex min-h-0 items-center gap-1">
        <input
          ref={inputRef}
          type="text"
          value={draft}
          placeholder={hint}
          onChange={(e) => {
            setDraft(e.target.value);
            setInvalid(false);
          }}
          onKeyDown={handleKeyDown}
          onFocus={(e) => {
            isFocusedRef.current = true;
            e.target.select();
          }}
          onBlur={() => {
            isFocusedRef.current = false;
          }}
          readOnly={isReadOnly}
          className={`flex-1 border-none bg-transparent text-xs outline-none ${
            invalid ? "text-error" : "text-white"
          } ${isReadOnly ? "text-muted" : ""}`}
        />
        <button
          type="button"
          onClick={applyPath}
          disabled={!goEnabled}
          className="menubar-item px-2"
          title={t("pathbar.goToPath")}
        >
          ▶
        </button>
        <button
          type="button"
          onClick={copyPath}
          disabled={!draft && !locationText}
          className="menubar-item px-2"
          title={t("pathbar.copyPath")}
        >
          C
        </button>
        <button
          type="button"
          onClick={pastePath}
          disabled={isReadOnly}
          className="menubar-item px-2"
          title={t("pathbar.pastePath")}
        >
          V
        </button>
      </div>
    </div>
  );
}