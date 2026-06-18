import { api } from "../api";
import type { ArchiveEntry } from "../types";

export const LIST_PAGE_SIZE = 5000;

export type PaginatedArchiveResult = {
  entries: ArchiveEntry[];
  totalCount: number;
  offset: number;
  limit: number;
};

/** Постраничная загрузка полного listing (ZIP/HEHE — нативные страницы; 7z — кэш после 1-го вызова). */
export async function fetchAllArchiveEntries(
  archivePath: string,
  pageSize = LIST_PAGE_SIZE,
  firstPage?: PaginatedArchiveResult,
): Promise<ArchiveEntry[]> {
  const first =
    firstPage ??
    (await api.listArchiveEntriesPaginated(archivePath, 0, pageSize));
  const all = [...first.entries];
  let offset = first.entries.length;

  while (offset < first.totalCount) {
    const page = await api.listArchiveEntriesPaginated(
      archivePath,
      offset,
      pageSize,
    );
    if (page.entries.length === 0) break;
    all.push(...page.entries);
    offset += page.entries.length;
  }

  return all;
}