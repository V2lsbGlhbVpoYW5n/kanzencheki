import { title, type Cheki } from "./model";
export const sortOptions = [
  { value: "date", label: "拍摄日期" },
  { value: "name", label: "人物／团体名称" },
  { value: "meetings", label: "见面天数" },
];
export function meetingDates(items: Cheki[]) {
  const dates = new Map<string, Set<string>>();
  for (const c of items)
    if (!c.deletedAt && c.date && c.shotType !== "团切")
      for (const id of c.peopleIds ?? []) {
        if (!dates.has(id)) dates.set(id, new Set());
        dates.get(id)!.add(c.date);
      }
  return dates;
}
export function orderChekis(
  items: Cheki[],
  sort: string,
  descending: boolean,
  allItems: Cheki[] = items,
  locale = "zh-CN",
) {
  const dates = meetingDates(allItems);
  const meetings = (c: Cheki) =>
    c.shotType === "团切"
      ? 0
      : Math.max(
          0,
          ...(c.peopleIds ?? []).map((id) => dates.get(id)?.size ?? 0),
        );
  return [...items].sort((a, b) => {
    if (sort === "meetings" && (a.shotType === "团切" || b.shotType === "团切"))
      return Number(a.shotType === "团切") - Number(b.shotType === "团切");
    if (sort === "date" && (!a.date || !b.date))
      return Number(!a.date) - Number(!b.date);
    const result =
      sort === "meetings"
        ? meetings(a) - meetings(b)
        : sort === "name"
          ? title(a).localeCompare(title(b), locale, { numeric: true })
          : a.date.localeCompare(b.date);
    // Stable sorting preserves existing order for equal values.
    return result * (descending ? -1 : 1);
  });
}
export function nearestDate(items: Cheki[], date: string) {
  return items
    .filter((c) => c.date)
    .reduce<Cheki | undefined>(
      (best, c) =>
        !best ||
        Math.abs(Date.parse(c.date) - Date.parse(date)) <
          Math.abs(Date.parse(best.date) - Date.parse(date))
          ? c
          : best,
      undefined,
    );
}
