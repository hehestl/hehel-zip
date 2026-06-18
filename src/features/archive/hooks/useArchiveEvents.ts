import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { invalidateArchive } from "../../../shared/api/tauriQueryClient";

export function useArchiveEvents(archivePath: string | null) {
  useEffect(() => {
    if (!archivePath) return;
    let unlisten: (() => void) | undefined;
    void listen<{ archivePath: string }>("hehel:status-changed", (event) => {
      if (event.payload.archivePath === archivePath) {
        void invalidateArchive(archivePath);
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, [archivePath]);
}
