<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { desktop } from "./session.svelte";
  import { notify } from "./tasks.svelte";
  import FileCover from "./FileCover.svelte";
  import { X, File, Download, Play } from "@lucide/svelte";
  import type { PersonFile } from "./model";
  import { formatBytes, fileLabel } from "./model";
  let { file, onclose }: { file: PersonFile; onclose: () => void } = $props();
  let dialog: HTMLDialogElement;
  let failed = $state(false);
  const systemMedia =
    desktop &&
    typeof navigator !== "undefined" &&
    /Linux/.test(navigator.userAgent);
  $effect(() => dialog?.showModal());
  let image = $derived(
    [
      "image/jpeg",
      "image/png",
      "image/webp",
      "image/gif",
      "image/avif",
    ].includes(file.mimeType),
  );
</script>

<dialog
  bind:this={dialog}
  class="modal bg-scrim/35 backdrop-blur-2xl"
  aria-label="附件预览"
  {onclose}
  oncancel={(e) => e.stopPropagation()}
>
  <div
    class="modal-box glass-panel flex max-h-[88vh] w-[min(900px,90vw)] max-w-none flex-col gap-4 rounded-3xl p-6"
  >
    <header class="flex items-center gap-3">
      <div class="min-w-0 flex-1">
        <h2 class="truncate text-sm">{fileLabel(file)}</h2>
        <p class="mt-1 text-xs text-ink/40">
          {formatBytes(file.byteSize)} · {file.createdAt.slice(0, 10)} 添加
        </p>
      </div>
      <button
        class="btn btn-ghost btn-circle btn-sm"
        aria-label="关闭附件预览"
        onclick={() => dialog.close()}><X size={18} /></button
      >
    </header>
    {#if file.available && !failed}
      {#if image}<img
          src={file.src}
          alt={fileLabel(file)}
          class="min-h-0 max-h-[65vh] object-contain"
          onerror={() => (failed = true)}
        />
      {:else if systemMedia && (file.mimeType.startsWith("video/") || file.mimeType.startsWith("audio/"))}<div
          class="mx-auto h-52 w-72 overflow-hidden rounded-xl"
        >
          <FileCover {file} />
        </div>
        <button
          class="btn glass-dark btn-sm self-center rounded-full text-white"
          onclick={async () => {
            try {
              await invoke("person_media_open", {
                personId: file.personId,
                fileId: file.id,
              });
            } catch (e) {
              notify(String(e), "error");
            }
          }}><Play size={16} />使用系统播放器打开</button
        >
      {:else if file.mimeType.startsWith("video/")}<!-- svelte-ignore a11y_media_has_caption --><video
          src={file.src}
          controls
          class="max-h-[65vh] w-full rounded-xl bg-ink/5"
          onerror={() => (failed = true)}
        ></video>
      {:else if file.mimeType.startsWith("audio/")}<div
          class="grid min-h-52 place-items-center rounded-2xl bg-surface/20"
        >
          <audio
            src={file.src}
            controls
            class="w-full max-w-xl"
            onerror={() => (failed = true)}
          ></audio>
        </div>
      {:else}<div
          class="grid min-h-52 place-content-center gap-3 text-center text-ink/45"
        >
          <File size={40} class="mx-auto" />
          <p class="text-sm">此文件类型暂不支持内嵌预览</p>
        </div>{/if}
    {:else}<p class="p-10 text-center text-sm text-ink/50">
        {file.available
          ? "当前环境无法解码此文件，可保存后使用其他应用打开。"
          : "找不到附件原件。"}
      </p>{/if}
    {#if file.available}{#if desktop}<button
          class="btn btn-ghost btn-sm self-end rounded-full"
          onclick={async () => {
            try {
              await invoke("person_file_export", {
                personId: file.personId,
                fileId: file.id,
              });
            } catch (e) {
              notify(String(e), "error");
            }
          }}><Download size={14} />保存副本</button
        >{:else}<a
          class="btn btn-ghost btn-sm self-end rounded-full"
          href={file.src}
          download={fileLabel(file)}><Download size={14} />保存副本</a
        >{/if}{/if}
  </div>
</dialog>
