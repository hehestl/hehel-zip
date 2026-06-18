import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../../../app/api";
import type { EntryStatusMap } from "../../../app/types";
import { archiveKeys } from "../../../shared/api/tauriQueryClient";

export function useEntryStatusesQuery(archivePath: string | null) {
  return useQuery({
    queryKey: archiveKeys.statuses(archivePath ?? ""),
    queryFn: () => api.getEntryStatuses(archivePath!),
    enabled: !!archivePath,
  });
}

export function useSetEntryStatusMutation(archivePath: string | null) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      entryPath,
      statusId,
    }: {
      entryPath: string;
      statusId: string | null;
    }) => api.setEntryStatus(archivePath!, entryPath, statusId),
    onMutate: async ({
      entryPath,
      statusId,
    }: {
      entryPath: string;
      statusId: string | null;
    }) => {
      if (!archivePath) return {};
      const key = archiveKeys.statuses(archivePath);
      await queryClient.cancelQueries({ queryKey: key });
      const previous = queryClient.getQueryData<EntryStatusMap>(key);
      queryClient.setQueryData<EntryStatusMap>(key, (old) => {
        const next = { ...(old ?? {}) };
        if (statusId) {
          next[entryPath] = statusId;
        } else {
          delete next[entryPath];
        }
        return next;
      });
      return { previous };
    },
    onError: (
      _err: Error,
      _vars: { entryPath: string; statusId: string | null },
      context: { previous?: EntryStatusMap } | undefined,
    ) => {
      if (!archivePath || !context?.previous) return;
      queryClient.setQueryData(archiveKeys.statuses(archivePath), context.previous);
    },
    onSettled: () => {
      if (!archivePath) return;
      void queryClient.invalidateQueries({
        queryKey: archiveKeys.statuses(archivePath),
      });
    },
  });
}
