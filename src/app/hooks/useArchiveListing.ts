import { useCallback, useMemo, useState } from "react";
import { api } from "../api";
import { useI18n } from "../i18n";
import { archiveKeys, queryClient } from "../../shared/api/tauriQueryClient";
import {
  fetchAllArchiveEntries,
  LIST_PAGE_SIZE,
} from "../lib/fetchArchiveEntries";
import {
  folderExists,
  getVisibleEntries,
  parentFolder,
  parseLocationInput,
} from "../lib/archiveView";
import type { ArchiveEntry } from "../types";

export function useArchiveListing() {
  const { t, locale } = useI18n();
  const [entries, setEntries] = useState<ArchiveEntry[]>([]);
  const [archivePath, setArchivePath] = useState<string | null>(null);
  const [archiveId, setArchiveId] = useState<string | null>(null);
  const [hasHehestl, setHasHehestl] = useState(false);
  const [metadataWarning, setMetadataWarning] = useState<string | null>(null);
  const [currentFolder, setCurrentFolder] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);

  const openArchive = useCallback(async (path: string) => {
    setLoading(true);
    setError(null);
    setInfo(null);
    try {
      const normalized = await api.normalizePath(path);
      const session = await api.openArchiveSession(normalized);
      setArchiveId(session.archiveId);
      setHasHehestl(session.hasHehestl);
      setMetadataWarning(session.metadataWarning);
      const restored = await api.tryRestoreArchiveStatuses(normalized);
      if (restored && restored > 0) {
        setInfo(t("workspace.statusesRestored", { count: restored }));
      }
      const firstPage = await api.listArchiveEntriesPaginated(
        normalized,
        0,
        LIST_PAGE_SIZE,
      );
      setArchivePath(normalized);
      setEntries(firstPage.entries);

      const list =
        firstPage.totalCount > firstPage.entries.length
          ? await fetchAllArchiveEntries(normalized, LIST_PAGE_SIZE, firstPage)
          : firstPage.entries;

      if (list.length !== firstPage.entries.length) {
        setEntries(list);
      }
      queryClient.setQueryData(archiveKeys.entries(normalized, ""), list);
      setCurrentFolder("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [t]);

  const visibleEntries = useMemo(
    () => getVisibleEntries(entries, currentFolder, locale),
    [entries, currentFolder, locale],
  );

  const navigateTo = (folderPath: string) => {
    setCurrentFolder(folderPath.replace(/\\/g, "/").replace(/\/$/, ""));
  };

  const basename = archivePath?.split(/[/\\]/).pop() ?? "";

  const navigateUp = useCallback(() => {
    setCurrentFolder((prev) => parentFolder(prev));
  }, []);

  const applyInternalPath = useCallback(
    (raw: string) => {
      if (!archivePath) return false;
      const parsed = parseLocationInput(raw, basename);
      if (!folderExists(entries, parsed)) return false;
      setCurrentFolder(parsed);
      return true;
    },
    [archivePath, basename, entries],
  );

  const canNavigateUp = currentFolder !== "";

  return {
    entries,
    visibleEntries,
    archivePath,
    archiveId,
    hasHehestl,
    metadataWarning,
    currentFolder,
    loading,
    error,
    info,
    openArchive,
    navigateTo,
    navigateUp,
    applyInternalPath,
    canNavigateUp,
    setCurrentFolder,
  };
}
