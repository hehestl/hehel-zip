export type Locale = "ru" | "en";

export type TranslationTree = {
  [key: string]: string | TranslationTree;
};

export const LOCALE_STORAGE_KEY = "hehel-zip-locale";

export const DEFAULT_LOCALE: Locale = "ru";

export type TranslateFn = (
  key: string,
  vars?: Record<string, string | number>,
) => string;