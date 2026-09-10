<script lang="ts">
  import { Check, ChevronDown } from "@lucide/svelte";
  import Floating from "./Floating.svelte";
  let {
    value = $bindable(""),
    options,
    label,
    onchange,
    disabled = false,
  }: {
    value: string;
    options: ({ value: string; label: string } | string)[];
    label: string;
    onchange?: (value: string) => void;
    disabled?: boolean;
  } = $props();
  let pop: Floating;
  let choices = $derived(
    options.map((o) => (typeof o === "string" ? { value: o, label: o } : o)),
  );
  function select(v: string) {
    value = v;
    onchange?.(v);
    pop.close();
  }
  function keys(e: KeyboardEvent) {
    const buttons = Array.from(
      (e.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>(
        '[role="option"]',
      ),
    );
    const i = buttons.indexOf(document.activeElement as HTMLButtonElement);
    let target = -1;
    if (e.key === "ArrowDown") target = (i + 1) % buttons.length;
    if (e.key === "ArrowUp") target = (i - 1 + buttons.length) % buttons.length;
    if (e.key === "Home") target = 0;
    if (e.key === "End") target = buttons.length - 1;
    if (target >= 0) {
      e.preventDefault();
      e.stopPropagation();
      buttons[target]?.focus();
    }
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      pop.close();
    }
  }
</script>

<Floating bind:this={pop} {label} {disabled}>
  {#snippet trigger()}<span class="flex-1 truncate text-left"
      >{choices.find((o) => o.value === value)?.label || label}</span
    ><ChevronDown size={13} />{/snippet}
  <ul
    class="menu w-full p-0 text-xs"
    aria-label={label}
    role="listbox"
    tabindex="-1"
    onkeydown={keys}
  >
    {#each choices as o}<li>
        <button
          type="button"
          role="option"
          aria-selected={o.value === value}
          class="flex justify-between rounded-xl"
          onclick={() => select(o.value)}
          >{o.label}{#if o.value === value}<Check size={13} />{/if}</button
        >
      </li>{/each}
  </ul>
</Floating>
