import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api } from "./api";
import { ArchiveTabBar } from "./components/ArchiveTabBar";
import { ArchiveTabShell } from "./components/ArchiveWorkspace";
import { pickSaveHehe } from "./components/ArchiveFileTable";
import { CreateHeheResultDialog } from "./components/CreateHeheResultDialog";
import { CreateHeheProgressOverlay } from "./components/CreateHeheProgressOverlay";
import { StatusManagerDialog } from "./components/StatusManagerDialog";
import { SyncSettingsDialog } from "./components/SyncSettingsDialog";
import { useArchiveTabs } from "./hooks/useArchiveTabs";
import { useWorkflowStatuses } from "./hooks/useWorkflowStatuses";
import { openNewAppWindow, initWindowLifecycle, closeCurrentWindow } from "./lib/windowManager";
import { consumeDetachPayload } from "./lib/detachPanel";
import { useI18n } from "./i18n";
import { defaultHeheNameFromPaths } from "./lib/createHeheSources";
import { readCompressionPreset } from "./lib/compressionPrefs";
import { isModifiedKey } from "./lib/keyboardShortcut";
import type { AreaNode, CreateHeheResult, SyncConfig } from "./types";

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA";
}

export default function App() {
  const { t } = useI18n();
  const { statuses, reload: reloadStatuses } = useWorkflowStatuses();
  const {
    tabs,
    activeTabId,
    createEmptyTab,
    openPathInNewTab,
    closeTab,
    setActiveTab,
    onTitleChange,
    onLayoutChange,
  } = useArchiveTabs();

  const [statusDialogOpen, setStatusDialogOpen] = useState(false);
  const [syncDialogOpen, setSyncDialogOpen] = useState(false);
  const [syncConfig, setSyncConfig] = useState<SyncConfig | null>(null);
  const [dropCreating, setDropCreating] = useState(false);
  const [dropHeheResult, setDropHeheResult] = useState<CreateHeheResult | null>(
    null,
  );

  const globalDialogsOpen = statusDialogOpen || syncDialogOpen;
  const detachHandled = useRef(false);

  useEffect(() => {
    if (detachHandled.current) return;
    const params = new URLSearchParams(window.location.search);
    const detachId = params.get("detach");
    if (!detachId) return;
    detachHandled.current = true;

    window.history.replaceState({}, "", window.location.pathname || "/");
    const payload = consumeDetachPayload(detachId);
    if (!payload) return;

    const tabId = openPathInNewTab(payload.archivePath);
    const layout: AreaNode = {
      kind: "leaf",
      id: crypto.randomUUID(),
      mode: payload.mode,
    };
    onLayoutChange(tabId, layout);
  }, [onLayoutChange, openPathInNewTab]);

  useEffect(() => {
    void api.getSyncConfig().then(setSyncConfig);
  }, []);

  useEffect(() => {
    void initWindowLifecycle();
  }, []);

  useEffect(() => {
    const isArchivePath = (path: string) =>
      /\.(hehe|zip|rar|7z)$/i.test(path);

    let unlisten: (() => void) | undefined;
    void getCurrentWindow()
      .onDragDropEvent(async (event) => {
        if (event.payload.type !== "drop") return;
        const paths = event.payload.paths.filter(Boolean);
        if (paths.length === 0) return;

        const archivePath = paths.find(isArchivePath);
        const nonArchivePaths = paths.filter((path) => !isArchivePath(path));

        if (paths.length === 1 && archivePath) {
          openPathInNewTab(archivePath);
          return;
        }

        if (nonArchivePaths.length === 0 && archivePath) {
          openPathInNewTab(archivePath);
          return;
        }

        const filesToArchive =
          nonArchivePaths.length > 0 ? nonArchivePaths : paths;
        const message =
          filesToArchive.length === 1
            ? t("app.dropConfirmSingle", { path: filesToArchive[0] })
            : t("app.dropConfirmMultiple", { count: filesToArchive.length });
        if (!window.confirm(message)) return;

        const dest = await pickSaveHehe(defaultHeheNameFromPaths(filesToArchive), t);
        if (!dest) return;

        setDropCreating(true);
        try {
          const result = await api.createArchive(
            dest,
            filesToArchive,
            readCompressionPreset(),
          );
          setDropHeheResult(result);
        } finally {
          setDropCreating(false);
        }
      })
      .then((fn) => {
        unlisten = fn;
      });
    return () => {
      unlisten?.();
    };
  }, [openPathInNewTab, t]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (globalDialogsOpen || isEditableTarget(e.target)) return;
      if (isModifiedKey(e, "KeyT")) {
        e.preventDefault();
        createEmptyTab();
        return;
      }
      if (isModifiedKey(e, "KeyW")) {
        e.preventDefault();
        if (tabs.length <= 1) {
          void closeCurrentWindow();
        } else {
          closeTab(activeTabId);
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [activeTabId, closeTab, createEmptyTab, globalDialogsOpen, tabs.length]);

  const handleNewWindow = () => {
    void openNewAppWindow().catch((e) => {
      console.error(e);
      alert(String(e));
    });
  };

  return (
    <div className="flex h-full flex-col">
      <div id="app-chrome-slot" className="shrink-0" />
      <ArchiveTabBar
        tabs={tabs}
        activeTabId={activeTabId}
        onSelectTab={setActiveTab}
        onCloseTab={closeTab}
        onAddTab={createEmptyTab}
      />

      <div className="flex min-h-0 flex-1 flex-col">
        {tabs.map((tab) => (
          <ArchiveTabShell
            key={tab.id}
            tab={tab}
            active={tab.id === activeTabId}
            statuses={statuses}
            globalDialogsOpen={globalDialogsOpen}
            onTitleChange={onTitleChange}
            onOpenPathInNewTab={openPathInNewTab}
            onManageStatuses={() => setStatusDialogOpen(true)}
            onSyncSettings={() => setSyncDialogOpen(true)}
            onRequestNewWindow={handleNewWindow}
            onLayoutChange={onLayoutChange}
          />
        ))}
      </div>

      <StatusManagerDialog
        open={statusDialogOpen}
        statuses={statuses}
        onClose={() => setStatusDialogOpen(false)}
        onCreate={async (label, color) => {
          await api.createWorkflowStatus(label, color);
          await reloadStatuses();
        }}
        onUpdate={async (id, label, color, sortOrder) => {
          await api.updateWorkflowStatus(id, label, color, sortOrder);
          await reloadStatuses();
        }}
        onDelete={async (id) => {
          await api.deleteWorkflowStatus(id);
          await reloadStatuses();
        }}
      />

      <SyncSettingsDialog
        open={syncDialogOpen}
        config={syncConfig}
        onClose={() => setSyncDialogOpen(false)}
        onSave={async (config) => {
          await api.saveSyncConfig(config);
          setSyncConfig(config);
          setSyncDialogOpen(false);
        }}
      />

      <CreateHeheProgressOverlay open={dropCreating} />

      <CreateHeheResultDialog
        result={dropHeheResult}
        onOpen={() => {
          if (dropHeheResult) {
            openPathInNewTab(dropHeheResult.outputPath);
          }
          setDropHeheResult(null);
        }}
        onStay={() => setDropHeheResult(null)}
      />
    </div>
  );
}
