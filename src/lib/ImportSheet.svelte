<script lang="ts">
  import { tr } from "$lib/i18n.svelte";
  import { X } from "@lucide/svelte";
  import SelectMenu from "./SelectMenu.svelte";
  import { desktop, librarySession } from "./session.svelte";
  let {
    onsubmit,
    onclose,
    attached = false,
  }: {
    onsubmit: (options: { reference: boolean; grouping: string }) => void;
    onclose: () => void;
    attached?: boolean;
  } = $props();
  let dialog: HTMLDialogElement;
  let storage = $state("copy");
  let grouping = $state("each");
  $effect(() => dialog?.showModal());
</script>

<dialog
  bind:this={dialog}
  {onclose}
  oncancel={(e) => e.stopPropagation()}
  class="modal bg-scrim/20 backdrop-blur-xl"
  aria-label={tr("导入方式")}
>
  <div class="modal-box glass-panel max-w-md rounded-3xl p-7 text-base-content">
    <div class="flex items-center justify-between">
      <h2 class="text-lg">{tr("导入拍立得影像")}</h2>
      <button
        class="btn btn-ghost btn-circle btn-sm"
        aria-label={tr("关闭导入")}
        onclick={() => dialog.close()}><X size={18} /></button
      >
    </div>
    <p class="my-4 text-xs leading-6 text-ink/50">
      {tr("每个文件是一份独立影像。同一张拍立得的多份影像可以放进一张收藏。")}
    </p>
    <div class="mb-5">
      <p class="mb-2 text-xs text-ink/50">{tr("这些文件如何组织")}</p>
      <SelectMenu
        label={tr("导入组织方式")}
        bind:value={grouping}
        options={[
          {
            value: "each",
            label: attached
              ? tr("为当前收藏添加多个影像")
              : tr("分别建立收藏，稍后归并"),
          },
          { value: "collection", label: tr("同一收藏的不同影像") },
        ]}
      />
    </div>
    <p class="mb-2 text-xs text-ink/50">{tr("原件保存方式")}</p>
    <SelectMenu
      label={tr("原件保存方式")}
      bind:value={storage}
      options={[
        { value: "copy", label: tr("复制到本机图库") },
        { value: "reference", label: tr("由图库管理已有目录中的原件") },
      ]}
    />
    {#if storage === "reference"}<p
        class="mt-3 text-[11px] leading-5 text-ink/50"
      >
        {tr(
          "先在设置中添加原件目录，再选择该目录内的文件。原件在原目录内按收藏信息重命名，本机保存浏览图。",
        )}
      </p>{/if}
    <button
      class="btn glass-dark mt-6 w-full rounded-full font-normal text-white"
      disabled={desktop &&
        storage === "reference" &&
        librarySession.locations.length < 2}
      onclick={() => {
        onsubmit({
          reference: storage === "reference",
          grouping,
        });
        dialog.close();
      }}>{tr("选择文件…")}</button
    >
  </div>
</dialog>
