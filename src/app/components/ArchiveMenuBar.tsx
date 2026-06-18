import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { useI18n } from "../i18n";
import { pickExtractFolder } from "./ArchiveFileTable";
import { ArchiveMenuDropdown, type MenuItem } from "./ArchiveMenuDropdown";
import type { CompressionPreset } from "../lib/compressionPrefs";

interface Props {
  archivePath: string | null;
  selectedCount: number;
  createLabel: string;
  creating: boolean;
  stlOnly: boolean;
  onStlOnlyChange: (value: boolean) => void;
  compressionPreset: CompressionPreset;
  onCompressionPresetChange: (preset: CompressionPreset) => void;
  extractCacheDir: string | null;
  onExtractCacheDirChange: (path: string | null) => void;
  onOpenArchive: () => void;
  onCopy: () => void;
  onExtractSelected: () => void;
  onExtractAll: () => void;
  onCreateHehe: () => void;
  onNewWindow: () => void;
  onManageStatuses: () => void;
  onSyncSettings: () => void;
  onPullSync: () => void;
  onPushSync: () => void;
  onCloudSave?: () => void;
}

export function useAppVersion(): string {
  const [version, setVersion] = useState("0.2.0");
  useEffect(() => {
    void getVersion().then(setVersion).catch(() => undefined);
  }, []);
  return version;
}

export function ArchiveMenuBar({
  archivePath,
  selectedCount,
  createLabel,
  creating,
  stlOnly,
  onStlOnlyChange,
  compressionPreset,
  onCompressionPresetChange,
  extractCacheDir,
  onExtractCacheDirChange,
  onOpenArchive,
  onCopy,
  onExtractSelected,
  onExtractAll,
  onCreateHehe,
  onNewWindow,
  onManageStatuses,
  onSyncSettings,
  onPullSync,
  onPushSync,
  onCloudSave,
}: Props) {
  const { t, locale, setLocale } = useI18n();
  const version = useAppVersion();
  const noArchive = !archivePath;
  const noSelection = selectedCount === 0;

  const fileItems: MenuItem[] = [
    { id: "open", label: t("menu.open"), onClick: onOpenArchive },
    {
      id: "copy",
      label: t("menu.copy"),
      disabled: noArchive || noSelection,
      onClick: onCopy,
    },
    {
      id: "extract",
      label: t("menu.extract"),
      disabled: noArchive || noSelection,
      onClick: onExtractSelected,
    },
    {
      id: "extract-all",
      label: t("menu.extractAll"),
      disabled: noArchive,
      onClick: onExtractAll,
    },
    {
      id: "create-hehe",
      label: createLabel,
      disabled: creating,
      onClick: onCreateHehe,
    },
    { id: "sep1", label: "", separator: true },
    { id: "new-window", label: t("menu.newWindow"), onClick: onNewWindow },
  ];

  const settingsItems: MenuItem[] = [
    {
      id: "stl-only",
      label: t("menu.stlOnly"),
      checked: stlOnly,
      onClick: () => onStlOnlyChange(!stlOnly),
    },
    {
      id: "extract-cache",
      label: extractCacheDir
        ? t("menu.extractCacheSet")
        : t("menu.extractCachePick"),
      onClick: async () => {
        const picked = await pickExtractFolder(t);
        onExtractCacheDirChange(picked);
      },
    },
    {
      id: "clear-extract-cache",
      label: t("menu.extractCacheClear"),
      disabled: !extractCacheDir,
      onClick: () => onExtractCacheDirChange(null),
    },
    { id: "compression-sep", label: "", separator: true },
    {
      id: "compression-fast",
      label: t("menu.compressionFast"),
      checked: compressionPreset === "fast",
      onClick: () => onCompressionPresetChange("fast"),
    },
    {
      id: "compression-balanced",
      label: t("menu.compressionBalanced"),
      checked: compressionPreset === "balanced",
      onClick: () => onCompressionPresetChange("balanced"),
    },
    {
      id: "compression-ultra",
      label: t("menu.compressionUltra"),
      checked: compressionPreset === "ultra",
      onClick: () => onCompressionPresetChange("ultra"),
    },
    {
      id: "auto-preview",
      label: t("menu.autoPreviewSoon"),
      disabled: true,
    },
    { id: "theme", label: t("menu.themeSoon"), disabled: true },
    { id: "lang-sep", label: "", separator: true },
    {
      id: "lang-ru",
      label: t("menu.languageRu"),
      checked: locale === "ru",
      onClick: () => setLocale("ru"),
    },
    {
      id: "lang-en",
      label: t("menu.languageEn"),
      checked: locale === "en",
      onClick: () => setLocale("en"),
    },
  ];

  const syncItems: MenuItem[] = [
    { id: "sync-settings", label: t("menu.syncSettings"), onClick: onSyncSettings },
    {
      id: "pull",
      label: t("menu.pullStatuses"),
      disabled: noArchive,
      onClick: onPullSync,
    },
    { id: "push", label: t("menu.pushHestia"), onClick: onPushSync },
    {
      id: "cloud",
      label: t("menu.cloudSave"),
      disabled: noArchive || !onCloudSave,
      onClick: onCloudSave,
    },
  ];

  return (
    <div className="app-menubar" data-tauri-drag-region>
      <ArchiveMenuDropdown label={t("menu.file")} items={fileItems} />
      <ArchiveMenuDropdown label={t("menu.settings")} items={settingsItems} />
      <ArchiveMenuDropdown label={t("menu.sync")} items={syncItems} />
      <button type="button" className="menubar-item" onClick={onManageStatuses}>
        {t("menu.statuses")}
      </button>
      <span className="ml-auto text-[11px] text-muted" title="Hehel Zip">
        v{version}
      </span>
    </div>
  );
}