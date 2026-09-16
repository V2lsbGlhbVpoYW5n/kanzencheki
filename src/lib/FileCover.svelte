<script lang="ts">
  import { File, Music, Video } from "@lucide/svelte";
  import { desktop } from "./session.svelte";
  import { videoCover } from "./videoCover";
  import type { PersonFile } from "./model";
  let { file }: { file: PersonFile } = $props();
  let failed = $state(false),
    poster = $state("");
  $effect(() => {
    const current = file;
    let disposed = false,
      url = "";
    failed = false;
    poster = "";
    if (desktop && current.available && current.mimeType.startsWith("video/"))
      videoCover(current)
        .then((bytes) => {
          if (!disposed) {
            url = URL.createObjectURL(
              new Blob([new Uint8Array(bytes)], { type: "image/jpeg" }),
            );
            poster = url;
          }
        })
        .catch(() => {
          if (!disposed) failed = true;
        });
    return () => {
      disposed = true;
      if (url) URL.revokeObjectURL(url);
    };
  });
</script>

<div
  class="relative grid h-full w-full place-items-center overflow-hidden bg-tint/25 text-accent-ink"
>
  {#if file.available && !failed && file.mimeType.startsWith("image/")}<img
      src={file.src}
      alt=""
      class="h-full w-full object-cover"
      loading="lazy"
      onerror={() => (failed = true)}
    />
  {:else if file.available && !failed && file.mimeType.startsWith("video/")}
    {#if desktop}{#if poster}<img
          src={poster}
          alt=""
          class="h-full w-full object-cover"
        />{:else}<Video size={24} />{/if}{:else}
      <!-- The paused video presents its first decoded frame without making another stored file. -->
      <!-- svelte-ignore a11y_media_has_caption -->
      <video
        src={file.src}
        muted
        playsinline
        preload="auto"
        class="h-full w-full object-cover"
        onloadeddata={(e) => {
          e.currentTarget.pause();
          e.currentTarget.currentTime = 0;
        }}
        onerror={() => (failed = true)}
      ></video>{/if}
    <span
      class="absolute bottom-2 right-2 rounded-full bg-black/35 p-1.5 text-white"
      ><Video size={14} /></span
    >
  {:else if file.mimeType.startsWith("audio/")}<Music
      size={32}
      strokeWidth={1}
    />
  {:else if file.mimeType.startsWith("video/")}<Video
      size={32}
      strokeWidth={1}
    />
  {:else}<File size={32} strokeWidth={1} />{/if}
</div>
