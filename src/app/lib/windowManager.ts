import { getCurrentWebviewWindow, WebviewWindow } from "@tauri-apps/api/webviewWindow";

import type { DetachPayload } from "./detachPanel";
import { stashDetachPayload } from "./detachPanel";

export async function closeAllChildWindows(): Promise<void> {
  try {
    const all = await WebviewWindow.getAll();
    await Promise.all(
      all.filter((w) => w.label.startsWith("hehel-")).map((w) => w.destroy()),
    );
  } catch (error) {
    console.error("[WindowManager] Failed to cascade close windows:", error);
  }
}

export async function initWindowLifecycle(): Promise<void> {
  const current = getCurrentWebviewWindow();
  if (current.label !== "main") return;

  await current.onCloseRequested(async (event) => {
    event.preventDefault();
    await closeAllChildWindows();
    await current.destroy();
  });
}

export async function closeCurrentWindow(): Promise<void> {
  await getCurrentWebviewWindow().destroy();
}

export function openDetachedPanel(payload: DetachPayload): Promise<void> {
  const id = stashDetachPayload(payload);
  const base = window.location.pathname || "/";
  const url = `${base}?detach=${encodeURIComponent(id)}`;
  const parent = getCurrentWebviewWindow();
  const label = `hehel-${Date.now()}`;
  return new Promise((resolve, reject) => {
    const webview = new WebviewWindow(label, {
      url,
      title: payload.title || "Hehel Zip",
      width: 1100,
      height: 720,
      minWidth: 800,
      minHeight: 500,
      resizable: true,
      parent: parent.label,
    });
    webview.once("tauri://created", () => resolve());
    webview.once("tauri://error", (e) => reject(e));
  });
}

export function openNewAppWindow(): Promise<void> {
  const parent = getCurrentWebviewWindow();
  const label = `hehel-${Date.now()}`;
  return new Promise((resolve, reject) => {
    const webview = new WebviewWindow(label, {
      url: window.location.pathname || "/",
      title: "Hehel Zip",
      width: 1100,
      height: 720,
      minWidth: 800,
      minHeight: 500,
      resizable: true,
      parent: parent.label,
    });
    webview.once("tauri://created", () => resolve());
    webview.once("tauri://error", (e) => reject(e));
  });
}
