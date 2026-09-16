import { invoke } from "@tauri-apps/api/core";
import type { PersonFile } from "./model";
// One decoder at a time; no persisted jobs or full-size copies.
let queue: Promise<unknown> = Promise.resolve();
export function videoCover(file: PersonFile): Promise<number[]> {
  const result = queue.then(() =>
    invoke<number[]>("person_video_cover", {
      personId: file.personId,
      fileId: file.id,
    }),
  );
  queue = result.catch(() => {});
  return result;
}
