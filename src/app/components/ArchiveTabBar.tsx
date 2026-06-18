import { useI18n } from "../i18n";
import type { ArchiveTabMetadata } from "../types";

interface Props {
  tabs: ArchiveTabMetadata[];
  activeTabId: string;
  onSelectTab: (id: string) => void;
  onCloseTab: (id: string) => void;
  onAddTab: () => void;
}

export function ArchiveTabBar({
  tabs,
  activeTabId,
  onSelectTab,
  onCloseTab,
  onAddTab,
}: Props) {
  const { t } = useI18n();

  return (
    <div
      className="panel app-tabbar flex items-stretch border-b"
      data-tauri-drag-region
    >
      <div className="flex min-w-0 flex-1 items-stretch overflow-x-auto">
        {tabs.map((tab) => {
          const isActive = tab.id === activeTabId;
          return (
            <div
              key={tab.id}
              className={`flex max-w-[220px] shrink-0 items-center border-r border-hh-border ${
                isActive
                  ? "border-b-2 border-b-hh-accent bg-hh-bg font-medium"
                  : "bg-hh-surface hover:bg-hh-accent/10"
              }`}
            >
              <button
                type="button"
                className="tab-btn min-w-0 flex-1 truncate px-2.5 text-left"
                title={tab.archivePath ?? tab.title}
                onClick={() => onSelectTab(tab.id)}
              >
                {tab.title}
              </button>
              <button
                type="button"
                className="tab-btn px-2 hover:bg-hh-danger/20"
                aria-label={t("tabs.closeTab")}
                onClick={(e) => {
                  e.stopPropagation();
                  onCloseTab(tab.id);
                }}
              >
                x
              </button>
            </div>
          );
        })}
      </div>
      <button
        type="button"
        className="btn btn-ghost shrink-0 rounded-none border-l text-sm"
        aria-label={t("tabs.addTab")}
        onClick={onAddTab}
      >
        +
      </button>
    </div>
  );
}