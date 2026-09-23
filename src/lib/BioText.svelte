<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { notify } from "./tasks.svelte";
  import { desktop } from "./session.svelte";
  import { bioParts } from "./bioLinks";
  let { text }: { text: string } = $props();
  async function follow(event: MouseEvent, href: string) {
    if (!desktop) return;
    event.preventDefault();
    try { await openUrl(href); }
    catch (e) { notify(String(e), "error"); }
  }
</script>

{#each bioParts(text) as part}
  {#if part.href}<a href={part.href} target="_blank" rel="noopener noreferrer"
      class="underline decoration-ink/25 underline-offset-4 hover:text-base-content"
      onclick={(e) => follow(e, part.href!)}>{part.text}</a>
  {:else}{part.text}{/if}
{/each}
