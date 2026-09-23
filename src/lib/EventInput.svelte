<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Search } from "@lucide/svelte";
  import { message, tr } from "$lib/i18n.svelte";
  import { onDestroy } from "svelte";
  import { normalize } from "./model";
  import { desktop } from "./session.svelte";

  type Suggestion = { title: string; venue: string };
  type Result = { items: Suggestion[]; total: number; truncated: boolean };
  let { date, value = $bindable("") }: { date: string; value: string } = $props();
  let input: HTMLInputElement;
  let menu: HTMLDivElement;
  const listId = $props.id();
  let focused = $state(false);
  let loading = $state(false);
  let error = $state("");
  let result = $state<Result>({ items: [], total: 0, truncated: false });
  let active = $state(0);
  let left = $state(0), top = $state(0), width = $state(320), maxHeight = $state(260);
  let request = 0;
  let candidates = $derived(
    result.items.filter(item => normalize(item.title).includes(normalize(value))).slice(0, 12),
  );

  function position() {
    const bounds = input.getBoundingClientRect();
    width = Math.min(360, window.innerWidth - 24);
    left = Math.max(12, Math.min(bounds.left, window.innerWidth - width - 12));
    const roomBelow = window.innerHeight - bounds.bottom - 12;
    if (roomBelow >= 220 || roomBelow >= bounds.top) {
      top = bounds.bottom + 6;
      maxHeight = Math.max(90, Math.min(280, roomBelow - 6));
    } else {
      maxHeight = Math.max(90, Math.min(280, bounds.top - 20));
      top = Math.max(12, bounds.top - maxHeight - 6);
    }
  }
  function open() {
    focused = true;
    active = 0;
    position();
    if (!menu.matches(":popover-open")) menu.showPopover();
  }
  function close() {
    focused = false;
    request++;
    loading = false;
    if (menu?.matches(":popover-open")) menu.hidePopover();
  }
  onDestroy(close);
  async function load(dateToFetch: string) {
    const current = ++request;
    result = { items: [], total: 0, truncated: false };
    error = "";
    if (!desktop) return;
    loading = true;
    try {
      const received = await invoke<Result>("event_suggestions", { date: dateToFetch });
      if (current === request) result = received;
    } catch (e) {
      if (current === request) error = String(e);
    } finally {
      if (current === request) loading = false;
    }
  }
  $effect(() => {
    if (!focused) return;
    if (date) void load(date);
    else {
      request++;
      result = { items: [], total: 0, truncated: false };
      error = "";
      loading = false;
    }
  });
  function select(title: string) {
    value = title;
    close();
    input.focus({ preventScroll: true });
  }
  function key(e: KeyboardEvent) {
    if (e.isComposing || e.ctrlKey || e.metaKey) return;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (!focused) open();
      active = (active + (e.key === "ArrowDown" ? 1 : -1) + Math.max(candidates.length, 1)) % Math.max(candidates.length, 1);
    } else if (e.key === "Enter" && focused && candidates[active]) {
      e.preventDefault();
      select(candidates[active].title);
    } else if (e.key === "Escape" && focused) {
      e.stopPropagation();
      close();
    }
  }
</script>

<label class="block text-[10px] text-ink/50">
  {tr("活动")}
  <div class="relative mt-1.5">
    <input bind:this={input} bind:value
      aria-label={tr("活动")}
      role="combobox" aria-autocomplete="list" aria-expanded={focused}
      aria-controls={listId}
      aria-activedescendant={focused && candidates[active] ? `${listId}-${active}` : undefined}
      class="input input-sm w-full border-transparent bg-surface/25 pr-8 text-xs shadow-[inset_0_1px_3px_#28301e12]"
      placeholder={tr("活动名称（可选）")}
      onfocus={open}
      oninput={() => { active = 0; if (!focused) open(); }}
      onkeydown={key}
      onblur={close}
    />
    {#if loading}<span class="loading loading-spinner loading-xs pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-ink/45"></span>
    {:else}<Search size={13} class="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-ink/35" />{/if}
  </div>
</label>
<div bind:this={menu} id={listId} popover="manual" role="listbox"
  aria-label={tr("当天活动候选")}
  class="glass-panel right-auto bottom-auto m-0 overflow-auto rounded-2xl border-0 p-2 text-base-content shadow-xl"
  style:position="fixed" style:left={`${left}px`} style:top={`${top}px`}
  style:width={`${width}px`} style:max-height={`${maxHeight}px`}
>
  <p class="px-2 pb-1.5 text-[10px] text-ink/45">
    {date ? tr("{0} · 推活日记", [date]) : tr("先填写收藏日期，再检索当天活动")}
  </p>
  {#if !date}{:else if !desktop}<p class="px-2 py-3 text-xs text-ink/55">{tr("桌面版可检索活动；仍可手动输入")}</p>
  {:else if loading}<p class="px-2 py-3 text-xs text-ink/55">{tr("正在检索当天活动…")}</p>
  {:else if error}<p class="px-2 py-3 text-xs text-ink/55">{tr("活动检索暂不可用，可继续手动输入")}: {message(error)}</p>
  {:else if !candidates.length}<p class="px-2 py-3 text-xs text-ink/55">{value ? tr("没有匹配的活动，可继续手动输入") : tr("当天暂无活动，可继续手动输入")}</p>
  {:else}
    {#each candidates as item, i}<button type="button" role="option"
      id={`${listId}-${i}`} aria-selected={active === i}
      class={`block w-full rounded-xl px-3 py-2 text-left hover:bg-ink/5 ${active === i ? "bg-ink/5" : ""}`}
      onpointerdown={(e) => e.preventDefault()}
      onclick={() => select(item.title)}
    ><span class="block text-xs leading-5">{item.title}</span>
      {#if item.venue}<span class="block truncate text-[10px] text-ink/45">{item.venue}</span>{/if}
    </button>{/each}
  {/if}
  {#if result.truncated}<p class="px-2 py-2 text-[10px] text-ink/45">{tr("当天活动较多，目前显示前 {0} 项", [result.items.length])}</p>{/if}
</div>
