import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { invoke } from "@tauri-apps/api/core";
import type { CompressionPreset } from "./lib/compressionPrefs";
import type {
  ActionLogEntry,
  ArchiveEntry,
  CreateHeheResult,
  EntryStatusMap,
  OpenArchiveSessionResult,
  PreviewBytesResult,
  SyncConfig,
  WorkflowStatus,
} from "./types";

export const api = {
  listArchiveEntries: (archivePath: string) =>
    invoke<ArchiveEntry[]>("list_archive_entries", { archivePath }),

  listArchiveEntriesPaginated: (
    archivePath: string,
    offset: number,
    limit: number,
  ) =>
    invoke<{
      entries: ArchiveEntry[];
      totalCount: number;
      offset: number;
      limit: number;
    }>("list_archive_entries_paginated", { archivePath, offset, limit }),

  probeArchive: (path: string) => invoke<boolean>("probe_archive", { path }),

  normalizePath: (path: string) => invoke<string>("normalize_path", { path }),

  openArchiveSession: (archivePath: string) =>
    invoke<OpenArchiveSessionResult>("open_archive_session", { archivePath }),

  tryRestoreArchiveStatuses: (archivePath: string) =>
    invoke<number | null>("try_restore_archive_statuses", { archivePath }),

  extractArchive: (
    archivePath: string,
    destination: string,
    entries: string[],
    preservePaths: boolean,
    overwrite: string,
  ) =>
    invoke<string[]>("extract_archive", {
      archivePath,
      destination,
      entries,
      preservePaths,
      overwrite,
    }),

  getWorkflowStatuses: () => invoke<WorkflowStatus[]>("get_workflow_statuses"),

  createWorkflowStatus: (label: string, color: string) =>
    invoke<WorkflowStatus>("create_workflow_status", { label, color }),

  updateWorkflowStatus: (
    id: string,
    label: string,
    color: string,
    sortOrder: number,
  ) =>
    invoke<WorkflowStatus>("update_workflow_status", {
      id,
      label,
      color,
      sortOrder,
    }),

  deleteWorkflowStatus: (id: string) =>
    invoke<void>("delete_workflow_status", { id }),

  getEntryStatuses: (archivePath: string) =>
    invoke<EntryStatusMap>("get_entry_statuses", { archivePath }),

  setEntryStatus: (
    archivePath: string,
    entryPath: string,
    statusId: string | null,
  ) =>
    invoke<void>("set_entry_status", { archivePath, entryPath, statusId }),

  setEntryStatusBulk: (
    archivePath: string,
    entryPaths: string[],
    statusId: string | null,
  ) =>
    invoke<void>("set_entry_status_bulk", {
      archivePath,
      entryPaths,
      statusId,
    }),

  getRecentArchives: () => invoke<string[]>("get_recent_archives"),

  getSyncConfig: () => invoke<SyncConfig>("get_sync_config"),

  saveSyncConfig: (config: SyncConfig) =>
    invoke<void>("save_sync_config", { config }),

  syncWithHestia: () => invoke<number>("sync_with_hestia"),

  pullHestiaStatuses: (archivePath: string) =>
    invoke<number>("pull_hestia_statuses", { archivePath }),

  startHeronLogin: (heronAuthUrl: string, hcomApiUrl: string) =>
    invoke<{ ok: boolean; message: string }>("start_heron_login_cmd", {
      heronAuthUrl,
      hcomApiUrl,
    }),

  getAuthState: () => invoke<boolean>("get_auth_state"),

  logoutHeron: () => invoke<void>("logout_heron"),

  cloudSaveArchive: (archivePath: string, label: string) =>
    invoke<string>("cloud_save_archive", { archivePath, label }),

  extractToSession: (
    archivePath: string,
    entries: string[],
    preservePaths: boolean,
    cacheDir?: string | null,
  ) =>
    invoke<{ sessionId: string; paths: string[] }>("extract_to_session", {
      archivePath,
      entries,
      preservePaths,
      cacheDir: cacheDir ?? null,
    }),

  warmExtractCache: (
    archivePath: string,
    entries: string[],
    preservePaths: boolean,
    cacheDir?: string | null,
  ) =>
    invoke<void>("warm_extract_cache", {
      archivePath,
      entries,
      preservePaths,
      cacheDir: cacheDir ?? null,
    }),

  dropExtractSession: (sessionId: string) =>
    invoke<void>("drop_extract_session", { sessionId }),

  copyFilesToClipboard: (paths: string[], sessionId?: string) =>
    invoke<void>("copy_files_to_clipboard", { paths, sessionId }),

  readClipboardFiles: () => invoke<string[]>("read_clipboard_files"),

  createArchive: (
    outputPath: string,
    filePaths: string[],
    compressionPreset?: CompressionPreset,
    convertImagesToWebp?: boolean,
  ) =>
    invoke<CreateHeheResult>("create_archive", {
      outputPath,
      filePaths,
      compressionPreset: compressionPreset ?? null,
      convertImagesToWebp: convertImagesToWebp ?? null,
    }),

  createHeheFromArchive: (
    archivePath: string,
    entryPaths: string[],
    stripPrefix: string | null,
    outputPath: string,
    compressionPreset?: CompressionPreset,
    convertImagesToWebp?: boolean,
  ) =>
    invoke<CreateHeheResult>("create_hehe_from_archive", {
      archivePath,
      entryPaths,
      stripPrefix,
      outputPath,
      compressionPreset: compressionPreset ?? null,
      convertImagesToWebp: convertImagesToWebp ?? null,
    }),

  readHehestlFromArchive: (archivePath: string) =>
    invoke<string | null>("read_hehestl_from_archive", { archivePath }),

  readPreviewBytes: (archiveId: string, entryPath: string) =>
    invoke<PreviewBytesResult>("read_preview_bytes", { archiveId, entryPath }),

  getActionLog: (archiveId: string, limit = 200) =>
    invoke<ActionLogEntry[]>("get_action_log", { archiveId, limit }),

  startFileDrag: (paths: string[], sessionId?: string) =>
    invoke<void>("start_file_drag", {
      windowLabel: getCurrentWebviewWindow().label,
      paths,
      sessionId,
    }),
};
