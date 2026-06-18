export const COMPRESSION_PRESET_KEY = "hehel-compression-preset";

export type CompressionPreset = "fast" | "balanced" | "ultra";

const VALID: CompressionPreset[] = ["fast", "balanced", "ultra"];

export function readCompressionPreset(): CompressionPreset {
  try {
    const value = localStorage.getItem(COMPRESSION_PRESET_KEY);
    if (value && VALID.includes(value as CompressionPreset)) {
      return value as CompressionPreset;
    }
  } catch {
    // ignore
  }
  return "balanced";
}

export function writeCompressionPreset(preset: CompressionPreset): void {
  try {
    localStorage.setItem(COMPRESSION_PRESET_KEY, preset);
  } catch {
    // ignore quota errors
  }
}
