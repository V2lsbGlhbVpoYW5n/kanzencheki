<script lang="ts">
  import { tr } from "$lib/i18n.svelte";
  import { ImageOff } from "@lucide/svelte";
  type Photo = {
    src: string;
    title: string;
    crop?: { x: number; y: number; w: number; h: number };
  };
  let {
    photo,
    full = false,
    fit = "contain",
  }: { photo: Photo; full?: boolean; fit?: "cover" | "contain" } = $props();
  let width = $state(1),
    height = $state(1),
    iw = $state(1),
    ih = $state(1);
  let cw = $derived((iw * (photo.crop?.w ?? 100)) / 100),
    ch = $derived((ih * (photo.crop?.h ?? 100)) / 100);
  let scale = $derived(
    (fit === "cover" ? Math.max : Math.min)(width / cw, height / ch),
  );
</script>

{#if !photo.src}
  <div
    class="flex h-full w-full flex-col items-center justify-center gap-3 bg-ink/5 text-ink/45"
  >
    <ImageOff size={28} /><span class="text-xs"
      >{tr("原件已保存 · 暂无预览")}</span
    >
  </div>
{:else if photo.crop && !full}
  <div
    bind:clientWidth={width}
    bind:clientHeight={height}
    class="grid h-full w-full place-items-center overflow-hidden"
  >
    <div
      class="relative shrink-0 overflow-hidden"
      role="img"
      aria-label={photo.title}
      style:width={`${cw * scale}px`}
      style:height={`${ch * scale}px`}
    >
      <img
        src={photo.src}
        alt=""
        class="absolute max-w-none"
        style:width={`${10000 / photo.crop.w}%`}
        style:height={`${10000 / photo.crop.h}%`}
        style:left={`${(-photo.crop.x / photo.crop.w) * 100}%`}
        style:top={`${(-photo.crop.y / photo.crop.h) * 100}%`}
        onload={(e) => {
          iw = (e.currentTarget as HTMLImageElement).naturalWidth;
          ih = (e.currentTarget as HTMLImageElement).naturalHeight;
        }}
      />
    </div>
  </div>
{:else}
  <img
    src={photo.src}
    alt={photo.title}
    class="h-full w-full"
    style:object-fit={fit}
  />
{/if}
