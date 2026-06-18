export function isModifiedKey(e: KeyboardEvent, code: string): boolean {
  return (e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && e.code === code;
}
