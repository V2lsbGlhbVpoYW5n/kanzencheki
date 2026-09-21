import en from "./locales/en.json";
import ja from "./locales/ja.json";
import enOne from "./locales/en-one.json";

export type Locale = "zh-CN" | "en" | "ja";
export const languages: { value: Locale; label: string }[] = [
  { value: "zh-CN", label: "简体中文" },
  { value: "en", label: "English" },
  { value: "ja", label: "日本語" },
];
export const catalogs: Record<
  Exclude<Locale, "zh-CN">,
  Record<string, string>
> = { en, ja };
export function detectLocale(preferences: readonly string[]): Locale {
  for (const preference of preferences) {
    const language = preference.toLowerCase().split("-")[0];
    if (language === "zh") return "zh-CN";
    if (language === "ja" || language === "en") return language;
  }
  return "en";
}
/** Chinese source messages are stable keys; parameters are always plain text. */
export function translate(
  locale: Locale,
  key: string,
  args: readonly unknown[] = [],
): string {
  const singular =
    locale === "en" && Number(args[0]) === 1
      ? (enOne as Record<string, string>)[key]
      : undefined;
  const template =
    singular ?? (locale === "zh-CN" ? key : (catalogs[locale][key] ?? key));
  return template.replace(/\{(\d+)\}/g, (token, index) =>
    args[Number(index)] === undefined ? token : String(args[Number(index)]),
  );
}
const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const patterns = Object.keys(en)
  .filter((k) => /\{\d+\}/.test(k))
  .sort(
    (a, b) =>
      b.replace(/\{\d+\}/g, "").length - a.replace(/\{\d+\}/g, "").length,
  )
  .map((key) => ({
    key,
    regex: new RegExp(
      "^" +
        key
          .split(/\{\d+\}/)
          .map(escape)
          .join("(.*?)") +
        "$",
      "s",
    ),
  }));
const prefixes = Object.keys(en)
  .filter((k) => !k.includes("{"))
  .sort((a, b) => b.length - a.length);
/** Only use on application messages, never names, tags, document contents or paths. */
export function translateMessage(locale: Locale, message: string): string {
  if (locale === "zh-CN") return message;
  if (catalogs[locale][message]) return translate(locale, message);
  for (const { key, regex } of patterns) {
    const match = message.match(regex);
    if (match) return translate(locale, key, match.slice(1));
  }
  // anyhow appends technical causes after the contextual application message.
  const prefix = prefixes.find(
    (k) => message.startsWith(k + ": ") || message.startsWith(k + "："),
  );
  return prefix
    ? translate(locale, prefix) + message.slice(prefix.length)
    : message;
}
