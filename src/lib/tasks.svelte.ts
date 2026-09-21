import { listen } from "@tauri-apps/api/event";
import { isTauri } from "@tauri-apps/api/core";
export interface Task {
  id: string;
  title: string;
  detail: string;
  literalDetail?: boolean;
  done: number;
  total: number;
  state: "running" | "done" | "error";
}
export const taskState = $state<{
  items: Task[];
  latest: Task | null;
  open: boolean;
  shown: string;
}>({ items: [], latest: null, open: false, shown: "" });
export function updateTask(task: Task) {
  const index = taskState.items.findIndex((t) => t.id === task.id);
  if (index < 0) taskState.items.unshift(task);
  else taskState.items[index] = task;
  if (task.state !== "running") taskState.latest = task;
}
export function notify(detail: string, state: "done" | "error" = "done") {
  updateTask({
    id: crypto.randomUUID(),
    title: state === "error" ? "操作未完成" : "已完成",
    detail,
    state,
    done: 1,
    total: 1,
  });
}
export function startTask(title: string) {
  const id = crypto.randomUUID();
  updateTask({
    id,
    title,
    detail: "等待选择文件…",
    done: 0,
    total: 0,
    state: "running",
  });
  return id;
}
let subscribed = false;
export async function watchTasks() {
  if (!isTauri() || subscribed) return;
  await listen<Task>("library-progress", (e) => updateTask(e.payload));
  subscribed = true;
}
