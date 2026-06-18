import type { Locale } from "./types";

const CANONICAL_RU_TO_KEY: Record<string, string> = {
  "Предпродакшен": "workflow.preProduction",
  "Направлено в печать": "workflow.sentToPrint",
  "Отпечатано": "workflow.printed",
  "Загрунтовано": "workflow.primed",
  "Брак": "workflow.defect",
  "Перепечатать": "workflow.reprint",
};

export function translateStatusLabel(
  label: string,
  locale: Locale,
  t: (key: string) => string,
): string {
  if (locale === "ru") return label;
  const key = CANONICAL_RU_TO_KEY[label];
  return key ? t(key) : label;
}