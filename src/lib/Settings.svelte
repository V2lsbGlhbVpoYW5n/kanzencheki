<script lang="ts">
  import { X, FolderPlus, HardDrive, RefreshCw } from "@lucide/svelte";
  import {
    librarySession,
    desktop,
    addLocation,
    importDesktop,
    loadLibrary,
    resumeCache,
  } from "./session.svelte";
  import { notify } from "./tasks.svelte";
  import TaskCenter from "./TaskCenter.svelte";
  let { onclose }: { onclose: () => void } = $props();
  let dialog: HTMLDialogElement;
  let working = $state(false);
  $effect(() => {
    dialog?.showModal();
  });
  async function add(id: string | null = null) {
    working = true;
    try {
      await addLocation(id);
    } catch (e) {
      notify(String(e), "error");
    } finally {
      working = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  {onclose}
  class="modal bg-[#c7cbbd]/20 backdrop-blur-xl"
  aria-label="图库设置"
>
  <div
    class="modal-box glass-panel w-[min(680px,90vw)] max-w-none rounded-3xl p-7 text-[#353a30]"
  >
    <header class="mb-6 flex items-center justify-between">
      <div>
        <h2 class="text-lg">图库设置</h2>
        <p class="mt-1 text-xs text-black/45">原件可以远行，收藏留在这里。</p>
      </div>
      <button
        class="btn btn-ghost btn-circle btn-sm"
        aria-label="关闭设置"
        onclick={() => dialog.close()}><X size={18} /></button
      >
    </header>
    <p class="mb-4 text-xs leading-6 text-black/55">
      本机保存数据库和浏览图。添加外部目录后，导入时可选择引用原件；目录离线不影响已有缓存和收藏资料。目录文件保持原位置。
    </p>
    {#each librarySession.locations as location}<div
        class="mb-3 rounded-2xl bg-white/25 p-4"
      >
        <div class="flex items-center gap-2 text-sm">
          <HardDrive size={16} />{location.name}<span
            class="ml-auto text-[10px] text-black/40"
            >{location.online ? "已连接" : "离线"}</span
          >
        </div>
        <p class="mt-2 break-all text-[11px] text-black/40">{location.path}</p>
        {#if !location.managed}<div class="mt-3 flex gap-2">
            <button
              class="btn btn-ghost btn-xs"
              disabled={!location.online || librarySession.busy}
              onclick={() =>
                importDesktop(
                  null,
                  { reference: true, kind: "scan" },
                  location.id,
                ).catch(() => {})}>扫描目录并登记照片</button
            ><button
              class="btn btn-ghost btn-xs"
              disabled={working || librarySession.busy}
              onclick={() => add(location.id)}>重新定位目录</button
            >
          </div>{/if}
      </div>{/each}
    {#if !desktop}<p class="rounded-xl bg-white/30 p-4 text-xs">
        浏览器为交互示例；文件夹设置请使用桌面版。
      </p>{/if}
    <div class="mt-5 flex flex-wrap gap-2">
      <button
        class="btn btn-sm glass-dark rounded-full text-white"
        disabled={!desktop || working || librarySession.busy}
        onclick={() => add()}><FolderPlus size={15} />添加原件目录</button
      ><button
        class="btn btn-ghost btn-sm rounded-full"
        disabled={!desktop || librarySession.busy}
        onclick={async () => {
          await loadLibrary(true);
          await resumeCache();
        }}><RefreshCw size={14} />刷新连接状态</button
      ><button
        class="btn btn-ghost btn-sm rounded-full"
        disabled={!desktop || librarySession.busy}
        onclick={resumeCache}>重试待处理缓存</button
      >
    </div>
    <p class="mt-5 text-[10px] text-black/40">
      扫描包含子文件夹，跳过符号链接。已登记文件不会重复导入；处理进度在左下角。
    </p>
  </div>
  <TaskCenter />
</dialog>
