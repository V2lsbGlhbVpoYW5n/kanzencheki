export const shotTypes = ["solo", "2 shot", "多人切", "团切", "其他"] as const;
export type ShotType = (typeof shotTypes)[number];
export interface Metadata {
  date: string;
  people: string[];
  event: string;
  tags: string[];
  shotType: ShotType;
  notes: string;
  favorite: boolean;
}
export interface Rendition {
  id: string;
  role: "original" | "display" | "thumbnail";
  relativePath: string;
  mimeType: string;
  byteSize: number;
}
export interface Asset {
  id: string;
  kind: string;
  src: string;
  originalPath: string;
  filename: string;
  originalFilename: string;
  width: number | null;
  height: number | null;
  byteSize: number;
  previewError: string | null;
  renditions: Rendition[];
}
export interface Cheki extends Metadata {
  id: string;
  coverAssetId: string | null;
  assets: Asset[];
  // Browser-only sample cover, never sent as metadata to the backend.
  demoTitle?: string;
  crop?: { x: number; y: number; w: number; h: number };
  source?: string;
}
export interface Library {
  root: string;
  chekis: Cheki[];
}
export function incomplete(c: Metadata) {
  return !c.date || c.people.length === 0;
}
export function title(c: Cheki) {
  return (
    c.people.join("、") ||
    c.demoTitle ||
    c.assets[0]?.originalFilename ||
    "未命名收藏"
  );
}
export function normalize(value: string) {
  return value.trim().normalize("NFKC").toLocaleLowerCase();
}
export function unique(values: string[]) {
  const seen = new Set<string>();
  return values
    .map((s) => s.trim())
    .filter((s) => s && !seen.has(normalize(s)) && seen.add(normalize(s)));
}
export function metadata(c: Metadata): Metadata {
  return {
    date: c.date,
    people: [...c.people],
    event: c.event,
    tags: [...c.tags],
    shotType: c.shotType,
    notes: c.notes,
    favorite: c.favorite,
  };
}
export function formatBytes(n: number) {
  return n >= 1024 ** 3
    ? `${(n / 1024 ** 3).toFixed(2)} GB`
    : n >= 1024 ** 2
      ? `${(n / 1024 ** 2).toFixed(1)} MB`
      : `${(n / 1024).toFixed(1)} KB`;
}
