<script lang="ts">
  import { X, Plus } from "@lucide/svelte";
  import { librarySession } from "./session.svelte";
  import { personLabel, normalize, type Person } from "./model";
  import PersonEditor from "./PersonEditor.svelte";
  let {
    ids = $bindable<string[]>([]),
    onchange,
  }: { ids: string[]; onchange?: () => void } = $props();
  let text = $state(""),
    focused = $state(false),
    active = $state(0),
    creating = $state(false),
    duplicate = $state(false);
  const listId = $props.id();
  let matches = $derived(
    librarySession.people
      .filter(
        (p) =>
          !p.deletedAt &&
          !ids.includes(p.id) &&
          [p.name, p.description, ...p.aliases].some((n) =>
            normalize(n).includes(normalize(text)),
          ),
      )
      .slice(0, 6),
  );
  let same = $derived(
    librarySession.people.some((p) => normalize(p.name) === normalize(text)),
  );
  function add(p: Person) {
    ids = [...new Set([...ids, p.id])];
    text = "";
    focused = false;
    active = 0;
    onchange?.();
  }
  export function flush() {
    if (!text.trim()) return true;
    const exact = librarySession.people.filter(
      (p) => !p.deletedAt && normalize(p.name) === normalize(text),
    );
    if (exact.length === 1) {
      add(exact[0]);
      return true;
    }
    return false;
  }
  function create() {
    if (!text.trim()) return;
    duplicate = same;
    creating = true;
    focused = false;
  }
  function key(e: KeyboardEvent) {
    if (e.isComposing) return;
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      focused = false;
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      focused = true;
      active =
        (active + (e.key === "ArrowDown" ? 1 : -1) + matches.length + 1) %
        (matches.length + 1);
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (matches[active]) add(matches[active]);
      else create();
    } else if (e.key === "Backspace" && !text) {
      ids = ids.slice(0, -1);
      onchange?.();
    }
  }
</script>

<div class="relative">
  <div
    class="flex flex-wrap items-center gap-1.5 rounded-xl bg-surface/25 p-2 shadow-sm"
  >
    {#each ids as id}<span
        class="flex max-w-full items-center gap-1 rounded-md bg-tint/65 px-2 py-1 text-[11px]"
        ><span class="truncate"
          >{librarySession.people.find((p) => p.id === id)
            ? personLabel(librarySession.people.find((p) => p.id === id)!)
            : "已删除人物"}</span
        ><button
          type="button"
          aria-label={`移除人物 ${librarySession.people.find((p) => p.id === id)?.name ?? id}`}
          onclick={() => {
            ids = ids.filter((x) => x !== id);
            onchange?.();
          }}><X size={11} /></button
        ></span
      >{/each}
    <input
      class="min-w-16 flex-1 bg-transparent p-1 text-xs outline-none"
      role="combobox"
      aria-label="人物"
      aria-expanded={focused}
      aria-controls={listId}
      aria-autocomplete="list"
      bind:value={text}
      placeholder="搜索人物、别名，或创建"
      onfocus={() => (focused = true)}
      onblur={() => (focused = false)}
      oninput={() => {
        active = 0;
        focused = true;
      }}
      onkeydown={key}
    />
  </div>
  {#if focused}<div
      id={listId}
      role="listbox"
      aria-label="人物候选"
      class="glass-panel absolute left-0 right-0 top-full z-50 mt-1 max-h-56 overflow-auto rounded-xl p-1.5"
    >
      {#each matches as p, i}<button
          type="button"
          role="option"
          aria-selected={active === i}
          class={`block w-full rounded-lg px-3 py-2 text-left text-xs ${active === i ? "bg-black/7" : ""}`}
          onpointerdown={(e) => e.preventDefault()}
          onclick={() => add(p)}
          >{personLabel(p)}{#if p.aliases.length}<span
              class="mt-1 block text-[10px] text-ink/40"
              >{p.aliases.join(" / ")}</span
            >{/if}</button
        >{/each}
      {#if text.trim()}<button
          type="button"
          role="option"
          aria-selected={active === matches.length}
          class={`flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-xs ${active === matches.length ? "bg-black/7" : ""}`}
          onpointerdown={(e) => e.preventDefault()}
          onclick={create}
          ><Plus size={12} />{same
            ? "创建同名人物"
            : "创建人物"}“{text.trim()}”</button
        >{/if}
      {#if !matches.length && !text.trim()}<p class="p-3 text-xs text-ink/40">
          输入名字即可创建人物
        </p>{/if}
    </div>{/if}
</div>
{#if creating}<PersonEditor
    initialName={text.trim()}
    {duplicate}
    onclose={() => (creating = false)}
    onsave={(p) => {
      add(p);
      creating = false;
    }}
  />{/if}
