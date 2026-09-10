<script lang="ts">
  import { X, ScanLine, Camera } from "@lucide/svelte";
  import SelectMenu from "./SelectMenu.svelte";
  import { desktop, librarySession } from "./session.svelte";
  let {
    onsubmit,
    onclose,
    version = false,
    attached = false,
  }: {
    onsubmit: (options: {
      reference: boolean;
      kind: string;
      grouping: string;
    }) => void;
    onclose: () => void;
    version?: boolean;
    attached?: boolean;
  } = $props();
  let dialog: HTMLDialogElement;
  let kind = $state("scan");
  let storage = $state("copy");
  let grouping = $state("each");
  $effect(() => dialog?.showModal());
</script>

<dialog
  bind:this={dialog}
  {onclose}
  oncancel={(e) => e.stopPropagation()}
  class="modal bg-[#c7cbbd]/20 backdrop-blur-xl"
  aria-label="导入方式"
>
  <div class="modal-box glass-panel max-w-md rounded-3xl p-7 text-[#353a30]">
    <div class="flex items-center justify-between">
      <h2 class="text-lg">{version ? "添加同源文件版本" : "导入拍立得影像"}</h2>
      <button
        class="btn btn-ghost btn-circle btn-sm"
        aria-label="关闭导入"
        onclick={() => dialog.close()}><X size={18} /></button
      >
    </div>
    <p class="my-4 text-xs leading-6 text-black/50">
      {version
        ? "用于同一次扫描的 TIFF / JPEG 等质量版本。不同扫描或拍摄请添加为另一份影像。"
        : "扫描件和手机翻拍可以关联到同一张收藏。场景返切以后在人物下单独管理。"}
    </p>
    <div class="mb-5 grid grid-cols-2 gap-3">
      {#each [{ id: "scan", name: "扫描件", icon: ScanLine }, { id: "phone", name: "手机翻拍", icon: Camera }] as item}<button
          class={`btn h-20 flex-col rounded-2xl border-0 font-normal ${kind === item.id ? "bg-[#d4ddc9] shadow-sm" : "bg-white/20"}`}
          aria-pressed={kind === item.id}
          onclick={() => (kind = item.id)}
          ><item.icon size={21} />{item.name}</button
        >{/each}
    </div>
    {#if !version}<div class="mb-5">
        <p class="mb-2 text-xs text-black/50">这些文件如何组织</p>
        <SelectMenu
          label="导入组织方式"
          bind:value={grouping}
          options={[
            {
              value: "each",
              label: attached
                ? "为当前收藏添加多个影像"
                : "分别建立收藏，稍后归并",
            },
            { value: "collection", label: "同一收藏的不同影像" },
            { value: "versions", label: "同一影像的不同文件版本" },
          ]}
        />
      </div>{/if}
    <p class="mb-2 text-xs text-black/50">原件保存方式</p>
    <SelectMenu
      label="原件保存方式"
      bind:value={storage}
      options={[
        { value: "copy", label: "复制到本机图库" },
        { value: "reference", label: "引用已有目录（原件留在原处）" },
      ]}
    />
    {#if storage === "reference"}<p
        class="mt-3 text-[11px] leading-5 text-black/50"
      >
        先在设置中添加原件目录，再选择该目录内的文件。本机会自动保存浏览图。
      </p>{/if}
    <button
      class="btn glass-dark mt-6 w-full rounded-full font-normal text-white"
      disabled={desktop &&
        storage === "reference" &&
        librarySession.locations.length < 2}
      onclick={() => {
        onsubmit({
          reference: storage === "reference",
          kind,
          grouping: version ? "versions" : grouping,
        });
        dialog.close();
      }}>选择文件…</button
    >
  </div>
</dialog>
