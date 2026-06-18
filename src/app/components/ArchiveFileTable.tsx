import { open, save } from "@tauri-apps/plugin-dialog";
import { useVirtualizer, type VirtualItem } from "@tanstack/react-virtual";
import { useI18n, type TranslateFn } from "../i18n";
import type { DragPhase } from "../hooks/useFileDragGesture";
import type { RefObject } from "react";
import type { SelectionClick } from "../hooks/useArchiveSelection";
import type { MarqueeState } from "../hooks/useMarqueeSelection";
import type { ArchiveEntry, EntryStatusMap, WorkflowStatus } from "../types";
import { formatBytes } from "../lib/archiveView";
import { SelectionMarquee } from "./SelectionMarquee";
import { StatusBadge } from "./StatusBadge";
interface Props {
  entries: ArchiveEntry[];
  selected: Set<string>;
  statusMap: EntryStatusMap;
  statuses: WorkflowStatus[];
  onRowClick: (click: SelectionClick) => void;
  tableContainerRef: RefObject<HTMLDivElement | null>;
  marqueeState: MarqueeState | null;
  onOpenFolder: (path: string) => void;
  onSetEntryStatus: (entryPath: string, statusId: string | null) => void;
  onRowMouseDown: (e: React.MouseEvent, entry: ArchiveEntry) => void;
  onRowMouseMove: (e: React.MouseEvent, entry: ArchiveEntry) => void;
  onRowMouseUp: () => void;
  dragPhase?: DragPhase;
  dragPendingPath?: string | null;
  dragActivePaths?: Set<string>;
  onContextMenu: (
    e: React.MouseEvent,
    entry: ArchiveEntry,
    index: number,
  ) => void;
  onWarmExtract?: (paths: string[]) => void;
}

export function HeheArchiveIcon({ className }: { className?: string }) {
  return (
    <svg
      aria-hidden
      className={className ?? "h-4 w-4 shrink-0 opacity-80"}
      viewBox="0 0 16 16"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        d="M3 2.5h7l3 3V13.5a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V3.5a1 1 0 0 1 1-1Z"
        stroke="currentColor"
        strokeWidth="1.2"
      />
      <path d="M10 2.5V5.5h3" stroke="currentColor" strokeWidth="1.2" />
      <path
        d="M5 8h6M5 10.5h4"
        stroke="currentColor"
        strokeWidth="1.2"
        strokeLinecap="round"
      />
    </svg>
  );
}

export function ArchiveToolbar({
  archivePath,
  selectedCount,
  createLabel,
  creating,
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
}: {
  archivePath: string | null;
  selectedCount: number;
  createLabel: string;
  creating: boolean;
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
}) {
  const { t } = useI18n();

  return (
    <div className="panel flex flex-wrap items-center gap-2.5 border-b p-2.5">
      <button type="button" className="btn btn-ghost" onClick={onOpenArchive}>
        {t("toolbar.open")}
      </button>
      <button
        type="button"
        className="btn btn-ghost"
        disabled={!archivePath || selectedCount === 0}
        onClick={onCopy}
      >
        {t("toolbar.copy")}
      </button>
      <button
        type="button"
        className="btn btn-ghost"
        disabled={!archivePath || selectedCount === 0}
        onClick={onExtractSelected}
      >
        {t("toolbar.extract")}
      </button>
      <button
        type="button"
        className="btn btn-ghost"
        disabled={!archivePath}
        onClick={onExtractAll}
      >
        {t("toolbar.extractAll")}
      </button>
      <button
        type="button"
        className="btn btn-ghost inline-flex items-center gap-1.5"
        disabled={creating}
        title={t("toolbar.createHeheTitle")}
        onClick={onCreateHehe}
      >
        <HeheArchiveIcon />
        {createLabel}
      </button>
      <button type="button" className="btn btn-ghost" onClick={onNewWindow}>
        {t("toolbar.newWindow")}
      </button>
      <button type="button" className="btn btn-ghost" onClick={onManageStatuses}>
        {t("toolbar.statuses")}
      </button>
      <button type="button" className="btn btn-ghost" onClick={onSyncSettings}>
        {t("toolbar.hestiaSync")}
      </button>
      <button
        type="button"
        className="btn btn-ghost"
        disabled={!archivePath}
        onClick={onPullSync}
      >
        {t("toolbar.pull")}
      </button>
      <button type="button" className="btn btn-ghost" onClick={onPushSync}>
        {t("toolbar.push")}
      </button>
      {onCloudSave ? (
        <button
          type="button"
          className="btn btn-ghost"
          disabled={!archivePath}
          onClick={onCloudSave}
        >
          {t("toolbar.cloudSave")}
        </button>
      ) : null}
    </div>
  );
}

export function ArchiveFileTable({
  entries,
  selected,
  statusMap,
  statuses,
  onRowClick,
  tableContainerRef,
  marqueeState,
  onOpenFolder,
  onSetEntryStatus,
  onRowMouseDown,
  onRowMouseMove,
  onRowMouseUp,
  dragPhase = "idle",
  dragPendingPath = null,
  dragActivePaths,
  onContextMenu,
  onWarmExtract,
}: Props) {
  const { t } = useI18n();
  const statusById = new Map(statuses.map((s) => [s.id, s]));
  const rowVirtualizer = useVirtualizer({
    count: entries.length,
    getScrollElement: () => tableContainerRef.current,
    estimateSize: () => 36,
    overscan: 12,
  });

  const virtualRows = rowVirtualizer.getVirtualItems();
  const paddingTop = virtualRows[0]?.start ?? 0;
  const paddingBottom =
    rowVirtualizer.getTotalSize() - (virtualRows[virtualRows.length - 1]?.end ?? 0);

  const renderRow = (entry: ArchiveEntry, index: number) => {
    const isSelected = selected.has(entry.path);
    const status = statusMap[entry.path]
      ? statusById.get(statusMap[entry.path])
      : undefined;
    const isDraggable =
      !entry.isDir &&
      ["stl", "obj"].includes(entry.extension.toLowerCase());
    const rowClasses = [
      isSelected ? "selected" : "",
      isDraggable ? "draggable-file" : "",
      dragPendingPath === entry.path && dragPhase === "pending"
        ? "drag-pending"
        : "",
      dragActivePaths?.has(entry.path) && dragPhase === "preparing"
        ? "drag-active"
        : "",
    ]
      .filter(Boolean)
      .join(" ");
    return (
      <tr
        key={entry.path}
        data-entry-index={index}
        className={rowClasses}
        onClick={(e) =>
          onRowClick({
            index,
            path: entry.path,
            ctrlKey: e.ctrlKey,
            shiftKey: e.shiftKey,
          })
        }
        onDoubleClick={() => {
          if (entry.isDir) onOpenFolder(entry.path.replace(/\/$/, ""));
        }}
        onContextMenu={(e) => onContextMenu(e, entry, index)}
        onMouseDown={(e) => onRowMouseDown(e, entry)}
        onMouseMove={(e) => onRowMouseMove(e, entry)}
        onMouseUp={onRowMouseUp}
        onMouseLeave={onRowMouseUp}
        onMouseEnter={() => {
          if (isDraggable) onWarmExtract?.([entry.path]);
        }}
      >
        <td>
          {entry.isDir
            ? t("table.folderBracket", { name: entry.name })
            : entry.name}
        </td>
        <td>{formatBytes(entry.size)}</td>
        <td>{formatBytes(entry.packedSize)}</td>
        <td>{entry.isDir ? t("table.folder") : entry.extension || t("table.file")}</td>
        <td>{entry.modified ?? ""}</td>
        <td onClick={(e) => e.stopPropagation()}>
          <select
            className="select max-w-[180px]"
            value={statusMap[entry.path] ?? ""}
            onChange={(e) => {
              const value = e.target.value;
              onSetEntryStatus(entry.path, value || null);
            }}
          >
            <option value="">{t("table.none")}</option>
            {statuses.map((s) => (
              <option key={s.id} value={s.id}>
                {s.label}
              </option>
            ))}
          </select>
          <StatusBadge status={status} />
        </td>
      </tr>
    );
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div
        ref={tableContainerRef}
        className="relative min-h-0 flex-1 overflow-auto bg-hh-bg"
      >
        <table className="data-grid">
          <thead>
            <tr>
              <th>{t("table.name")}</th>
              <th>{t("table.size")}</th>
              <th>{t("table.packed")}</th>
              <th>{t("table.type")}</th>
              <th>{t("table.modified")}</th>
              <th>{t("table.status")}</th>
            </tr>
          </thead>
          <tbody>
            {paddingTop > 0 ? (
              <tr aria-hidden style={{ height: paddingTop }} />
            ) : null}
            {virtualRows.map((virtualRow: VirtualItem) =>
              renderRow(entries[virtualRow.index], virtualRow.index),
            )}
            {paddingBottom > 0 ? (
              <tr aria-hidden style={{ height: paddingBottom }} />
            ) : null}
          </tbody>
        </table>
        <SelectionMarquee state={marqueeState} />
      </div>    </div>
  );
}

export async function pickArchiveFile(t: TranslateFn): Promise<string | null> {
  const selected = await open({
    multiple: false,
    filters: [
      {
        name: t("dialog.heheFilter"),
        extensions: ["hehe"],
      },
      {
        name: t("dialog.archivesFilter"),
        extensions: ["zip", "rar", "7z", "hehe"],
      },
    ],
  });
  if (!selected || Array.isArray(selected)) return null;
  return selected;
}

export async function pickExtractFolder(t: TranslateFn): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: t("dialog.pickExtractTitle"),
  });
  if (!selected || Array.isArray(selected)) return null;
  return selected;
}

export async function pickSaveHehe(
  sourceName: string | undefined,
  t: TranslateFn,
): Promise<string | null> {
  const selected = await save({
    title: t("dialog.saveHeheTitle"),
    defaultPath: sourceName ? `${sourceName}.hehe` : "archive.hehe",
    filters: [{ name: t("dialog.heheFilter"), extensions: ["hehe"] }],
  });
  return selected;
}

/** @deprecated use pickSaveHehe */
export async function pickSaveArchive(t: TranslateFn): Promise<string | null> {
  return pickSaveHehe(undefined, t);
}

export async function pickFolderForHehe(t: TranslateFn): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: t("dialog.pickFolderTitle"),
  });
  if (!selected || Array.isArray(selected)) return null;
  return selected;
}
