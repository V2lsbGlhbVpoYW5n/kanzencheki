import { describe, expect, it } from "vitest";
import {
  activeTag,
  completeTag,
  matches,
  parseQuery,
  quotedTag,
  completeToken,
  activeToken,
} from "./search";
import { incomplete, metadata, type Cheki } from "./model";

const photo: Cheki = {
  id: "test",
  group: "",
  date: "2026-08-27",
  people: ["小明", "小蓝"],
  event: "生日公演",
  tags: ["夏日 演出", "Stage"],
  shotType: "多人切",
  notes: "第一排",
  favorite: false,
  coverAssetId: null,
  assets: [],
};

describe("tag queries", () => {
  it("combines exact tags and ordinary text with AND", () => {
    expect(matches(photo, '#Stage #"夏日 演出" 小明 公演')).toBe(true);
    expect(matches(photo, "#Stage #不存在")).toBe(false);
    expect(matches(photo, "#Sta")).toBe(false);
    expect(matches(photo, "#Stage 小红")).toBe(false);
  });
  it("round trips punctuation and whitespace in user entered tags", () => {
    for (const tag of ["夏日 演出", 'a"b', "a\\b", "a#b", "普通"]) {
      expect(matches({ ...photo, tags: [tag] }, quotedTag(tag))).toBe(true);
    }
  });
  it("completes only the tag at the caret", () => {
    expect(activeTag("小明 #夏", 5)).toEqual({ start: 3, term: "夏" });
    expect(activeTag('#"夏日 演', 6)?.term).toBe("夏日 演");
    expect(activeTag("普通文字", 4)).toBeNull();
    expect(completeTag('#"夏日 演出" #Stage', 3, "春日 公演").value).toBe(
      '#"春日 公演" #Stage',
    );
    expect(completeTag("#Sta #其他", 3, "Stage").value).toBe("#Stage #其他");
  });
});

it("requires both date and people for leaving inbox, independent of favorite", () => {
  expect(incomplete(photo)).toBe(false);
  expect(incomplete({ ...photo, date: "", favorite: true })).toBe(true);
  expect(incomplete({ ...photo, people: [] })).toBe(true);
});

it("editing a draft does not mutate persisted people or tags", () => {
  const draft = metadata(photo);
  draft.people.push("小红");
  draft.tags.splice(0);
  expect(photo.people).toEqual(["小明", "小蓝"]);
  expect(photo.tags).toHaveLength(2);
});

it("uses group instead of people to complete a group shot", () => {
  expect(
    incomplete({ ...photo, shotType: "团切", group: "测试团体", people: [] }),
  ).toBe(false);
  expect(
    incomplete({ ...photo, shotType: "团切", group: "", people: ["旧人物"] }),
  ).toBe(true);
  expect(
    metadata({ ...photo, shotType: "团切", group: "测试团体" }).people,
  ).toEqual([]);
  expect(
    matches({ ...photo, shotType: "团切", group: "测试团体" }, "测试团体"),
  ).toBe(true);
});

describe("logical queries and people", () => {
  it("uses NOT then AND then OR precedence, with grouping overrides", () => {
    expect(matches(photo, "@小明 OR @不存在 AND #不存在")).toBe(true);
    expect(matches(photo, "(@小明 OR @不存在) AND #不存在")).toBe(false);
    expect(matches(photo, "@小明 AND NOT #不存在")).toBe(true);
    expect(matches(photo, "NOT (@小明 OR #不存在)")).toBe(false);
    expect(matches(photo, "not not @小明")).toBe(true);
    expect(matches(photo, "@小明 NOT #不存在")).toBe(true);
  });
  it("matches exact people separately from notes and group names", () => {
    expect(matches(photo, "@小")).toBe(false);
    expect(matches(photo, "@小明 @小蓝")).toBe(true);
    expect(matches({ ...photo, people: [], notes: "小明" }, "@小明")).toBe(
      false,
    );
    expect(
      matches({ ...photo, shotType: "团切", group: "小明" }, "@小明"),
    ).toBe(false);
    expect(matches({ ...photo, people: ["AND"] }, "@AND")).toBe(true);
    expect(matches({ ...photo, notes: "AND" }, '"AND"')).toBe(true);
  });
  it("rejects incomplete or invalid expressions instead of widening results", () => {
    for (const q of [
      "@",
      "#",
      "@小明 OR",
      "AND @小明",
      "()",
      "(@小明",
      "@小明)",
      '#"未完成',
    ]) {
      expect(parseQuery(q).error, q).not.toBe("");
      expect(matches(photo, q), q).toBe(false);
    }
    expect(matches(photo, "")).toBe(true);
  });
  it("completes people inside parentheses without replacing neighboring conditions", () => {
    expect(completeToken("(@小 OR #Stage)", 3, "小明").value).toBe(
      "(@小明 OR #Stage)",
    );
    expect(completeToken("NOT @小蓝)", 6, "小 明").value).toBe(
      'NOT @"小 明" )',
    );
    expect(activeToken('#"a@b"', 5)?.prefix).toBe("#");
    expect(activeToken('"@小明"', 3)).toBeNull();
    expect(activeToken('@"小 明"', 6)).toBeNull();
  });
});
