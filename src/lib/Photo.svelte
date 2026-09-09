<script lang="ts">
  import type { Photo } from './demo';
  let { photo, full = false }: { photo: Photo; full?: boolean } = $props();
</script>
{#if photo.crop && !full}
  <div class="relative h-full w-full overflow-hidden" role="img" aria-label={photo.title}>
    <img src={photo.src} alt="" class="absolute max-w-none" style:width={`${100 / photo.crop.w * 100}%`} style:height={`${100 / photo.crop.h * 100}%`} style:left={`${-photo.crop.x / photo.crop.w * 100}%`} style:top={`${-photo.crop.y / photo.crop.h * 100}%`} />
  </div>
{:else}
  <img src={photo.src} alt={photo.title} class="h-full w-full object-contain" />
{/if}
