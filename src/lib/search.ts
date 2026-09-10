import { normalize, type Cheki } from "./model";
export function parseQuery(query: string) {
  const tags: string[] = [];
  const text = query.replace(
    /#(?:"((?:\\.|[^"\\])*)"|([^\s#]+))/g,
    (_, quoted, plain) => {
      tags.push(
        normalize(
          quoted === undefined ? plain : quoted.replace(/\\(["\\])/g, "$1"),
        ),
      );
      return " ";
    },
  );
  return { tags, words: normalize(text).split(/\s+/).filter(Boolean) };
}
export function matches(c: Cheki, query: string) {
  const parsed = parseQuery(query);
  const haystack = normalize(
    [
      c.date,
      ...c.people,
      c.group,
      c.event,
      c.notes,
      c.shotType,
      ...c.tags,
      ...c.assets.map((a) => a.originalFilename),
    ].join(" "),
  );
  return (
    parsed.tags.every((t) => c.tags.some((tag) => normalize(tag) === t)) &&
    parsed.words.every((w) => haystack.includes(w))
  );
}
export function activeTag(query: string, caret: number) {
  const before = query.slice(0, caret);
  const match = /(?:^|\s)#([^#\s"]*|"[^"\n]*)$/.exec(before);
  return match
    ? { start: before.lastIndexOf("#"), term: match[1].replace(/^"/, "") }
    : null;
}
export function quotedTag(tag: string) {
  return /[\s#"\\]/.test(tag)
    ? `#"${tag.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`
    : `#${tag}`;
}
export function completeTag(query: string, caret: number, tag: string) {
  const token = activeTag(query, caret);
  if (!token) return { value: query, caret };
  const current =
    /^#(?:"(?:\\.|[^"\\])*"?|[^\s]*)/.exec(query.slice(token.start))?.[0] ?? "";
  const prefix = query.slice(0, token.start) + quotedTag(tag) + " ";
  return {
    value:
      prefix + query.slice(token.start + current.length).replace(/^\s*/, ""),
    caret: prefix.length,
  };
}

export function relatedScore(a: Cheki, b: Cheki): number {
  let best = 0;
  for (const x of a.assets)
    for (const y of b.assets) {
      if (
        x.fingerprint &&
        y.fingerprint &&
        x.fingerprint !== "0000000000000000" &&
        y.fingerprint !== "0000000000000000"
      ) {
        let bits = BigInt("0x" + x.fingerprint) ^ BigInt("0x" + y.fingerprint);
        let distance = 0;
        while (bits) {
          distance++;
          bits &= bits - 1n;
        }
        if (distance <= 7) best = Math.max(best, 100 - distance * 5);
      }
      const stem = (s: string) =>
        s
          .replace(/\.[^.]+$/, "")
          .replace(
            /[-_ ](scan|mobile|phone|small|large|compressed|original|扫描|压缩).*$/i,
            "",
          )
          .toLowerCase();
      if (
        stem(x.originalFilename).length > 3 &&
        stem(x.originalFilename) === stem(y.originalFilename)
      )
        best = Math.max(best, 60);
    }
  return best;
}
