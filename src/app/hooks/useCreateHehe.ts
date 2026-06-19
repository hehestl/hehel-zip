import { useCallback, useState } from "react";
import { api } from "../api";
import { useI18n } from "../i18n";
import { pickFolderForHehe, pickSaveHehe } from "../components/ArchiveFileTable";
import type { ResolvedCreateSource } from "../lib/createHeheSources";
import { readCompressionPreset } from "../lib/compressionPrefs";
import { readConvertImagesToWebp } from "../lib/createHehePrefs";
import type { CreateHeheResult } from "../types";

export function useCreateHehe() {
  const { t } = useI18n();
  const [creating, setCreating] = useState(false);
  const [resultDialog, setResultDialog] = useState<CreateHeheResult | null>(null);

  const runCreate = useCallback(
    async (source: ResolvedCreateSource) => {
      const dest = await pickSaveHehe(source.sourceName, t);
      if (!dest) return null;

      setCreating(true);
      try {
        const preset = readCompressionPreset();
        const convertWebp = readConvertImagesToWebp();
        if (source.kind === "local") {
          return await api.createArchive(dest, source.paths, preset, convertWebp);
        }
        return await api.createHeheFromArchive(
          source.archivePath,
          source.entryPaths,
          source.stripPrefix,
          dest,
          preset,
          convertWebp,
        );
      } finally {
        setCreating(false);
      }
    },
    [t],
  );

  const createFromResolved = useCallback(
    async (source: ResolvedCreateSource | null) => {
      let resolved = source;
      if (!resolved) {
        const folder = await pickFolderForHehe(t);
        if (!folder) return;
        const name = folder.replace(/\\/g, "/").split("/").filter(Boolean).pop() ?? "archive";
        resolved = { kind: "local", paths: [folder], sourceName: name };
      }

      const result = await runCreate(resolved);
      if (result) {
        setResultDialog(result);
      }
    },
    [runCreate, t],
  );

  const dismissResult = useCallback(() => setResultDialog(null), []);

  return {
    creating,
    showCreatingOverlay: creating,
    resultDialog,
    dismissResult,
    createFromResolved,
    runCreate,
  };
}