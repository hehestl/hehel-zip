import { convertFileSrc } from "@tauri-apps/api/core";
import type { PreviewBytesResult } from "../types";

export function previewUrl(archiveId: string, entryPath: string): string {
  const virtualPath = `preview/${archiveId}/${encodeURIComponent(entryPath)}`;
  return convertFileSrc(virtualPath, "hehe");
}

export function previewBytesToObjectUrl(result: PreviewBytesResult): string {
  const bytes = Uint8Array.from(atob(result.base64), (c) => c.charCodeAt(0));
  return URL.createObjectURL(new Blob([bytes], { type: result.mime }));
}

export function isImageEntry(extension: string): boolean {
  return ["png", "jpg", "jpeg", "webp", "gif", "bmp"].includes(
    extension.toLowerCase(),
  );
}
