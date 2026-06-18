import type { TranslationTree } from "./types";

export function resolvePath(tree: TranslationTree, path: string): string | undefined {
  const value = path.split(".").reduce<unknown>((node, key) => {
    if (node && typeof node === "object" && key in (node as TranslationTree)) {
      return (node as TranslationTree)[key];
    }
    return undefined;
  }, tree);
  return typeof value === "string" ? value : undefined;
}

export function interpolate(
  template: string,
  vars?: Record<string, string | number>,
): string {
  if (!vars) return template;
  return template.replace(/\{\{(\w+)\}\}/g, (_, key: string) => {
    const value = vars[key];
    return value === undefined ? `{{${key}}}` : String(value);
  });
}