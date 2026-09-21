import { detectLocale, translate, translateMessage, type Locale } from "./i18n";
export { languages, type Locale } from "./i18n";
export const language = $state<{ current: Locale }>({ current: "zh-CN" });
export const tr = (key: string, args: readonly unknown[] = []) =>
  translate(language.current, key, args);
export const message = (value: unknown) =>
  translateMessage(
    language.current,
    String(value ?? "").replace(/^Error: /, ""),
  );
export function setLanguage(locale: Locale) {
  language.current = locale;
  if (typeof document !== "undefined") document.documentElement.lang = locale;
  try {
    localStorage.setItem("cheki-language", locale);
  } catch {
    /* Still applies for this session. */
  }
}
export function initLanguage() {
  let locale = detectLocale(
    navigator.languages?.length ? navigator.languages : [navigator.language],
  );
  try {
    const saved = localStorage.getItem("cheki-language");
    if (saved === "zh-CN" || saved === "en" || saved === "ja") locale = saved;
  } catch {}
  setLanguage(locale);
}

export function collectionTitle(c: import("./model").Cheki): string {
  return (
    (c.shotType === "团切"
      ? c.group
      : c.people.join(language.current === "en" ? ", " : "、")) ||
    c.demoTitle ||
    c.assets[0]?.originalFilename ||
    tr("未命名收藏")
  );
}
export function displayPerson(p: import("./model").Person): string {
  return (
    p.name +
    (p.description ? ` · ${p.description}` : "") +
    (p.deletedAt ? tr("（已删除）") : "")
  );
}

export const sourceMessage = (key: string, args: readonly unknown[] = []) =>
  translate("zh-CN", key, args);
