import { notify, startTask, updateTask, watchTasks } from "./tasks.svelte";
import { invoke, convertFileSrc, isTauri } from "@tauri-apps/api/core";
import { demoPhotos } from "./demo";
import { type Cheki, type Metadata, type Library, metadata } from "./model";
export const desktop = isTauri();
function samples(): Cheki[] {
  return demoPhotos
    .filter((p) => p.tag !== "返切")
    .map((p, i) => ({
      id: p.id,
      date: p.date,
      group: "",
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
  locations: import("./model").Location[];
  loaded: boolean;
  busy: boolean;
  error: string;
}>({
  photos: desktop ? [] : samples(),
  root: "",
  locations: [],
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
      baseSrc: a.baseSrc ? convertFileSrc(a.baseSrc) : "",
    })),
  }));
  librarySession.locations = library.locations;
  librarySession.root = library.root;
  librarySession.loaded = true;
}
export async function loadLibrary(force = false) {
  await watchTasks();
  if (!desktop || (librarySession.loaded && !force)) return;
  try {
    const result = await invoke<Library>("library_load");
    receive(result);
    librarySession.error = "";
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
export async function importDesktop(
  chekiId: string | null,
  options: { reference?: boolean; grouping?: string } = {},
  locationId: string | null = null,
) {
  await watchTasks();
  librarySession.busy = true;
  const taskId = startTask("导入影像");
  try {
    const result = await invoke<{
      imported: number;
      errors: string[];
      library: Library;
    } | null>("import_photos", {
      options: { chekiId, ...options },
      taskId,
      locationId,
    });
    if (result) {
      receive(result.library);
      const failed = result.library.chekis
        .flatMap((c) => c.assets)
        .filter((a) => a.previewError);
      if (failed.length)
        notify(`${failed.length} 份影像的预览生成失败。`, "error");
      return "";
    }
    updateTask({
      id: taskId,
      title: "导入影像",
      detail: "已取消",
      done: 0,
      total: 0,
      state: "done",
    });
    return "";
  } catch (e) {
    updateTask({
      id: taskId,
      title: "导入影像",
      detail: String(e),
      done: 0,
      total: 0,
      state: "error",
    });
    throw e;
  } finally {
    librarySession.busy = false;
  }
}
export async function catalogCommand(
  command: string,
  args: Record<string, unknown> = {},
) {
  const processing = ["asset_crop"].includes(command);
  const taskId = processing
    ? startTask(command === "asset_crop" ? "保存裁切" : "生成浏览图")
    : null;
  if (taskId)
    updateTask({
      id: taskId,
      title: "处理影像",
      detail: "正在读取影像并更新本机缓存…",
      done: 0,
      total: 0,
      state: "running",
    });
  try {
    const lib = await invoke<Library>(command, args);
    receive(lib);
    if (taskId) {
      const asset = lib.chekis
        .flatMap((c) => c.assets)
        .find((a) => a.id === args.assetId);
      updateTask({
        id: taskId,
        title: "处理影像",
        detail: asset?.previewError || "浏览图已更新",
        done: 1,
        total: 1,
        state: asset?.previewError ? "error" : "done",
      });
    }
  } catch (e) {
    // A filesystem batch may have completed some files before a later failure.
    try {
      receive(await invoke<Library>("library_load"));
    } catch {}
    if (taskId)
      updateTask({
        id: taskId,
        title: "处理影像",
        detail: String(e),
        done: 0,
        total: 1,
        state: "error",
      });
    else notify(String(e), "error");
    throw e;
  }
}
export async function addLocation(replace: string | null = null) {
  const result = await invoke<Library | null>("location_add", { replace });
  if (result) receive(result);
}
export async function trashChekis(ids: string[], restore = false) {
  if (desktop) await catalogCommand("cheki_trash", { ids, restore });
  else
    for (const c of librarySession.photos) {
      if (ids.includes(c.id))
        c.deletedAt = restore ? null : new Date().toISOString();
    }
  notify(restore ? "已恢复收藏" : `已将 ${ids.length} 张收藏移入回收站`);
}
