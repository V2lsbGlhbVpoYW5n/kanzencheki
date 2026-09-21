import { describe, expect, it } from "vitest";
import { catalogs, detectLocale, translate, translateMessage } from "./i18n";
import enOne from "./locales/en-one.json";
import { shotTypes } from "./model";

const slots = (text: string) =>
  [...text.matchAll(/\{(\d+)\}/g)].map((m) => m[1]).sort();
describe("language catalogs", () => {
  it("covers all languages and preserves every interpolation parameter", () => {
    expect(Object.keys(catalogs.en).sort()).toEqual(
      Object.keys(catalogs.ja).sort(),
    );
    for (const catalog of [catalogs.en, catalogs.ja, enOne]) {
      for (const [key, value] of Object.entries(catalog)) {
        expect(value.trim(), key).not.toBe("");
        expect(slots(value), key).toEqual(slots(key));
      }
    }
    for (const type of shotTypes) expect(catalogs.en[type]).toBeDefined();
  });
  it("has entries for all literal UI message keys", () => {
    const sources = import.meta.glob("../**/*.{svelte,ts}", {
      eager: true,
      query: "?raw",
      import: "default",
    }) as Record<string, string>;
    for (const [path, text] of Object.entries(sources)) {
      if (path.endsWith(".test.ts")) continue;
      for (const m of text.matchAll(
        /\b(?:tr|sourceMessage)\(\s*(["'])(.*?)\1/g,
      )) {
        expect(catalogs.en[m[2]], `${path}: ${m[2]}`).toBeDefined();
      }
    }
  });
  it("follows supported language preferences and falls back to English", () => {
    expect(detectLocale(["ja-JP", "en-US"])).toBe("ja");
    expect(detectLocale(["zh-TW", "en"])).toBe("zh-CN");
    expect(detectLocale(["fr-FR", "en-GB"])).toBe("en");
    expect(detectLocale([])).toBe("en");
  });
  it("interpolates user content literally without treating it as another message or markup", () => {
    expect(translate("en", "打开人物 {0}", ["<小明> {1} $&"])).toBe(
      "Open person <小明> {1} $&",
    );
    expect(translate("ja", "已选 {0} 张", [3])).toBe("3枚選択中");
    expect(translate("zh-CN", "已选 {0} 张", [3])).toBe("已选 3 张");
    expect(translate("en", "unknown key")).toBe("unknown key");
  });
  it("handles English singular counts and backend messages without altering file paths", () => {
    expect(translate("en", "{0} 份影像", [1])).toBe("1 image");
    expect(translate("en", "{0} 份影像", [2])).toBe("2 images");
    expect(translateMessage("en", "已导入 1 份影像")).toBe("Imported 1 image");
    expect(translateMessage("ja", "原件离线，未保存修改：/相册/小明.jpg")).toBe(
      "原本がオフラインのため変更を保存できません：/相册/小明.jpg",
    );
    expect(
      translateMessage("en", "无法读取本机浏览图: Permission denied"),
    ).toBe("Could not read local preview: Permission denied");
    expect(translateMessage("en", "/照片/猫.jpg")).toBe("/照片/猫.jpg");
  });
});
