import { useEffect, useState } from "react";
import { api } from "../../api";
import { useI18n } from "../../i18n";
import { ProgressBar } from "../ProgressBar";
import { parseHehestl, linkRowsFromRaw } from "../../lib/hehestlParser";

interface Props {
  archivePath: string | null;
  hasHehestl: boolean;
}

async function copyText(text: string) {
  await navigator.clipboard.writeText(text);
}

export function HehestlMetadataEditor({ archivePath, hasHehestl }: Props) {
  const { t } = useI18n();
  const [raw, setRaw] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState<string | null>(null);

  useEffect(() => {
    if (!archivePath || !hasHehestl) {
      setRaw(null);
      return;
    }
    void api.readHehestlFromArchive(archivePath).then((text) => {
      setRaw(text);
      setError(text ? null : t("metadata.notFound"));
    });
  }, [archivePath, hasHehestl, t]);

  if (!archivePath) {
    return (
      <div className="editor-panel flex flex-1 items-center justify-center p-2.5 text-sm text-muted">
        {t("metadata.openArchive")}
      </div>
    );
  }

  if (!hasHehestl || error) {
    return (
      <div className="editor-panel flex flex-1 items-center justify-center p-2.5 text-sm text-muted">
        {error ?? t("metadata.notFound")}
      </div>
    );
  }

  if (!raw) {
    return (
      <div className="editor-panel flex flex-1 items-center justify-center p-2.5">
        <ProgressBar className="w-[200px]" ariaLabel={t("metadata.loading")} />
      </div>
    );
  }

  const doc = parseHehestl(raw);
  const linkRows = linkRowsFromRaw(doc.rawLines);

  return (
    <div className="editor-panel flex min-h-0 flex-1 flex-col overflow-auto bg-[var(--editor-bg)] p-2.5 text-[var(--editor-text)]">
      <div className="panel mb-2 flex items-center justify-between border-b px-0 pb-2.5 text-sm">
        <div>
          <div className="font-medium">{t("metadata.title")}</div>
          <div className="text-muted text-xs">{t("metadata.hint")}</div>
        </div>
        <button
          type="button"
          className="btn btn-ghost text-xs"
          onClick={() => {
            void copyText(raw).then(() => setCopied(t("metadata.copyAllValue")));
          }}
        >
          {t("metadata.copyAll")}
        </button>
      </div>
      {copied ? (
        <div className="mb-2 text-xs text-hh-link">{t("metadata.copied", { value: copied })}</div>
      ) : null}
      <div className="space-y-2.5">
        {doc.fields.map((f) => (
          <div key={`${f.key}-${f.value}`} className="flex flex-wrap gap-2.5 text-sm">
            <span className="font-medium text-muted">{f.key}:</span>
            {f.copyable ? (
              <button
                type="button"
                className="text-left text-hh-link underline"
                onClick={() => {
                  void copyText(f.value).then(() => setCopied(f.value));
                }}
              >
                {f.value || t("metadata.empty")}
              </button>
            ) : (
              <span>{f.value}</span>
            )}
          </div>
        ))}
      </div>
      {doc.scales.length > 0 ? (
        <div className="mt-4">
          <div className="mb-2 text-sm font-medium text-muted">{t("metadata.scales")}</div>
          <div className="flex flex-wrap gap-3">
            {doc.scales.map((s, i) => (
              <div key={`${s.scale}-${s.size ?? ""}-${i}`} className="flex flex-col items-center text-sm">
                <button
                  type="button"
                  className="text-hh-link underline"
                  onClick={() => {
                    void copyText(s.scale).then(() => setCopied(s.scale));
                  }}
                >
                  {s.scale}
                </button>
                {s.size ? (
                  <button
                    type="button"
                    className="text-muted text-xs underline"
                    onClick={() => {
                      void copyText(s.size!).then(() => setCopied(s.size!));
                    }}
                  >
                    {s.size}
                  </button>
                ) : null}
              </div>
            ))}
          </div>
        </div>
      ) : null}
      {doc.tags.length > 0 ? (
        <div className="mt-4 flex flex-wrap gap-2.5">
          {doc.tags.map((tag) => (
            <button
              key={tag.text}
              type="button"
              className="btn btn-ghost text-xs"
              onClick={() => {
                void copyText(tag.copyText).then(() => setCopied(tag.copyText));
              }}
            >
              {tag.text}
            </button>
          ))}
        </div>
      ) : null}
      {linkRows.length > 0 ? (
        <div className="mt-4 space-y-1.5">
          {linkRows.map((group, gi) => (
            <div key={gi} className="flex flex-wrap items-center gap-1 text-sm">
              {group.map((l, li) => (
                <span key={`${l.label}-${l.url}`} className="inline-flex items-center gap-1">
                  {li > 0 ? <span className="text-muted">|</span> : null}
                  <a href={l.url} target="_blank" rel="noreferrer" className="text-hh-link underline">
                    {l.label}
                  </a>
                </span>
              ))}
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}
