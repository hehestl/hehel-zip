import type { SyncConfig } from "../types";
import { api } from "../api";
import { useEffect, useState } from "react";
import { useI18n } from "../i18n";

interface Props {
  open: boolean;
  config: SyncConfig | null;
  onClose: () => void;
  onSave: (config: SyncConfig) => Promise<void>;
}

export function SyncSettingsDialog({ open, config, onClose, onSave }: Props) {
  const { t } = useI18n();
  const [authBusy, setAuthBusy] = useState(false);
  const [loggedIn, setLoggedIn] = useState<boolean | null>(null);
  const [keychainWait, setKeychainWait] = useState(false);

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setKeychainWait(true);
    void api.getAuthState().then((v) => {
      if (!cancelled) setLoggedIn(v);
    }).finally(() => {
      if (!cancelled) setKeychainWait(false);
    });
    return () => {
      cancelled = true;
    };
  }, [open]);

  if (!open || !config) return null;

  const sessionState = loggedIn
    ? t("sync.sessionInKeychain")
    : t("sync.sessionLoggedOut");

  return (
    <div className="dialog-overlay">
      <form
        className="dialog"
        onSubmit={async (e) => {
          e.preventDefault();
          const fd = new FormData(e.currentTarget);
          await onSave({
            enabled: fd.get("enabled") === "on",
            apiBaseUrl: String(fd.get("apiBaseUrl") ?? ""),
            accessToken: "",
            projectId: String(fd.get("projectId") ?? ""),
            heronAuthUrl: String(fd.get("heronAuthUrl") ?? ""),
          });
        }}
      >
        <h2 className="mb-3 text-sm font-medium">{t("sync.title")}</h2>
        {keychainWait ? (
          <p className="text-muted mb-2 text-xs">{t("sync.keychainWait")}</p>
        ) : null}
        <p className="mb-2 text-xs">
          {t("sync.sessionLabel", { state: sessionState })}
        </p>
        <label className="mb-2 flex items-center gap-2.5">
          <input name="enabled" type="checkbox" defaultChecked={config.enabled} />
          {t("sync.enabled")}
        </label>
        <label className="mb-2 block">
          {t("sync.heronAuthUrl")}
          <input
            name="heronAuthUrl"
            defaultValue={config.heronAuthUrl || "https://auth.hehestl.com"}
            className="input mt-1"
          />
        </label>
        <label className="mb-2 block">
          {t("sync.apiBaseUrl")}
          <input
            name="apiBaseUrl"
            defaultValue={config.apiBaseUrl}
            className="input mt-1"
            placeholder={t("sync.apiPlaceholder")}
          />
        </label>
        <label className="mb-4 block">
          {t("sync.projectId")}
          <input
            name="projectId"
            defaultValue={config.projectId}
            className="input mt-1"
          />
        </label>
        <div className="mb-4 flex flex-wrap gap-2.5">
          <button
            type="button"
            className="btn btn-ghost"
            disabled={authBusy}
            onClick={async () => {
              setAuthBusy(true);
              setKeychainWait(true);
              try {
                const form = document.querySelector(
                  ".dialog-overlay form",
                ) as HTMLFormElement | null;
                const fd = new FormData(form ?? undefined);
                const heronAuthUrl = String(
                  fd.get("heronAuthUrl") ?? "https://auth.hehestl.com",
                );
                const hcomApiUrl = String(fd.get("apiBaseUrl") ?? "");
                await api.startHeronLogin(heronAuthUrl, hcomApiUrl);
                setLoggedIn(true);
              } catch (err) {
                alert(String(err));
              } finally {
                setAuthBusy(false);
                setKeychainWait(false);
              }
            }}
          >
            {authBusy ? t("sync.loginBusy") : t("sync.login")}
          </button>
          <button
            type="button"
            className="btn btn-ghost"
            onClick={async () => {
              await api.logoutHeron();
              setLoggedIn(false);
            }}
          >
            {t("sync.logout")}
          </button>
        </div>
        <div className="flex justify-end gap-2.5">
          <button type="button" className="btn btn-ghost" onClick={onClose}>
            {t("dialog.cancel")}
          </button>
          <button type="submit" className="btn btn-primary">
            {t("dialog.save")}
          </button>
        </div>
      </form>
    </div>
  );
}