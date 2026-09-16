<script lang="ts">
  import { Search, X } from "@lucide/svelte";
  import { tick } from "svelte";
  import { activeToken, completeToken } from "./search";
  import { normalize, unique, personLabel, type Person } from "./model";
  let {
    value = $bindable(""),
    tags = [],
    people = [],
    error = "",
  }: {
    value: string;
    tags: string[];
    people?: Person[];
    error?: string;
  } = $props();
  let input: HTMLInputElement;
  let caret = $state(0);
  let focused = $state(false);
  let active = $state(0);
  const listId = $props.id();
  let token = $derived(activeToken(value, caret));
  let choices = $derived(token ? (token.prefix === "@" ? people.filter(p=>!p.deletedAt && [p.name,p.description,...p.aliases].some(n=>normalize(n).includes(normalize(token!.term)))).map(personLabel) : unique(tags).filter(t=>normalize(t).includes(normalize(token!.term)))).slice(0,7) : []);
  async function select(tag: string) {
    if (!token) return;
    const completed = completeToken(value, caret, tag);
    value = completed.value;
    caret = completed.caret;
    focused = false;
    await tick();
    input.focus();
    input.setSelectionRange(caret, caret);
    focused = false;
  }
  function key(e: KeyboardEvent) {
    if (e.isComposing) return;
    if (focused && choices.length) {
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        active =
          (active + (e.key === "ArrowDown" ? 1 : -1) + choices.length) %
          choices.length;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        select(choices[Math.min(active, choices.length - 1)]);
      }
      if (e.key === "Escape") {
        e.stopPropagation();
        focused = false;
      }
    }
  }
</script>

<div class="relative flex-1">
  <label
    class="input input-sm h-10 w-full rounded-full border-transparent bg-surface/35 px-4 shadow-[inset_0_1px_3px_#28301e12]"
    ><Search size={15} /><input
      bind:this={input}
      bind:value
      aria-label="搜索收藏"
      placeholder="@人物、#标签，或 AND / OR / NOT"
      aria-invalid={!!error}
      aria-describedby={error ? `${listId}-error` : undefined}
      role="combobox"
      aria-autocomplete="list"
      aria-expanded={focused && choices.length > 0}
      aria-controls={listId}
      aria-activedescendant={focused && choices[active]
        ? `${listId}-${active}`
        : undefined}
      oninput={() => {
        caret = input.selectionStart ?? value.length;
        active = 0;
        focused = true;
      }}
      onclick={() => {
        caret = input.selectionStart ?? value.length;
        focused = true;
      }}
      onkeyup={(e) => {
        if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) {
          caret = input.selectionStart ?? 0;
          active = 0;
          focused = true;
        }
      }}
      onfocus={() => {
        caret = input.selectionStart ?? value.length;
        focused = true;
      }}
      onblur={() => (focused = false)}
      onkeydown={key}
    />{#if value}<button
        class="btn btn-ghost btn-xs btn-circle"
        aria-label="清除搜索"
        onclick={() => {
          value = "";
          caret = 0;
        }}><X size={12} /></button
      >{/if}</label
  >
  {#if focused && choices.length}<div
      id={listId}
      role="listbox"
      aria-label={token?.prefix === "@" ? "搜索人物候选" : "搜索标签候选"}
      class="glass-panel absolute bottom-full left-0 right-0 z-50 mb-2 max-h-52 overflow-auto rounded-xl p-1.5"
    >
      {#each choices as choice, i}<button
          id={`${listId}-${i}`}
          role="option"
          aria-selected={active === i}
          class={`block w-full rounded-lg px-3 py-2 text-left text-xs ${active === i ? "bg-black/7" : ""}`}
          onpointerdown={(e) => e.preventDefault()}
          onclick={() => select(choice)}>{token?.prefix}{choice}</button
        >{/each}
    </div>{/if}
  {#if error}<p
      id={`${listId}-error`}
      role="status"
      class="mt-2 px-2 text-xs text-error"
    >
      {error}
    </p>{/if}
</div>
