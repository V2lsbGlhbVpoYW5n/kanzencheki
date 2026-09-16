import { normalize, personLabel, type Person, type Cheki } from "./model";
type Term = { type: "term"; field: "text" | "tag" | "person"; value: string };
type Expr =
  | Term
  | { type: "not"; child: Expr }
  | { type: "and"; left: Expr; right: Expr }
  | { type: "or"; left: Expr; right: Expr };
type Token = Term | { type: "AND" | "OR" | "NOT" | "(" | ")" };
export function parseQuery(query: string): {
  expression: Expr | null;
  error: string;
} {
  try {
    const tokens: Token[] = [];
    let i = 0;
    while (i < query.length) {
      if (/\s/.test(query[i])) {
        i++;
        continue;
      }
      if (query[i] === "(" || query[i] === ")") {
        tokens.push({ type: query[i++] as "(" | ")" });
        continue;
      }
      const prefix = /[#@]/.test(query[i]) ? query[i++] : "";
      let value = "";
      let quoted = false;
      if (query[i] === '"') {
        quoted = true;
        i++;
        let closed = false;
        while (i < query.length) {
          if (query[i] === '"') {
            i++;
            closed = true;
            break;
          }
          if (query[i] === "\\" && i + 1 < query.length) i++;
          value += query[i++];
        }
        if (!closed) throw new Error("请补全结束引号");
      } else {
        while (i < query.length && !/[\s()]/.test(query[i]))
          value += query[i++];
      }
      if (!value)
        throw new Error(prefix ? `请在 ${prefix} 后输入名称` : "缺少搜索内容");
      if (!prefix && !quoted && /^(AND|OR|NOT)$/i.test(value))
        tokens.push({ type: value.toUpperCase() as "AND" | "OR" | "NOT" });
      else
        tokens.push({
          type: "term",
          field: prefix === "#" ? "tag" : prefix === "@" ? "person" : "text",
          value: normalize(value),
        });
    }
    let cursor = 0;
    function unary(): Expr {
      const token = tokens[cursor++];
      if (!token) throw new Error("运算符后缺少条件");
      if (token.type === "NOT") return { type: "not", child: unary() };
      if (token.type === "(") {
        const expression = or();
        if (tokens[cursor++]?.type !== ")") throw new Error("缺少右括号 )");
        return expression;
      }
      if (token.type !== "term") throw new Error("此处需要搜索条件");
      return token;
    }
    function and(): Expr {
      let left = unary();
      while (
        cursor < tokens.length &&
        !["OR", ")"].includes(tokens[cursor].type)
      ) {
        if (tokens[cursor].type === "AND") cursor++;
        left = { type: "and", left, right: unary() };
      }
      return left;
    }
    function or(): Expr {
      let left = and();
      while (tokens[cursor]?.type === "OR") {
        cursor++;
        left = { type: "or", left, right: and() };
      }
      return left;
    }
    if (!tokens.length) return { expression: null, error: "" };
    const expression = or();
    if (cursor !== tokens.length) throw new Error("多余的右括号 )");
    return { expression, error: "" };
  } catch (e) {
    return { expression: null, error: (e as Error).message };
  }
}
export function matchesParsed(c: Cheki, parsed: ReturnType<typeof parseQuery>, people: Person[] = []) {
  if (parsed.error) return false;
  const text = normalize(
    [
      c.date,
      ...c.people,
      ...people.filter(p=>c.peopleIds?.includes(p.id)).flatMap(p=>[p.description,...p.aliases]),
      c.group,
      c.event,
      c.notes,
      c.shotType,
      ...c.tags,
      ...c.assets.flatMap((a) => [a.filename, a.originalFilename]),
    ].join(" "),
  );
  function evaluate(e: Expr): boolean {
    if (e.type === "not") return !evaluate(e.child);
    if (e.type === "and") return evaluate(e.left) && evaluate(e.right);
    if (e.type === "or") return evaluate(e.left) || evaluate(e.right);
    if (e.field === "tag") return c.tags.some((t) => normalize(t) === e.value);
    if (e.field === "person")
      return (
        c.shotType !== "团切" && (
          people.filter(p=>c.peopleIds?.includes(p.id)).some(p=>[p.name,personLabel(p),...p.aliases].some(n=>normalize(n)===e.value)) || c.people.some(p=>normalize(p)===e.value)
        )
      );
    return text.includes(e.value);
  }
  return !parsed.expression || evaluate(parsed.expression);
}
export function matches(c: Cheki, query: string) {
  return matchesParsed(c, parseQuery(query));
}
export function activeToken(query: string, caret: number) {
  // Walk tokens so @/# inside quoted names never start a second completion.
  const pattern =
    /([#@])(?:"(?:\\.|[^"\\])*"?|[^\s()]*)|"(?:\\.|[^"\\])*"?|[^\s()]+/g;
  for (const match of query.matchAll(pattern)) {
    const start = match.index!;
    if (match[1] && caret > start && caret <= start + match[0].length) {
      const part = query.slice(start + 1, caret);
      if (part.startsWith('"') && /(?<!\\)"$/.test(part.slice(1))) return null;
      return {
        start,
        end: start + match[0].length,
        prefix: match[1],
        term: part.replace(/^"/, "").replace(/\\(.)/g, "$1"),
      };
    }
  }
  return null;
}
export function quotedName(name: string, prefix: string) {
  return /[\s#@()"\\]/.test(name)
    ? `${prefix}"${name.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`
    : prefix + name;
}
export function quotedTag(tag: string) {
  return quotedName(tag, "#");
}
export function activeTag(query: string, caret: number) {
  const t = activeToken(query, caret);
  return t?.prefix === "#" ? { start: t.start, term: t.term } : null;
}
export function completeToken(query: string, caret: number, name: string) {
  const t = activeToken(query, caret);
  if (!t) return { value: query, caret };
  const prefix = query.slice(0, t.start) + quotedName(name, t.prefix) + " ";
  return {
    value: prefix + query.slice(t.end).replace(/^\s+/, ""),
    caret: prefix.length,
  };
}
export const completeTag = completeToken;
