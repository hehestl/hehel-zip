import { useCallback, useState } from "react";
import { useI18n } from "../i18n";
import {
  archiveTabTitle,
  createEmptyTabMetadata,
  createTabWithPath,
} from "../lib/archiveTabs";
import type { ArchiveTabMetadata } from "../types";

export function useArchiveTabs() {
  const { t } = useI18n();
  const initial = createEmptyTabMetadata(t);
  const [tabs, setTabs] = useState<ArchiveTabMetadata[]>([initial]);
  const [activeTabId, setActiveTabId] = useState(initial.id);

  const createEmptyTab = useCallback(() => {
    const tab = createEmptyTabMetadata(t);
    setTabs((prev) => [...prev, tab]);
    setActiveTabId(tab.id);
    return tab.id;
  }, [t]);

  const openPathInNewTab = useCallback(
    (path: string) => {
      const tab = createTabWithPath(path, t);
      setTabs((prev) => [...prev, tab]);
      setActiveTabId(tab.id);
      return tab.id;
    },
    [t],
  );

  const closeTab = useCallback(
    (id: string) => {
      setTabs((prev) => {
        if (prev.length <= 1) {
          const empty = createEmptyTabMetadata(t);
          setActiveTabId(empty.id);
          return [empty];
        }
        const next = prev.filter((tab) => tab.id !== id);
        setActiveTabId((current) => {
          if (current !== id) return current;
          const closedIndex = prev.findIndex((tab) => tab.id === id);
          const fallback = next[Math.max(0, closedIndex - 1)] ?? next[0];
          return fallback.id;
        });
        return next;
      });
    },
    [t],
  );

  const setActiveTab = useCallback((id: string) => {
    setActiveTabId(id);
  }, []);

  const updateTab = useCallback(
    (
      id: string,
      patch: Partial<
        Pick<ArchiveTabMetadata, "title" | "archivePath" | "initialPath" | "layout">
      >,
    ) => {
      setTabs((prev) =>
        prev.map((tab) => (tab.id === id ? { ...tab, ...patch } : tab)),
      );
    },
    [],
  );

  const onTitleChange = useCallback(
    (tabId: string, title: string, archivePath: string | null) => {
      updateTab(tabId, {
        title: archivePath ? archiveTabTitle(archivePath, t) : title,
        archivePath,
        initialPath: undefined,
      });
    },
    [t, updateTab],
  );

  const onLayoutChange = useCallback(
    (tabId: string, layout: ArchiveTabMetadata["layout"]) => {
      updateTab(tabId, { layout });
    },
    [updateTab],
  );

  return {
    tabs,
    activeTabId,
    createEmptyTab,
    openPathInNewTab,
    closeTab,
    setActiveTab,
    onTitleChange,
    onLayoutChange,
  };
}