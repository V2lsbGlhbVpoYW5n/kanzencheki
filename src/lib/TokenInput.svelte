<script lang="ts">
  import { X, Plus } from "@lucide/svelte";
  import { normalize, unique } from "./model";
  let {
    values = $bindable<string[]>([]),
    suggestions = [],
    label,
    placeholder = "输入后按 Enter 添加",
    prefix = "",
  }: {
    values: string[];
    suggestions: string[];
    label: string;
    placeholder?: string;
    prefix?: string;
  } = $props();
  let text = $state("");
  let focused = $state(false);
  let active = $state(0);
  const listId = $props.id();
  let candidates = $derived(
    unique(suggestions)
      .filter(
        (s) =>
          !values.some((v) => normalize(v) === normalize(s)) &&
          normalize(s).includes(normalize(text)),
      )
      .slice(0, 6),
  );
  let canCreate = $derived(
    !!text.trim() &&
      ![...suggestions, ...values].some(
        (v) => normalize(v) === normalize(text),
      ),
  );
  let choices = $derived([...candidates, ...(canCreate ? [text.trim()] : [])]);
  function add(value: string) {
    values = unique([...values, value]);
    text = "";
    active = 0;
  }
  export function flush() {
    if (text.trim()) add(text);
  }
  function key(e: KeyboardEvent) {
    if (e.isComposing) return;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      focused = true;
      active =
        (active +
          (e.key === "ArrowDown" ? 1 : -1) +
          Math.max(choices.length, 1)) %
        Math.max(choices.length, 1);
    } else if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
      const value = choices[active] || text;
      if (value.trim()) add(value);
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      focused = false;
    } else if (e.key === "Backspace" && !text && values.length) {
      values = values.slice(0, -1);
    }
  }
</script>

<div class="relative">
  <div
    class="flex flex-wrap items-center gap-1.5 rounded-xl bg-surface/25 p-2 shadow-[inset_0_1px_3px_#28301e12] focus-within:ring-1 focus-within:ring-[#8b977d]/40"
  >
    {#each values as value}<span
        class="flex max-w-full items-center gap-1 rounded-md bg-tint/65 px-2 py-1 text-[11px]"
        ><span class="truncate">{prefix}{value}</span><button
          type="button"
          class="shrink-0 rounded hover:bg-ink/10"
          aria-label={`移除${label} ${value}`}
          onclick={() => (values = values.filter((v) => v !== value))}
          ><X size={11} /></button
        ></span
      >{/each}
    <input
      class="min-w-16 flex-1 bg-transparent p-1 text-xs outline-none"
      aria-label={label}
      role="combobox"
      aria-autocomplete="list"
      aria-expanded={focused && choices.length > 0}
      aria-controls={listId}
      aria-activedescendant={focused && choices[active]
        ? `${listId}-${active}`
        : undefined}
      {placeholder}
      bind:value={text}
      oninput={() => {
        active = 0;
        focused = true;
      }}
      onfocus={() => (focused = true)}
      onblur={() => {
        flush();
        focused = false;
      }}
      onkeydown={key}
    />
  </div>
  {#if focused && choices.length > 0}<div
      id={listId}
      role="listbox"
      aria-label={`${label}候选`}
      class="glass-panel absolute left-0 right-0 top-full z-50 mt-1 max-h-44 overflow-auto rounded-xl p-1.5"
    >
      {#each choices as choice, i}<button
          type="button"
          id={`${listId}-${i}`}
          role="option"
          aria-selected={active === i}
          class={`flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-xs ${active === i ? "bg-black/7" : ""}`}
          onpointerdown={(e) => e.preventDefault()}
          onclick={() => add(choice)}
          >{#if i === candidates.length && canCreate}<Plus
              size={12}
            />创建“{choice}”{:else}{prefix}{choice}{/if}</button
        >{/each}
    </div>{/if}
</div>
