export type BioPart = { text: string; href?: string };

export function safeWebUrl(value: string): string | null {
  try {
    const url = new URL(value.startsWith("www.") ? `https://${value}` : value);
    return (url.protocol === "http:" || url.protocol === "https:") && url.hostname
      ? url.href
      : null;
  } catch {
    return null;
  }
}

/** A small, safe subset for a person's plain-text introduction. */
export function bioParts(source: string): BioPart[] {
  const pattern = /\[([^\]\n]+)\]\((https?:\/\/[^\s)]+|www\.[^\s)]+)\)|(https?:\/\/[^\s<>()]+|www\.[^\s<>()]+)/gi;
  const parts: BioPart[] = [];
  let cursor = 0;
  for (const match of source.matchAll(pattern)) {
    const index = match.index;
    if (index > cursor) parts.push({ text: source.slice(cursor, index) });
    const markdown = !!match[2];
    const raw = match[2] ?? match[3];
    const suffix = markdown ? "" : (raw.match(/[.,!?;:，。！？；：]+$/u)?.[0] ?? "");
    const visible = raw.slice(0, raw.length - suffix.length);
    const href = safeWebUrl(visible);
    if (href) parts.push({ text: markdown ? match[1] : visible, href });
    else parts.push({ text: match[0].slice(0, match[0].length - suffix.length) });
    if (suffix) parts.push({ text: suffix });
    cursor = index + match[0].length;
  }
  if (cursor < source.length) parts.push({ text: source.slice(cursor) });
  return parts;
}
