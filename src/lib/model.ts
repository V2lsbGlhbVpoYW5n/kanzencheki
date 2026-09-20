export const shotTypes = ["solo", "2 shot", "多人切", "团切", "其他"] as const;
export type ShotType = (typeof shotTypes)[number];
export interface Metadata {
  date: string;
  people: string[];
  peopleIds?: string[];
  group: string;
  event: string;
  tags: string[];
  shotType: ShotType;
  notes: string;
  favorite: boolean;
}
export interface Rendition {
  id: string;
  role: string;
  locationId?: string;
  available?: boolean;
  width?: number | null;
  height?: number | null;
  relativePath: string;
  mimeType: string;
  byteSize: number;
}
export interface Asset {
  id: string;
  baseSrc?: string;
  crop?: Crop | null;
  fingerprint?: string | null;
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
  deletedAt?: string | null;
  coverManual?: boolean;
  reviewFaces?: number | null;
  assets: Asset[];
  // Browser-only sample cover, never sent as metadata to the backend.
  demoTitle?: string;
  crop?: { x: number; y: number; w: number; h: number };
  source?: string;
}
export interface Library {
  root: string;
  chekis: Cheki[];
  locations: Location[];
  people: Person[];
}
export function incomplete(c: Metadata) {
  return (
    !c.date || (c.shotType === "团切" ? !c.group.trim() : c.people.length === 0)
  );
}
export function title(c: Cheki) {
  return (
    (c.shotType === "团切" ? c.group : c.people.join("、")) ||
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
    people: c.shotType === "团切" ? [] : [...c.people],
    peopleIds:
      c.shotType === "团切" ? [] : c.peopleIds ? [...c.peopleIds] : undefined,
    group: c.group || "",
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

export interface Point {
  x: number;
  y: number;
}
export interface Crop {
  quad?: [Point, Point, Point, Point] | null;
  ratio?: number | null;
  x: number;
  y: number;
  w: number;
  h: number;
}
export interface Location {
  id: string;
  path: string;
  name: string;
  online: boolean;
  managed: boolean;
}

export interface Person {
  id: string;
  name: string;
  description: string;
  aliases: string[];
  notes: string;
  deletedAt: string | null;
  createdAt: string;
}
export interface PersonDraft {
  id?: string;
  name: string;
  description: string;
  aliases: string[];
  notes: string;
  allowDuplicate?: boolean;
}
export interface PersonFile {
  id: string;
  personId: string;
  filename: string;
  originalFilename: string;
  src: string;
  mimeType: string;
  byteSize: number;
  createdAt: string;
  available: boolean;
}
export interface PersonDocument {
  id: string;
  personId: string;
  title: string;
  filename: string;
  updatedAt: string;
}
export interface PersonSpace {
  files: PersonFile[];
  documents: PersonDocument[];
}
export function personLabel(p: Person) {
  return (
    p.name +
    (p.description ? ` · ${p.description}` : "") +
    (p.deletedAt ? "（已删除）" : "")
  );
}

/** The visible name is the actual filename; the import name remains separate metadata. */
export function fileLabel(file: PersonFile): string {
  return file.filename;
}

export function coverPhoto(c: Cheki, desktop: boolean) {
  const a = c.assets.find((a) => a.id === c.coverAssetId) || c.assets[0];
  return {
    src: a?.src || "",
    title: title(c),
    crop:
      !desktop && a?.crop
        ? {
            x: a.crop.x * 100,
            y: a.crop.y * 100,
            w: a.crop.w * 100,
            h: a.crop.h * 100,
          }
        : a?.crop === null
          ? undefined
          : c.crop,
  };
}
