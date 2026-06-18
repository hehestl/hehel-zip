import type { TranslateFn } from "../i18n";
import type { ArchiveEntry } from "../types";

export interface LocalCreateSource {
  kind: "local";
  paths: string[];
  sourceName: string;
}

export interface ArchiveCreateSource {
  kind: "archive";
  archivePath: string;
  entryPaths: string[];
  stripPrefix: string | null;
  sourceName: string;
}

export type ResolvedCreateSource = LocalCreateSource | ArchiveCreateSource;

function basename(path: string): string {
  const norm = path.replace(/\\/g, "/");
  const parts = norm.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? "archive";
}

function stem(path: string): string {
  const name = basename(path);
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(0, dot) : name;
}

export function defaultHeheNameFromPaths(paths: string[]): string {
  if (paths.length === 0) return "archive";
  if (paths.length === 1) return stem(paths[0]);
  const now = new Date();
  const stamp = [
    now.getFullYear(),
    String(now.getMonth() + 1).padStart(2, "0"),
    String(now.getDate()).padStart(2, "0"),
    "-",
    String(now.getHours()).padStart(2, "0"),
    String(now.getMinutes()).padStart(2, "0"),
  ].join("");
  return `Archive-${stamp}`;
}

export function entryMatchesFolder(entryPath: string, folder: string): boolean {
  const norm = entryPath.replace(/\\/g, "/").replace(/\/$/, "");
  const prefix = folder.replace(/\\/g, "/").replace(/\/$/, "");
  if (!prefix) return true;
  return norm === prefix || norm.startsWith(`${prefix}/`);
}

export function resolveCreateSources(params: {
  clipboardPaths: string[];
  archivePath: string | null;
  selected: Set<string>;
  allEntries: ArchiveEntry[];
  currentFolder: string;
}): ResolvedCreateSource | null {
  const { clipboardPaths, archivePath, selected, allEntries, currentFolder } =
    params;

  if (clipboardPaths.length > 0) {
    return {
      kind: "local",
      paths: clipboardPaths,
      sourceName: defaultHeheNameFromPaths(clipboardPaths),
    };
  }

  if (archivePath) {
    const archiveName = stem(archivePath);
    if (selected.size > 0) {
      const entryPaths = [...selected];
      return {
        kind: "archive",
        archivePath,
        entryPaths,
        stripPrefix: null,
        sourceName: archiveName,
      };
    }
    if (currentFolder) {
      const folderName = basename(currentFolder);
      const entryPaths = allEntries
        .filter((e) => !e.isDir && entryMatchesFolder(e.path, currentFolder))
        .map((e) => e.path);
      return {
        kind: "archive",
        archivePath,
        entryPaths,
        stripPrefix: currentFolder.replace(/\\/g, "/").replace(/\/$/, ""),
        sourceName: folderName,
      };
    }
    const entryPaths = allEntries.filter((e) => !e.isDir).map((e) => e.path);
    return {
      kind: "archive",
      archivePath,
      entryPaths,
      stripPrefix: null,
      sourceName: archiveName,
    };
  }

  return null;
}

export function createHeheButtonLabel(
  params: {
    archivePath: string | null;
    selectedCount: number;
    currentFolder: string;
  },
  t: TranslateFn,
): string {
  if (params.archivePath && params.selectedCount > 0) {
    return t("toolbar.createHeheFromSelection");
  }
  if (params.archivePath && params.currentFolder) {
    return t("toolbar.createHeheFromFolder");
  }
  return t("toolbar.createHehe");
}