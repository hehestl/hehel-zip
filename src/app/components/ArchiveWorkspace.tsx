import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { api } from "../api";
import { ActionHistoryEditor } from "./editors/ActionHistoryEditor";
import { HehestlMetadataEditor } from "./editors/HehestlMetadataEditor";
import { ImageGalleryEditor } from "./editors/ImageGalleryEditor";
import { AreaLayoutRoot } from "./layout/AreaLayoutRoot";
import { EDITOR_MODE_IDS } from "./layout/AreaHeader";
import { useI18n } from "../i18n";
import { ArchiveWorkspaceChrome } from "./ArchiveWorkspaceChrome";
import {
  ArchiveFileTable,
  pickArchiveFile,
  pickExtractFolder,
} from "./ArchiveFileTable";
import { CreateHeheProgressOverlay } from "./CreateHeheProgressOverlay";
import { CreateHeheResultDialog } from "./CreateHeheResultDialog";
import { ExtractProgressOverlay } from "./ExtractProgressOverlay";
import { FileDragGhost } from "./FileDragGhost";
import {
  filePathsFromSelection,
  useArchiveExtract,
} from "../hooks/useArchiveExtract";
import { useFileDragGesture } from "../hooks/useFileDragGesture";
import {
  ArchiveContextMenu,
  type ContextMenuState,
} from "./ArchiveContextMenu";
import { useAreaLayout } from "../hooks/useAreaLayout";
import { useArchiveListing } from "../hooks/useArchiveListing";
import { isImageEntry } from "../lib/previewUrl";
import type { EditorMode } from "../types";
import { useArchiveSelection } from "../hooks/useArchiveSelection";
import { useMarqueeSelection } from "../hooks/useMarqueeSelection";
import { useArchiveShortcuts } from "../hooks/useArchiveShortcuts";
import { archiveTabTitle } from "../lib/archiveTabs";
import { findLeaf } from "../lib/areaLayout";
import { openDetachedPanel } from "../lib/windowManager";
import { folderStatsFromVisible, formatLocationBar } from "../lib/archiveView";
import { useCreateHehe } from "../hooks/useCreateHehe";
import {
  createHeheButtonLabel,
  resolveCreateSources,
} from "../lib/createHeheSources";
import { findPrintStatusId, pathsWithoutStatus } from "../lib/entryStatus";
import {
  readExtractCacheDir,
  writeExtractCacheDir,
} from "../lib/extractCachePrefs";
import {
  readConvertImagesToWebp,
  writeConvertImagesToWebp,
} from "../lib/createHehePrefs";
import {
  readCompressionPreset,
  writeCompressionPreset,
  type CompressionPreset,
} from "../lib/compressionPrefs";
import type { ArchiveEntry, ArchiveTabMetadata, WorkflowStatus } from "../types";
import { useArchiveEvents } from "../../features/archive/hooks/useArchiveEvents";
import {
  useEntryStatusesQuery,
  useSetEntryStatusMutation,
} from "../../features/archive/hooks/useEntryStatusesQuery";
import { archiveKeys, queryClient } from "../../shared/api/tauriQueryClient";

const STL_ONLY_KEY = "hehel-stl-only";

function readStlOnlyPreference(): boolean {
  try {
    return localStorage.getItem(STL_ONLY_KEY) !== "false";
  } catch {
    return true;
  }
}

interface Props {
  tab: ArchiveTabMetadata;
  active: boolean;
  statuses: WorkflowStatus[];
  globalDialogsOpen: boolean;
  onTitleChange: (tabId: string, title: string, archivePath: string | null) => void;
  onOpenPathInNewTab: (path: string) => void;
  onManageStatuses: () => void;
  onSyncSettings: () => void;
  onRequestNewWindow: () => void;
  onLayoutChange: (tabId: string, layout: ArchiveTabMetadata["layout"]) => void;
}

export function ArchiveWorkspace({
  tab,
  active,
  statuses,
  globalDialogsOpen,
  onTitleChange,
  onOpenPathInNewTab,
  onManageStatuses,
  onSyncSettings,
  onRequestNewWindow,
  onLayoutChange,
}: Props) {
  const { t } = useI18n();
  const listing = useArchiveListing();
  const area = useAreaLayout(tab.layout, (layout) => onLayoutChange(tab.id, layout));
  const { data: statusMap = {} } = useEntryStatusesQuery(listing.archivePath);
  const setStatusMutation = useSetEntryStatusMutation(listing.archivePath);
  useArchiveEvents(listing.archivePath);
  const [stlOnly, setStlOnly] = useState(readStlOnlyPreference);
  const [extractCacheDir, setExtractCacheDir] = useState(readExtractCacheDir);
  const [compressionPreset, setCompressionPreset] = useState(readCompressionPreset);
  const [convertImagesToWebp, setConvertImagesToWebp] = useState(readConvertImagesToWebp);
  const [preservePaths] = useState(true);
  const [overwrite] = useState("ask");
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);

  const tableContainerRef = useRef<HTMLDivElement>(null);
  const initialLoadRef = useRef(false);

  const filteredEntries = useMemo(
    () =>
      stlOnly
        ? listing.visibleEntries.filter(
            (e) =>
              e.isDir || ["stl", "obj"].includes(e.extension.toLowerCase()),
          )
        : listing.visibleEntries,
    [listing.visibleEntries, stlOnly],
  );

  const folderStats = useMemo(
    () => folderStatsFromVisible(listing.visibleEntries),
    [listing.visibleEntries],
  );

  const folderStatsText = useMemo(() => {
    const parts: string[] = [];
    if (folderStats.images > 0) {
      parts.push(t("statusbar.images", { count: folderStats.images }));
    }
    if (folderStats.models > 0) {
      parts.push(t("statusbar.models", { count: folderStats.models }));
    }
    if (folderStats.files > 0) {
      parts.push(t("statusbar.files", { count: folderStats.files }));
    }
    return parts.join(" • ");
  }, [folderStats, t]);

  const handleStlOnlyChange = useCallback((value: boolean) => {
    setStlOnly(value);
    try {
      localStorage.setItem(STL_ONLY_KEY, value ? "true" : "false");
    } catch {
      // ignore
    }
  }, []);

  const {
    selected,
    setSelected,
    setAnchorIndex,
    clearSelection,
    selectAll,
    handleRowClick,
    selectSingle,
  } = useArchiveSelection(filteredEntries);

  const reloadStatusesForArchive = useCallback(async (path: string) => {
    await queryClient.invalidateQueries({
      queryKey: archiveKeys.statuses(path),
    });
  }, []);

  const applyStatus = useCallback(
    async (entryPaths: string[], statusId: string | null) => {
      if (!listing.archivePath || entryPaths.length === 0) return;
      if (entryPaths.length === 1) {
        await setStatusMutation.mutateAsync({
          entryPath: entryPaths[0],
          statusId,
        });
        return;
      }
      await api.setEntryStatusBulk(
        listing.archivePath,
        entryPaths,
        statusId,
      );
      await reloadStatusesForArchive(listing.archivePath);
    },
    [listing.archivePath, reloadStatusesForArchive, setStatusMutation],
  );

  const markSentToPrintIfUnset = useCallback(
    async (entryPaths: string[]) => {
      const targets = pathsWithoutStatus(entryPaths, statusMap);
      const printStatusId = findPrintStatusId(statuses);
      if (targets.length === 0 || !printStatusId) return;
      await applyStatus(targets, printStatusId);
    },
    [applyStatus, statusMap, statuses],
  );

  const {
    extracting,
    showProgressOverlay,
    overlayMode,
    cancelling,
    progress,
    copyEntriesToClipboard,
    dragEntries,
    warmExtract,
  } = useArchiveExtract(
    listing.archivePath,
    filteredEntries,
    preservePaths,
    extractCacheDir,
    (paths) => void markSentToPrintIfUnset(paths),
  );

  const {
    creating,
    showCreatingOverlay,
    resultDialog,
    dismissResult,
    createFromResolved,
  } = useCreateHehe();

  const dragGesture = useFileDragGesture({
    active,
    disabled: extracting || creating || globalDialogsOpen,
    selected,
    visibleEntries: filteredEntries,
    onDragReady: dragEntries,
  });

  const createHeheLabel = useMemo(
    () =>
      createHeheButtonLabel(
        {
          archivePath: listing.archivePath,
          selectedCount: selected.size,
          currentFolder: listing.currentFolder,
        },
        t,
      ),
    [listing.archivePath, listing.currentFolder, selected.size, t],
  );

  const handlePopOutPanel = useCallback(
    async (leafId: string) => {
      const archivePath = listing.archivePath;
      if (!archivePath) return;
      const leaf = findLeaf(area.layout, leafId);
      if (!leaf || leaf.kind !== "leaf") return;
      try {
        await openDetachedPanel({
          archivePath,
          mode: leaf.mode,
          title: archiveTabTitle(archivePath, t),
        });
        area.removePanel(leafId);
      } catch (e) {
        console.error(e);
        alert(String(e));
      }
    },
    [area, listing.archivePath, t],
  );

  const handleClosePanel = useCallback(
    (leafId: string) => {
      area.removePanel(leafId);
    },
    [area],
  );

  /**
   * Приоритет источников для создания .hehe:
   * 1. Буфер Проводника (readClipboardFiles)
   * 2. Контекст открытого архива (выделение → currentFolder → все entries)
   * 3. Диалог выбора папки (pickFolderForHehe) — основной fallback
   */
  const handleCreateHehe = useCallback(async () => {
    const clipboardPaths = await api.readClipboardFiles();
    const resolved = resolveCreateSources({
      clipboardPaths,
      archivePath: listing.archivePath,
      selected,
      allEntries: listing.entries,
      currentFolder: listing.currentFolder,
    });
    await createFromResolved(resolved);
  }, [
    createFromResolved,
    listing.archivePath,
    listing.currentFolder,
    listing.entries,
    selected,
  ]);

  const marqueeState = useMarqueeSelection(
    tableContainerRef,
    filteredEntries,
    selected,
    setSelected,
    setAnchorIndex,
    !active || extracting || creating || !!contextMenu,
  );

  useEffect(() => {
    if (!active) setContextMenu(null);
  }, [active]);

  useEffect(() => {
    clearSelection();
  }, [listing.currentFolder, clearSelection]);

  useEffect(() => {
    if (!active || !listing.archivePath || selected.size === 0) return;
    const paths = filePathsFromSelection(selected, filteredEntries);
    if (paths.length > 0) {
      warmExtract(paths);
    }
  }, [active, filteredEntries, listing.archivePath, selected, warmExtract]);

  useEffect(() => {
    if (listing.archivePath) {
      void reloadStatusesForArchive(listing.archivePath);
      onTitleChange(
        tab.id,
        archiveTabTitle(listing.archivePath, t),
        listing.archivePath,
      );
    }
  }, [listing.archivePath, onTitleChange, reloadStatusesForArchive, tab.id, t]);

  useEffect(() => {
    if (!tab.initialPath || listing.archivePath || initialLoadRef.current) {
      return;
    }
    initialLoadRef.current = true;
    void listing.openArchive(tab.initialPath);
  }, [tab.initialPath, listing.archivePath, listing.openArchive]);

  const resolveExtractEntryPaths = useCallback(
    (entries: string[]) => {
      if (entries.length > 0) {
        return entries.filter((p) => {
          const entry = filteredEntries.find((e) => e.path === p);
          return entry && !entry.isDir;
        });
      }
      return filteredEntries.filter((e) => !e.isDir).map((e) => e.path);
    },
    [filteredEntries],
  );

  const resolveCopyPaths = useCallback(
    (fallbackPath?: string) => {
      if (selected.size > 0) {
        return filePathsFromSelection(selected, filteredEntries);
      }
      if (fallbackPath) {
        return filteredEntries
          .filter((e) => e.path === fallbackPath && !e.isDir)
          .map((e) => e.path);
      }
      return [];
    },
    [filteredEntries, selected],
  );

  const handleCopy = useCallback(
    async (fallbackPath?: string) => {
      const paths = resolveCopyPaths(fallbackPath);
      if (paths.length === 0) return;
      await copyEntriesToClipboard(paths);
    },
    [copyEntriesToClipboard, resolveCopyPaths],
  );


  const handleExtract = async (entries: string[]) => {
    if (!listing.archivePath) return;
    const dest = await pickExtractFolder(t);
    if (!dest) return;
    const entryPaths = resolveExtractEntryPaths(entries);
    await api.extractArchive(
      listing.archivePath,
      dest,
      entries,
      preservePaths,
      overwrite,
    );
    await markSentToPrintIfUnset(entryPaths);
  };

  const handleOpenArchive = async () => {
    const path = await pickArchiveFile(t);
    if (path) onOpenPathInNewTab(path);
  };

  const shortcutsDisabled =
    !active ||
    globalDialogsOpen ||
    extracting ||
    creating;

  useArchiveShortcuts({
    disabled: shortcutsDisabled,
    statuses,
    selected,
    visibleEntries: filteredEntries,
    onSelectAll: () => {
      selectAll();
      setContextMenu(null);
    },
    onCopy: () => void handleCopy(),
    onCreateArchive: () => void handleCreateHehe(),
    onApplyStatus: applyStatus,
    onOpenFolder: listing.navigateTo,
  });

  useEffect(() => {
    if (!active || globalDialogsOpen) return;
    const handler = (e: KeyboardEvent) => {
      if (!e.ctrlKey || e.shiftKey) return;
      const num = Number(e.key);
      if (num >= 1 && num <= 4) {
        e.preventDefault();
        area.setFocusedMode(EDITOR_MODE_IDS[num - 1]);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [active, area, globalDialogsOpen]);

  const hasImages = useMemo(
    () => listing.entries.some((e) => !e.isDir && isImageEntry(e.extension)),
    [listing.entries],
  );

  const disabledReasons: Partial<Record<EditorMode, string>> = {
    images: hasImages ? undefined : t("workspace.disabledNoImages"),
    metadata: listing.hasHehestl ? undefined : t("workspace.disabledNoMetadata"),
  };

  const onRowMouseDown = dragGesture.onRowMouseDown;
  const onRowMouseMove = dragGesture.onRowMouseMove;
  const onRowMouseUp = dragGesture.onRowMouseUp;

  const openContextMenu = (
    e: React.MouseEvent,
    entry: ArchiveEntry,
    index: number,
  ) => {
    if (!active) return;
    e.preventDefault();
    if (!selected.has(entry.path)) {
      selectSingle(entry.path, index);
    }
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      entryPath: entry.path,
      entryIndex: index,
      isDir: entry.isDir,
    });
  };

  const handleNavigateToPath = (raw: string) => {
    const ok = listing.applyInternalPath(raw);
    if (ok) clearSelection();
    return ok;
  };

  const chromeSlot =
    typeof document !== "undefined"
      ? document.getElementById("app-chrome-slot")
      : null;

  const chrome =
    active && chromeSlot
      ? createPortal(
          <ArchiveWorkspaceChrome
            archivePath={listing.archivePath}
            locationText={
              listing.archivePath
                ? formatLocationBar(listing.archivePath, listing.currentFolder)
                : ""
            }
            canNavigateUp={listing.canNavigateUp}
            loading={listing.loading}
            error={listing.error}
            info={listing.info}
            metadataWarning={listing.metadataWarning}
            selectedCount={selected.size}
            createLabel={createHeheLabel}
            creating={creating}
            stlOnly={stlOnly}
            onStlOnlyChange={handleStlOnlyChange}
            compressionPreset={compressionPreset}
            onCompressionPresetChange={(preset: CompressionPreset) => {
              writeCompressionPreset(preset);
              setCompressionPreset(preset);
            }}
            convertImagesToWebp={convertImagesToWebp}
            onConvertImagesToWebpChange={(value) => {
              writeConvertImagesToWebp(value);
              setConvertImagesToWebp(value);
            }}
            extractCacheDir={extractCacheDir}
            onExtractCacheDirChange={(path) => {
              writeExtractCacheDir(path);
              setExtractCacheDir(path);
            }}
            onOpenArchive={() => void handleOpenArchive()}
            onCopy={() => void handleCopy()}
            onExtractSelected={() => void handleExtract([...selected])}
            onExtractAll={() => void handleExtract([])}
            onCreateHehe={() => void handleCreateHehe()}
            onNewWindow={() => void onRequestNewWindow()}
            onManageStatuses={onManageStatuses}
            onSyncSettings={onSyncSettings}
            onPullSync={async () => {
              if (!listing.archivePath) return;
              const n = await api.pullHestiaStatuses(listing.archivePath);
              await reloadStatusesForArchive(listing.archivePath);
              alert(t("workspace.pullResult", { count: n }));
            }}
            onPushSync={async () => {
              const n = await api.syncWithHestia();
              alert(t("workspace.pushResult", { count: n }));
            }}
            onCloudSave={
              listing.archivePath
                ? async () => {
                    const label =
                      listing.archivePath!.split(/[/\\]/).pop() ?? "archive";
                    const hash = await api.cloudSaveArchive(
                      listing.archivePath!,
                      label,
                    );
                    alert(t("workspace.cloudSaveResult", { hash: hash.slice(0, 12) }));
                  }
                : undefined
            }
            onNavigateUp={() => {
              listing.navigateUp();
              clearSelection();
            }}
            onNavigateToPath={handleNavigateToPath}
          />,
          chromeSlot,
        )
      : null;

  return (
    <div className={`flex min-h-0 flex-1 flex-col ${active ? "" : "hidden"}`}>
      {chrome}

      <div className="flex min-h-0 flex-1 flex-col">
        <AreaLayoutRoot
          node={area.layout}
          layout={area.layout}
          focusedLeafId={area.focusedLeafId}
          disabledReasons={disabledReasons}
          onFocusLeaf={area.setFocusedLeafId}
          onSplit={area.split}
          onResize={area.resize}
          onModeChange={area.setLeafMode}
          canPopOut={!!listing.archivePath}
          canClosePanel={area.canRemovePanel}
          onPopOutPanel={(leafId) => void handlePopOutPanel(leafId)}
          onClosePanel={handleClosePanel}
          renderEditor={(mode) => {
            if (mode === "archive") {
              return (
                <ArchiveFileTable
                  entries={filteredEntries}
                  selected={selected}
                  statusMap={statusMap}
                  statuses={statuses}
                  onRowClick={handleRowClick}
                  tableContainerRef={tableContainerRef}
                  marqueeState={marqueeState}
                  onOpenFolder={listing.navigateTo}
                  onSetEntryStatus={(entryPath, statusId) =>
                    void applyStatus([entryPath], statusId)
                  }
                  onRowMouseDown={onRowMouseDown}
                  onRowMouseMove={onRowMouseMove}
                  onRowMouseUp={onRowMouseUp}
                  dragPhase={dragGesture.phase}
                  dragPendingPath={dragGesture.pendingPath}
                  dragActivePaths={dragGesture.activePaths}
                  onContextMenu={openContextMenu}
                  onWarmExtract={warmExtract}
                />
              );
            }
            if (mode === "images") {
              return (
                <ImageGalleryEditor
                  archiveId={listing.archiveId}
                  entries={listing.entries}
                />
              );
            }
            if (mode === "metadata") {
              return (
                <HehestlMetadataEditor
                  archivePath={listing.archivePath}
                  hasHehestl={listing.hasHehestl}
                />
              );
            }
            return (
              <ActionHistoryEditor
                archiveId={listing.archiveId}
                statuses={statuses}
              />
            );
          }}
        />
      </div>

      <div className="panel app-statusbar flex items-center justify-between border-t text-muted">
        <span>
          {folderStatsText ? (
            <span
              title={t("statusbar.folderTitle", {
                folder: listing.currentFolder || t("statusbar.root"),
              })}
            >
              {folderStatsText}
            </span>
          ) : null}
          {folderStatsText ? " · " : ""}
          {t("statusbar.itemCount", { count: filteredEntries.length })}
          {selected.size > 0
            ? ` · ${t("statusbar.selectedCount", { count: selected.size })}`
            : ""}
          {" · "}
          {t("statusbar.hints")}
        </span>
      </div>

      {active && (
        <ArchiveContextMenu
          menu={contextMenu}
          statuses={statuses}
          hasSelection={selected.size > 0}
          onClose={() => setContextMenu(null)}
          onExtract={() => void handleExtract([...selected])}
          onCopy={() => void handleCopy(contextMenu?.entryPath)}
          onSelectAll={() => {
          selectAll();
          setContextMenu(null);
        }}
          onOpenFolder={() => {
            if (contextMenu?.isDir) {
              listing.navigateTo(contextMenu.entryPath.replace(/\/$/, ""));
            }
          }}
          onSetStatus={(statusId) => void applyStatus([...selected], statusId)}
        />
      )}

      {active && (
        <ExtractProgressOverlay
          open={showProgressOverlay}
          fileCount={progress.fileCount}
          totalBytes={progress.totalBytes}
          mode={overlayMode}
          cancelHint={overlayMode === "drag" && dragGesture.phase === "preparing"}
          cancelling={cancelling}
        />
      )}

      {active && (
        <FileDragGhost
          ref={dragGesture.ghostRef}
          visible={dragGesture.phase === "pending"}
          fileCount={dragGesture.fileCount}
        />
      )}

      {active && <CreateHeheProgressOverlay open={showCreatingOverlay} />}

      <CreateHeheResultDialog
        result={resultDialog}
        onOpen={() => {
          if (resultDialog) {
            onOpenPathInNewTab(resultDialog.outputPath);
          }
          dismissResult();
        }}
        onStay={dismissResult}
      />
    </div>
  );
}

export { ArchiveWorkspace as ArchiveTabShell };
