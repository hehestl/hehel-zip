import { useQuery } from "@tanstack/react-query";
import type { ArchiveEntry } from "../../../entities/file-entry";
import { api } from "../../../app/api";
import { archiveKeys } from "../../../shared/api/tauriQueryClient";

export function useArchiveEntriesQuery(archivePath: string | null) {
  return useQuery({
    queryKey: archiveKeys.entries(archivePath ?? ""),
    queryFn: () => api.listArchiveEntries(archivePath!),
    enabled: !!archivePath,
  });
}

export function useArchiveEntriesPaginatedQuery(
  archivePath: string | null,
  offset: number,
  limit: number,
) {
  return useQuery({
    queryKey: [...archiveKeys.entries(archivePath ?? ""), "page", offset, limit],
    queryFn: () => api.listArchiveEntriesPaginated(archivePath!, offset, limit),
    enabled: !!archivePath,
  });
}

export type PaginatedArchiveResult = {
  entries: ArchiveEntry[];
  totalCount: number;
  offset: number;
  limit: number;
};
