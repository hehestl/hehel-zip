import { useI18n } from "../i18n";
import type { CompressionPreset } from "../lib/compressionPrefs";
import { ArchiveMenuBar } from "./ArchiveMenuBar";
import { ArchivePathBar } from "./ArchivePathBar";
import { ProgressBar } from "./ProgressBar";

export interface ArchiveWorkspaceChromeProps {
  archivePath: string | null;
  locationText: string;
  canNavigateUp: boolean;
  loading: boolean;
  error: string | null;
  info: string | null;
  metadataWarning: string | null;
  selectedCount: number;
  createLabel: string;
  creating: boolean;
  stlOnly: boolean;
  compressionPreset: CompressionPreset;
  onCompressionPresetChange: (preset: CompressionPreset) => void;
  convertImagesToWebp: boolean;
  onConvertImagesToWebpChange: (value: boolean) => void;
  extractCacheDir: string | null;
  onStlOnlyChange: (value: boolean) => void;
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
  onNavigateUp: () => void;
  onNavigateToPath: (raw: string) => boolean;
}

export function ArchiveWorkspaceChrome({
  archivePath,
  locationText,
  canNavigateUp,
  loading,
  error,
  info,
  metadataWarning,
  onNavigateUp,
  onNavigateToPath,
  ...menuProps
}: ArchiveWorkspaceChromeProps) {
  const { t } = useI18n();
  const banner = error ?? info ?? metadataWarning;

  return (
    <>
      <ArchiveMenuBar {...menuProps} archivePath={archivePath} />

      {banner ? (
        <div className="border-b border-yellow-500 bg-yellow-50 px-2 py-0.5 text-xs">
          {banner}
        </div>
      ) : null}

      {loading ? (
        <ProgressBar
          size="thin"
          className="w-full rounded-none"
          ariaLabel={t("workspace.loadingArchive")}
        />
      ) : null}

      <ArchivePathBar
        archivePath={archivePath}
        locationText={locationText}
        canNavigateUp={canNavigateUp}
        onNavigateUp={onNavigateUp}
        onNavigateToPath={onNavigateToPath}
      />
    </>
  );
}