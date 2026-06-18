export type { ArchiveEntry } from "../../app/types";

export function entryDisplayName(entry: { isDir: boolean; name: string }): string {
  return entry.isDir ? `[${entry.name}]` : entry.name;
}
