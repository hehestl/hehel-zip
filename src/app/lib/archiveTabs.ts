import type { TranslateFn } from "../i18n";
import type { ArchiveTabMetadata } from "../types";

let tabIdSeq = 0;

function newTabId(): string {
  tabIdSeq += 1;
  return `tab-${tabIdSeq}-${Date.now()}`;
}

export function archiveTabTitle(path: string | null, t: TranslateFn): string {
  if (!path) return t("tabs.newTab");
  return path.split(/[/\\]/).pop() ?? t("tabs.archiveFallback");
}

export function createEmptyTabMetadata(t: TranslateFn): ArchiveTabMetadata {
  return {
    id: newTabId(),
    title: t("tabs.newTab"),
    archivePath: null,
  };
}

export function createTabWithPath(path: string, t: TranslateFn): ArchiveTabMetadata {
  return {
    id: newTabId(),
    title: archiveTabTitle(path, t),
    archivePath: null,
    initialPath: path,
  };
}