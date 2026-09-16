import { expect, it } from "vitest";
import { orderChekis, nearestDate, meetingDates } from "./gallery-order";
import type { Cheki } from "./model";
const create = (id: string, date: string): Cheki => ({
  id,
  date,
  people: [id],
  group: "",
  event: "",
  tags: [],
  shotType: "solo",
  notes: "",
  favorite: false,
  coverAssetId: null,
  assets: [],
});
const items = [
  create("新导入", ""),
  create("第二张", "2026-08-27"),
  create("最早导入", "2026-08-01"),
];
it("keeps undated collections last in both chronological directions", () => {
  expect(orderChekis(items, "date", true).map((c) => c.id)).toEqual([
    "第二张",
    "最早导入",
    "新导入",
  ]);
  expect(orderChekis(items, "date", false).map((c) => c.id)).toEqual([
    "最早导入",
    "第二张",
    "新导入",
  ]);
});
it("reverses name order without mutating the source", () => {
  const names = [create("A", ""), create("C", ""), create("B", "")];
  expect(orderChekis(names, "name", false).map((c) => c.id)).toEqual([
    "A",
    "B",
    "C",
  ]);
  expect(orderChekis(names, "name", true).map((c) => c.id)).toEqual([
    "C",
    "B",
    "A",
  ]);
  expect(names.map((c) => c.id)).toEqual(["A", "C", "B"]);
});
it("navigates only within current results, choosing nearest dated collection", () => {
  expect(nearestDate(items, "2026-08-25")?.id).toBe("第二张");
  expect(nearestDate(items, "2026-08-01")?.id).toBe("最早导入");
  expect(nearestDate([items[0]], "2026-08-25")).toBeUndefined();
});
it("counts distinct active meeting dates, uses the maximum for multi-person shots and always puts groups last", () => {
  const a = { ...create("a", "2026-08-01"), peopleIds: ["a"] };
  const duplicate = { ...create("same day", "2026-08-01"), peopleIds: ["a"] };
  const b = { ...create("b", "2026-08-02"), peopleIds: ["b"] };
  const b2 = { ...create("b2", "2026-08-03"), peopleIds: ["b"] };
  const multi = {
    ...create("multi", "2026-08-02"),
    peopleIds: ["a", "b"],
    shotType: "多人切" as const,
  };
  const deleted = {
    ...create("deleted", "2026-08-04"),
    peopleIds: ["a"],
    deletedAt: "now",
  };
  const group = {
    ...create("group", "2026-08-05"),
    peopleIds: [],
    shotType: "团切" as const,
  };
  const c = { ...create("c", "2026-08-06"), peopleIds: ["c"] };
  const all = [a, duplicate, b, b2, multi, deleted, group, c];
  expect(meetingDates(all).get("a")?.size).toBe(2);
  expect(meetingDates(all).get("b")?.size).toBe(2);
  // Both a and b have two distinct meeting days; their union has three.
  expect(orderChekis([multi, b], "meetings", false, all).map((c) => c.id))
    .toEqual(["multi", "b"]);
  expect(
    orderChekis([group, c, multi], "meetings", true, all).map((c) => c.id),
  ).toEqual(["multi", "c", "group"]);
  expect(
    orderChekis([group, multi, c], "meetings", false, all).map((c) => c.id),
  ).toEqual(["c", "multi", "group"]);
});
