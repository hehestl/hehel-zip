export interface ArchiveEntry {
  path: string;
  name: string;
  size: number;
  packedSize: number;
  modified?: string;
  isDir: boolean;
  extension: string;
}

export interface WorkflowStatus {
  id: string;
  label: string;
  color: string;
  sortOrder: number;
  isDefault: boolean;
}

export interface SyncConfig {
  enabled: boolean;
  apiBaseUrl: string;
  accessToken: string;
  projectId: string;
  heronAuthUrl: string;
}

export type EntryStatusMap = Record<string, string>;

export type OverwriteMode = "ask" | "skip" | "replace";

export type SyncState = "synced" | "pending" | "conflict";

export interface ArchiveTabMetadata {
  id: string;
  title: string;
  archivePath: string | null;
  /** Однократная загрузка при создании вкладки */
  initialPath?: string;
  layout?: AreaNode;
}

export type EditorMode = "archive" | "images" | "metadata" | "history";

export type AreaNode =
  | { kind: "leaf"; id: string; mode: EditorMode }
  | {
      kind: "split";
      id: string;
      direction: "row" | "col";
      ratio: number;
      a: AreaNode;
      b: AreaNode;
    };

export interface OpenArchiveSessionResult {
  archiveId: string;
  metadataWarning: string | null;
  hasHehestl: boolean;
}

export interface ActionLogEntry {
  id: number;
  archiveId: string;
  archivePath: string | null;
  actionType: string;
  entryPath: string | null;
  fromStatusId: string | null;
  toStatusId: string | null;
  detail: string | null;
  createdAt: string;
}

export interface CreateHeheResult {
  archiveId: string;
  outputPath: string;
  entryCount: number;
  totalBytes: number;
}

export interface PreviewBytesResult {
  base64: string;
  mime: string;
}

export interface HehestlField {
  key: string;
  value: string;
  copyable: boolean;
}

export interface HehestlTag {
  text: string;
  copyText: string;
}

export interface HehestlLink {
  label: string;
  url: string;
}

export interface HehestlScale {
  scale: string;
  size?: string;
}

export interface HehestlDocument {
  fields: HehestlField[];
  tags: HehestlTag[];
  scales: HehestlScale[];
  links: HehestlLink[];
  rawLines: string[];
}

export const IMAGE_EXTENSIONS = new Set([
  "png",
  "jpg",
  "jpeg",
  "webp",
  "gif",
  "bmp",
]);
