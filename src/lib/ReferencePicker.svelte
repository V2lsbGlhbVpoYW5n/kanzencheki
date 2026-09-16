<script lang="ts">
  import { onMount } from "svelte";
  import { Search, X, Images, Paperclip, Check } from "@lucide/svelte";
  import PhotoImage from "./Photo.svelte";
  import { coverPhoto } from "./model";
  import { librarySession, desktop } from "./session.svelte";
  import { fileLabel, title, type PersonFile } from "./model";
  import FileCover from "./FileCover.svelte";
  let {
    files,
    onselect,
    onclose,
  }: {
    files: PersonFile[];
    onselect: (uri: string, label: string) => void;
    onclose: () => void;
  } = $props();
  let dialog: HTMLDialogElement;
  let tab = $state("cheki"),
    query = $state(""),
    selected = $state<{ uri: string; label: string } | null>(null);
  let chekis = $derived(
    librarySession.photos.filter(
      (c) =>
        !c.deletedAt &&
        [
          title(c),
          c.date,
          c.event,
          ...c.tags,
          ...c.assets.map((a) => a.filename),
        ]
          .join(" ")
          .toLowerCase()
          .includes(query.toLowerCase()),
    ),
  );
  let attachments = $derived(
    files.filter((f) =>
      fileLabel(f).toLowerCase().includes(query.toLowerCase()),
    ),
  );
  onMount(() => dialog.showModal());
</script>

<dialog
  bind:this={dialog}
  class="modal bg-scrim/30 backdrop-blur-xl"
  aria-label="选择引用"
  oncancel={(e) => {
    e.preventDefault();
    e.stopPropagation();
    onclose();
  }}
>
  <div
    class="modal-box glass-panel flex max-h-[85vh] w-[min(900px,94vw)] max-w-none flex-col gap-5 rounded-3xl p-6"
  >
    <header class="flex items-center gap-3">
      <h2 class="flex-1 text-base">选择引用</h2>
      <button
        class="btn btn-ghost btn-sm btn-circle"
        aria-label="关闭引用选择"
        onclick={onclose}><X size={18} /></button
      >
    </header>
    <div class="flex items-center gap-2">
      <button
        class={`btn btn-sm rounded-full ${tab === "cheki" ? "glass-dark text-white" : "btn-ghost"}`}
        onclick={() => {
          tab = "cheki";
          selected = null;
        }}><Images size={15} />拍立得</button
      ><button
        class={`btn btn-sm rounded-full ${tab === "attachment" ? "glass-dark text-white" : "btn-ghost"}`}
        onclick={() => {
          tab = "attachment";
          selected = null;
        }}><Paperclip size={15} />附件</button
      ><label class="input input-sm ml-auto rounded-full border-0 bg-surface/30"
        ><Search size={14} /><input
          aria-label="搜索引用"
          placeholder="日期、名字、文件名…"
          bind:value={query}
        /></label
      >
    </div>
    <div
      class="grid min-h-48 grid-cols-3 gap-4 overflow-auto p-1 sm:grid-cols-4"
    >
      {#if tab === "cheki"}{#each chekis as c}{@const a =
            c.assets.find((a) => a.id === c.coverAssetId) ?? c.assets[0]}<button
            class={`rounded-xl p-2 text-left transition ${selected?.uri === `cheki:${c.id}` ? "bg-[#8c9d72]/20 ring-2 ring-[#8c9d72]" : "bg-surface/25 hover:bg-surface/50"}`}
            aria-label={`引用 ${c.date} ${title(c)}`}
            aria-pressed={selected?.uri === `cheki:${c.id}`}
            onclick={() =>
              (selected = {
                uri: `cheki:${c.id}`,
                label: `${c.date || "未定日期"} · ${title(c)}`,
              })}
            ><div class="grid aspect-[3/4] place-items-center">
              {#if a?.src}<PhotoImage
                  photo={coverPhoto(c, desktop)}
                />{:else}<Images size={30} />{/if}
            </div>
            <p class="mt-2 truncate text-xs">
              {c.date || "未定日期"} · {title(c)}
            </p>
            <p class="mt-1 truncate text-[10px] text-ink/40">
              {a?.filename ?? c.id}
            </p></button
          >{:else}<p class="col-span-full p-10 text-center text-sm text-ink/40">
            没有匹配的收藏
          </p>{/each}
      {:else}{#each attachments as f}<button
            class={`overflow-hidden rounded-xl text-left ${selected?.uri === `attachment:${f.id}` ? "ring-2 ring-[#8c9d72]" : "bg-surface/25"}`}
            aria-label={`引用附件 ${fileLabel(f)}`}
            aria-pressed={selected?.uri === `attachment:${f.id}`}
            onclick={() =>
              (selected = { uri: `attachment:${f.id}`, label: fileLabel(f) })}
            ><div class="aspect-square"><FileCover file={f} /></div>
            <p class="truncate p-3 text-xs">{fileLabel(f)}</p></button
          >{:else}<p class="col-span-full p-10 text-center text-sm text-ink/40">
            没有匹配的附件
          </p>{/each}{/if}
    </div>
    <footer class="flex items-center justify-between gap-3">
      <p class="truncate text-xs text-ink/45">
        {selected?.label ?? "选择一张收藏或一个附件"}
      </p>
      <button
        class="btn glass-dark btn-sm shrink-0 rounded-full text-white"
        disabled={!selected}
        onclick={() => {
          if (selected) onselect(selected.uri, selected.label);
        }}><Check size={14} />插入引用</button
      >
    </footer>
  </div>
</dialog>
