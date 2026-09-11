import { title, type Cheki } from "./model";
export const sortOptions = [
  { value: "date", label: "拍摄日期" },
  { value: "name", label: "人物／团体名称" },
];
export function orderChekis(items: Cheki[], sort: string, descending: boolean) {
  return [...items].sort((a, b) => {
    if (sort === "date" && (!a.date || !b.date))
      return Number(!a.date) - Number(!b.date);
    const result =
      sort === "name"
        ? title(a).localeCompare(title(b), "zh-CN", { numeric: true })
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
