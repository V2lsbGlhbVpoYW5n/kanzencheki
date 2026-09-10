<script lang="ts">
  import { ImageOff } from "@lucide/svelte";
  type Photo = {
    src: string;
    title: string;
    crop?: { x: number; y: number; w: number; h: number };
  };
  let { photo, full = false }: { photo: Photo; full?: boolean } = $props();
</script>

{#if !photo.src}
  <div
    class="flex h-full w-full flex-col items-center justify-center gap-3 bg-black/5 text-black/45"
  >
    <ImageOff size={28} /><span class="text-xs">原件已保存 · 暂无预览</span>
  </div>
{:else if photo.crop && !full}
  <div
    class="relative h-full w-full overflow-hidden"
    role="img"
    aria-label={photo.title}
  >
    <img
      src={photo.src}
      alt=""
      class="absolute max-w-none"
      style:width={`${(100 / photo.crop.w) * 100}%`}
      style:height={`${(100 / photo.crop.h) * 100}%`}
      style:left={`${(-photo.crop.x / photo.crop.w) * 100}%`}
      style:top={`${(-photo.crop.y / photo.crop.h) * 100}%`}
    />
  </div>
{:else}
  <img src={photo.src} alt={photo.title} class="h-full w-full object-contain" />
{/if}
