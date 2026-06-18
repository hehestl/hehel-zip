export const EXTRACT_CACHE_DIR_KEY = "hehel-extract-cache-dir";

export function readExtractCacheDir(): string | null {
  try {
    const value = localStorage.getItem(EXTRACT_CACHE_DIR_KEY);
    return value && value.trim() ? value : null;
  } catch {
    return null;
  }
}

export function writeExtractCacheDir(path: string | null): void {
  try {
    if (!path?.trim()) {
      localStorage.removeItem(EXTRACT_CACHE_DIR_KEY);
      return;
    }
    localStorage.setItem(EXTRACT_CACHE_DIR_KEY, path);
  } catch {
    // ignore quota errors
  }
}
