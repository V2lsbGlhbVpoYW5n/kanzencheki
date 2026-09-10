import { describe, expect, it } from "vitest";
import {
  activeTag,
  completeTag,
  matches,
  parseQuery,
  quotedTag,
} from "./search";
import { incomplete, metadata, type Cheki } from "./model";

const photo: Cheki = {
  id: "test",
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
      expect(parseQuery(quotedTag(tag))).toEqual({ tags: [tag], words: [] });
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
