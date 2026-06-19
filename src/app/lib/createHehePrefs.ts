export const CONVERT_IMAGES_WEBP_KEY = "hehel-convert-images-webp";

export function readConvertImagesToWebp(): boolean {
  try {
    return localStorage.getItem(CONVERT_IMAGES_WEBP_KEY) === "true";
  } catch {
    return false;
  }
}

export function writeConvertImagesToWebp(enabled: boolean): void {
  try {
    if (enabled) {
      localStorage.setItem(CONVERT_IMAGES_WEBP_KEY, "true");
    } else {
      localStorage.removeItem(CONVERT_IMAGES_WEBP_KEY);
    }
  } catch {
    // ignore
  }
}
