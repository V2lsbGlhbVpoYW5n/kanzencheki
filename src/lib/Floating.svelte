<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    label,
    children,
    trigger,
    wide = false,
    buttonClass = "input input-sm w-full border-0 bg-surface/25 text-xs",
    disabled = false,
  }: {
    label: string;
    children: Snippet;
    trigger?: Snippet;
    wide?: boolean;
    buttonClass?: string;
    disabled?: boolean;
  } = $props();
  const id = $props.id();
  let panel: HTMLDivElement;
  let button: HTMLButtonElement;
  let left = $state(0);
  let top = $state(0);
  function position() {
    const r = button.getBoundingClientRect();
    const w = wide ? 320 : 280;
    left = Math.max(12, Math.min(r.left, window.innerWidth - w - 12));
    top = r.bottom + 6;
  }
  export function close() {
    panel?.hidePopover();
    button?.focus({ preventScroll: true });
  }
  function opened(e: ToggleEvent) {
    if (e.newState === "open") {
      panel
        .querySelector<HTMLButtonElement>(
          '[role="option"][aria-selected="true"]',
        )
        ?.focus({ preventScroll: true });
      const h = panel.getBoundingClientRect().height;
      const r = button.getBoundingClientRect();
      if (top + h > window.innerHeight - 12) top = Math.max(12, r.top - h - 6);
    }
  }
</script>

<button
  bind:this={button}
  type="button"
  class={buttonClass}
  aria-label={label}
  popovertarget={id}
  onclick={position}
  {disabled}
  >{#if trigger}{@render trigger()}{:else}{label}{/if}</button
>
<div
  bind:this={panel}
  {id}
  popover="auto"
  ontoggle={opened}
  class="glass-panel right-auto bottom-auto m-0 max-h-[min(420px,80vh)] overflow-auto rounded-2xl border-0 p-2 text-base-content shadow-xl"
  style:position="fixed"
  style:left={`${left}px`}
  style:top={`${top}px`}
  style:width={wide ? "320px" : "280px"}
>
  {@render children()}
</div>
