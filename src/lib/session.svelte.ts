import { invoke, convertFileSrc, isTauri } from "@tauri-apps/api/core";
import { demoPhotos } from "./demo";
import { type Cheki, type Metadata, type Library, metadata } from "./model";
export const desktop = isTauri();
function samples(): Cheki[] {
  return demoPhotos.map((p, i) => ({
    id: p.id,
    date: p.date,
    people:
      i > 5
        ? []
        : [["示例人物 A", "示例人物 B", "示例人物 B", "示例人物 C"][i % 4]],
    event: "",
    tags: [p.tag],
    shotType: "其他",
    notes: "",
    favorite: p.favorite,
    coverAssetId: p.id + "-asset",
    demoTitle: p.title,
    crop: p.crop,
    source: p.source,
    assets: [
      {
        id: p.id + "-asset",
        kind: "scene",
        src: p.src,
        originalPath: p.src,
        filename: p.src.split("/").pop()!,
        originalFilename: p.src.split("/").pop()!,
        width: null,
        height: null,
        byteSize: 0,
        previewError: null,
        renditions: [],
      },
    ],
  }));
}
export const librarySession = $state<{
  photos: Cheki[];
  root: string;
  loaded: boolean;
  busy: boolean;
  error: string;
}>({
  photos: desktop ? [] : samples(),
  root: "",
  loaded: !desktop,
  busy: false,
  error: "",
});
function receive(library: Library) {
  librarySession.photos = library.chekis.map((c) => ({
    ...c,
    assets: c.assets.map((a) => ({
      ...a,
      src: a.src ? convertFileSrc(a.src) : "",
    })),
  }));
  librarySession.root = library.root;
  librarySession.loaded = true;
}
export async function loadLibrary() {
  if (!desktop || librarySession.loaded) return;
  try {
    receive(await invoke<Library>("library_load"));
  } catch (e) {
    librarySession.error = String(e);
  }
}
export async function saveCheki(id: string, value: Metadata) {
  if (desktop)
    receive(await invoke<Library>("cheki_update", { id, metadata: value }));
  else {
    const c = librarySession.photos.find((c) => c.id === id);
    if (c) Object.assign(c, structuredClone(value));
  }
}
export async function setFavorite(c: Cheki) {
  await saveCheki(c.id, { ...metadata(c), favorite: !c.favorite });
}
export async function importDesktop(chekiId: string | null) {
  librarySession.busy = true;
  try {
    const result = await invoke<{
      imported: number;
      errors: string[];
      library: Library;
    } | null>("import_photos", { chekiId });
    if (result) {
      receive(result.library);
      return `已导入 ${result.imported} 份影像${result.errors.length ? "\n" + result.errors.join("\n") : ""}`;
    }
    return "";
  } finally {
    librarySession.busy = false;
  }
}
export async function linkAsset(chekiId: string, assetId: string) {
  if (desktop)
    receive(await invoke<Library>("asset_link", { chekiId, assetId }));
  else {
    const target = librarySession.photos.find((c) => c.id === chekiId);
    const asset = librarySession.photos
      .flatMap((c) => c.assets)
      .find((a) => a.id === assetId);
    if (target && asset && !target.assets.some((a) => a.id === assetId))
      target.assets.push(asset);
  }
}
export async function retryPreview(assetId: string) {
  receive(await invoke<Library>("preview_retry", { assetId }));
}
