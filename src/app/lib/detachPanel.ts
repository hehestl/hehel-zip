import type { EditorMode } from "../types";

export type DetachPayload = {
  archivePath: string;
  mode: EditorMode;
  title: string;
};

const PREFIX = "hehel_detach_";

export function stashDetachPayload(payload: DetachPayload): string {
  const id = crypto.randomUUID();
  localStorage.setItem(`${PREFIX}${id}`, JSON.stringify(payload));
  return id;
}

export function consumeDetachPayload(id: string): DetachPayload | null {
  const key = `${PREFIX}${id}`;
  const data = localStorage.getItem(key);
  if (!data) return null;
  localStorage.removeItem(key);
  try {
    return JSON.parse(data) as DetachPayload;
  } catch {
    return null;
  }
}
