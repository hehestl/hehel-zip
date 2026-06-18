import type {
  HehestlDocument,
  HehestlField,
  HehestlLink,
  HehestlScale,
  HehestlTag,
} from "../types";

const INLINE_LINK_RE = /(.+?)\s*\((https?:\/\/[^)]+)\)/g;
const TAG_KEY_RE = /^(Теги|Tags)$/i;
const OS_KEY_RE = /^O\/S$/i;
const HASHTAG_RE = /#[\p{L}\p{N}_]+/gu;

function parseFieldSegment(segment: string): HehestlField | null {
  if (segment.includes("(http")) return null;
  const colon = segment.indexOf(":");
  if (colon <= 0) return null;
  const key = segment.slice(0, colon).trim();
  const value = segment.slice(colon + 1).trim();
  if (!key) return null;
  return { key, value, copyable: true };
}

function splitBilingualSegments(line: string): string[] {
  return line.split(" | ").map((s) => s.trim()).filter(Boolean);
}

function parseTagsFromValue(value: string): HehestlTag[] {
  const tags: HehestlTag[] = [];
  for (const match of value.matchAll(HASHTAG_RE)) {
    const text = match[0];
    tags.push({ text, copyText: text });
  }
  return tags;
}

function isTagLine(segment: string): boolean {
  const colon = segment.indexOf(":");
  if (colon <= 0) return false;
  const key = segment.slice(0, colon).trim();
  return TAG_KEY_RE.test(key);
}

function parseTagSegment(segment: string): HehestlTag[] {
  const colon = segment.indexOf(":");
  if (colon <= 0) return [];
  const value = segment.slice(colon + 1).trim();
  return parseTagsFromValue(value);
}

export function parseOsValue(raw: string): HehestlScale {
  const trimmed = raw.trim();
  if (!trimmed) return { scale: "" };

  const parenMatch = trimmed.match(/^(.+?)\s*\(([^)]+)\)\s*$/);
  if (parenMatch) {
    return { scale: parenMatch[1].trim(), size: parenMatch[2].trim() };
  }

  const dashMatch = trimmed.match(/^(.+?)\s*(?:—|–|\s-\s)\s*(.+)$/);
  if (dashMatch) {
    const size = dashMatch[2].trim();
    if (/\d/.test(size)) {
      return { scale: dashMatch[1].trim(), size };
    }
  }

  const commaMatch = trimmed.match(/^(\d+\s*[-/]\s*\d+)\s*,\s*(.+)$/);
  if (commaMatch) {
    return { scale: commaMatch[1].trim(), size: commaMatch[2].trim() };
  }

  return { scale: trimmed };
}

function isOsLine(segment: string): boolean {
  const colon = segment.indexOf(":");
  if (colon <= 0) return false;
  const key = segment.slice(0, colon).trim();
  return OS_KEY_RE.test(key);
}

function parseOsSegment(segment: string): HehestlScale | null {
  const colon = segment.indexOf(":");
  if (colon <= 0) return null;
  const value = segment.slice(colon + 1).trim();
  return parseOsValue(value);
}

function extractLinksFromText(text: string): HehestlLink[] {
  const links: HehestlLink[] = [];
  let searchFrom = 0;
  while (searchFrom < text.length) {
    INLINE_LINK_RE.lastIndex = searchFrom;
    const match = INLINE_LINK_RE.exec(text);
    if (!match) break;
    const label = match[1].trim().replace(/^\|/, "").trim();
    const url = match[2].trim();
    if (label && url) links.push({ label, url });
    searchFrom = match.index + match[0].length;
  }
  return links;
}

function pushUniqueLinks(target: HehestlLink[], incoming: HehestlLink[]) {
  const seen = new Set(target.map((l) => l.url));
  for (const link of incoming) {
    if (seen.has(link.url)) continue;
    seen.add(link.url);
    target.push(link);
  }
}

function pushUniqueTags(target: HehestlTag[], incoming: HehestlTag[]) {
  const seen = new Set(target.map((t) => t.text));
  for (const tag of incoming) {
    if (seen.has(tag.text)) continue;
    seen.add(tag.text);
    target.push(tag);
  }
}

function parseTextHehestl(raw: string): HehestlDocument {
  const fields: HehestlField[] = [];
  const tags: HehestlTag[] = [];
  const scales: HehestlScale[] = [];
  const links: HehestlLink[] = [];
  const rawLines = raw.split("\n");

  for (const line of rawLines) {
    const t = line.trim();
    if (!t) continue;

    if (t.includes(" | ")) {
      const segments = splitBilingualSegments(t);
      let handled = false;

      if (segments.every(isTagLine)) {
        for (const seg of segments) pushUniqueTags(tags, parseTagSegment(seg));
        handled = true;
      } else if (segments.every(isOsLine)) {
        for (const seg of segments) {
          const scale = parseOsSegment(seg);
          if (scale) scales.push(scale);
        }
        handled = true;
      } else {
        const parsedFields: HehestlField[] = [];
        const segmentLinks: HehestlLink[] = [];
        for (const seg of segments) {
          if (isTagLine(seg)) {
            pushUniqueTags(tags, parseTagSegment(seg));
            handled = true;
            continue;
          }
          if (isOsLine(seg)) {
            const scale = parseOsSegment(seg);
            if (scale) scales.push(scale);
            handled = true;
            continue;
          }
          const field = parseFieldSegment(seg);
          if (field) {
            parsedFields.push(field);
          } else {
            pushUniqueLinks(segmentLinks, extractLinksFromText(seg));
          }
        }
        if (parsedFields.length > 0) {
          fields.push(...parsedFields);
          handled = true;
        }
        if (segmentLinks.length > 0) {
          pushUniqueLinks(links, segmentLinks);
          handled = true;
        }
      }

      if (handled) continue;
    }

    if (isTagLine(t)) {
      pushUniqueTags(tags, parseTagSegment(t));
      continue;
    }

    if (isOsLine(t)) {
      const scale = parseOsSegment(t);
      if (scale) scales.push(scale);
      continue;
    }

    if (t.includes("(http")) {
      pushUniqueLinks(links, extractLinksFromText(t));
      continue;
    }

    const field = parseFieldSegment(t);
    if (field && !INLINE_LINK_RE.test(t)) {
      INLINE_LINK_RE.lastIndex = 0;
      fields.push(field);
      continue;
    }
    INLINE_LINK_RE.lastIndex = 0;

    const lineLinks = extractLinksFromText(t);
    if (lineLinks.length > 0) {
      pushUniqueLinks(links, lineLinks);
      continue;
    }

    pushUniqueTags(tags, parseTagsFromValue(t));
  }

  return { fields, tags, scales, links, rawLines };
}

export function parseHehestl(raw: string): HehestlDocument {
  const trimmed = raw.trim();
  if (trimmed.startsWith("{")) {
    try {
      const json = JSON.parse(trimmed) as {
        fields?: { key: string; value: string; copyable?: boolean }[];
        tags?: string[];
        scales?: { scale: string; size?: string }[];
        links?: { label: string; url: string }[];
      };
      return {
        fields: (json.fields ?? []).map((f) => ({
          key: f.key,
          value: f.value,
          copyable: f.copyable ?? true,
        })),
        tags: (json.tags ?? []).map((t) => ({ text: t, copyText: t })),
        scales: json.scales ?? [],
        links: json.links ?? [],
        rawLines: trimmed.split("\n"),
      };
    } catch {
      /* fall through to text */
    }
  }

  return parseTextHehestl(raw);
}

export function linkRowsFromRaw(rawLines: string[]): HehestlLink[][] {
  const rows: HehestlLink[][] = [];
  for (const line of rawLines) {
    const t = line.trim();
    if (!t) continue;
    const rowLinks = extractLinksFromText(t);
    if (rowLinks.length > 0) rows.push(rowLinks);
  }
  return rows;
}
