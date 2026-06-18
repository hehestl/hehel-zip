import type { ArchiveEntry } from "../types";
import { isImageEntry } from "./previewUrl";

function normalize(p: string) {
  return p.replace(/\\/g, "/").replace(/\/$/, "");
}

export function parentFolder(folder: string): string {
  const n = normalize(folder);
  if (!n) return "";
  const parts = n.split("/");
  parts.pop();
  return parts.join("/");
}

export function folderExists(entries: ArchiveEntry[], folder: string): boolean {
  if (normalize(folder) === "") return true;
  const f = normalize(folder);
  const prefix = `${f}/`;
  return entries.some((e) => {
    const p = normalize(e.path);
    return p === f || p.startsWith(prefix);
  });
}

export function formatLocationBar(
  archivePath: string,
  currentFolder: string,
): string {
  const basename = archivePath.split(/[/\\]/).pop() ?? "";
  if (!normalize(currentFolder)) return basename;
  return `${basename}\\${normalize(currentFolder).replace(/\//g, "\\")}`;
}

export function parseLocationInput(
  raw: string,
  archiveBasename: string,
): string {
  let parsed = raw.trim().replace(/\\/g, "/");
  const baseLower = archiveBasename.toLowerCase();
  const parsedLower = parsed.toLowerCase();
  if (parsedLower.startsWith(`${baseLower}/`)) {
    parsed = parsed.slice(archiveBasename.length + 1);
  } else if (parsedLower === baseLower) {
    return "";
  }
  return normalize(parsed);
}

function sortLocale(): string {
  if (typeof document !== "undefined" && document.documentElement.lang) {
    return document.documentElement.lang;
  }
  return "ru";
}

export function getVisibleEntries(
  entries: ArchiveEntry[],
  currentFolder: string,
  locale: string = sortLocale(),
): ArchiveEntry[] {
  const folder = normalize(currentFolder);
  const seen = new Set<string>();

  return entries
    .map((entry) => {
      const path = normalize(entry.path);
      if (folder === "") {
        const parts = path.split("/").filter(Boolean);
        if (parts.length === 0) return null;
        const top = parts[0];
        const key = parts.length === 1 || entry.isDir ? top : top;
        if (seen.has(key)) return null;
        seen.add(key);
        if (parts.length === 1) return entry;
        return {
          ...entry,
          path: top + (entry.isDir ? "/" : ""),
          name: top,
          isDir: true,
          size: 0,
          packedSize: 0,
          extension: "",
        };
      }

      const prefix = `${folder}/`;
      if (!path.startsWith(prefix) && path !== folder) return null;
      const rest = path.slice(prefix.length);
      const parts = rest.split("/").filter(Boolean);
      if (parts.length === 0) return null;
      if (parts.length === 1) return { ...entry, path };
      const child = parts[0];
      const childPath = `${folder}/${child}`;
      if (seen.has(childPath)) return null;
      seen.add(childPath);
      return {
        ...entry,
        path: childPath + "/",
        name: child,
        isDir: true,
        size: 0,
        packedSize: 0,
        extension: "",
      };
    })
    .filter((e): e is ArchiveEntry => e !== null)
    .sort((a, b) => {
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
      return a.name.localeCompare(b.name, locale);
    });
}

export function folderStatsFromVisible(visible: ArchiveEntry[]): {
  images: number;
  models: number;
  files: number;
} {
  let images = 0;
  let models = 0;
  let files = 0;
  for (const entry of visible) {
    if (entry.isDir) continue;
    files += 1;
    if (isImageEntry(entry.extension)) images += 1;
    if (["stl", "obj"].includes(entry.extension.toLowerCase())) models += 1;
  }
  return { images, models, files };
}

export function countFilesInFolder(
  entries: ArchiveEntry[],
  folder: string,
): number {
  return folderStatsFromVisible(getVisibleEntries(entries, folder)).files;
}

export function countImagesInFolder(
  entries: ArchiveEntry[],
  folder: string,
): number {
  return folderStatsFromVisible(getVisibleEntries(entries, folder)).images;
}

export function countStlObjInFolder(
  entries: ArchiveEntry[],
  folder: string,
): number {
  return folderStatsFromVisible(getVisibleEntries(entries, folder)).models;
}

export function formatBytes(value: number): string {
  if (value === 0) return "";
  const units = ["B", "KB", "MB", "GB"];
  let size = value;
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit += 1;
  }
  return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}
