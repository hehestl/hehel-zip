import { QueryClient } from "@tanstack/react-query";

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      retry: 1,
    },
  },
});

export const archiveKeys = {
  all: ["archive"] as const,
  path: (archivePath: string) => ["archive", archivePath] as const,
  entries: (archivePath: string, folder?: string) =>
    ["archive", archivePath, "entries", folder ?? ""] as const,
  statuses: (archivePath: string) =>
    ["archive", archivePath, "statuses"] as const,
};

export function invalidateArchive(archivePath: string) {
  return queryClient.invalidateQueries({
    queryKey: archiveKeys.path(archivePath),
  });
}
