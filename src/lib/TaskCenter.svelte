<script lang="ts">
  import { Activity, Check, X, AlertCircle } from "@lucide/svelte";
  import Floating from "./Floating.svelte";
  import { taskState } from "./tasks.svelte";
  let toast: HTMLDivElement;
  let current = $state("");
  let active = $derived(taskState.items.filter((t) => t.state === "running"));
  $effect(() => {
    const t = taskState.latest;
    if (t && t.id !== taskState.shown && toast) {
      taskState.shown = t.id;
      toast.showPopover();
      const timer = setTimeout(() => toast.hidePopover(), 5000);
      return () => clearTimeout(timer);
    }
  });
</script>

<div class="pointer-events-auto fixed bottom-7 left-6 z-50">
  <Floating
    label="任务与消息"
    buttonClass="btn glass-panel btn-circle border-0"
    wide
  >
    {#snippet trigger()}{#if active.length}<span
          class="loading loading-ring loading-sm"
        ></span>{:else}<Activity size={18} />{/if}{/snippet}
    <div class="flex items-center justify-between px-3 py-2">
      <h2 class="text-sm">任务与消息</h2>
      <button
        class="btn btn-ghost btn-xs"
        onclick={() =>
          (taskState.items = taskState.items.filter(
            (t) => t.state === "running",
          ))}>清除已完成</button
      >
    </div>
    {#each taskState.items as t}<div
        class="space-y-2 border-t border-black/5 p-3"
      >
        <div class="flex items-center gap-2 text-xs">
          {#if t.state === "running"}<span
              class="loading loading-spinner loading-xs"
            ></span>{:else if t.state === "error"}<AlertCircle
              size={14}
            />{:else}<Check size={14} />{/if}{t.title}
        </div>
        <p class="break-words whitespace-pre-wrap text-[11px] text-black/50">
          {t.detail}
        </p>
        {#if t.state === "running"}<progress
            class="progress h-1 w-full"
            value={t.total ? t.done : undefined}
            max={t.total || 1}
          ></progress>
          <p class="text-[10px] text-black/40">
            {t.total ? `${t.done} / ${t.total} · 当前文件处理中` : "等待操作"}
          </p>{/if}
      </div>{:else}<p class="p-5 text-xs text-black/45">
        所有操作消息会留在这里
      </p>{/each}
  </Floating>
</div>
<div
  bind:this={toast}
  popover="manual"
  class="glass-panel fixed top-auto right-auto bottom-24 left-6 m-0 max-w-80 rounded-2xl border-0 px-4 py-3 text-[#353a30] shadow-xl"
  role="status"
>
  <div class="flex items-start gap-3">
    <p class="whitespace-pre-wrap text-xs">{taskState.latest?.detail}</p>
    <button
      class="btn btn-ghost btn-xs btn-circle shrink-0"
      aria-label="关闭消息"
      onclick={() => toast.hidePopover()}><X size={12} /></button
    >
  </div>
</div>
