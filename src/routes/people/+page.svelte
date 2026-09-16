<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import {
    Plus,
    Search,
    Trash2,
    UserRound,
    Settings2,
    CheckSquare,
    Check,
    Undo2,
  } from "@lucide/svelte";
  import { librarySession, loadLibrary } from "$lib/session.svelte";
  import { normalize, personLabel } from "$lib/model";
  import { trashPerson } from "$lib/people.svelte";
  import { notify } from "$lib/tasks.svelte";
  import PersonEditor from "$lib/PersonEditor.svelte";
  import TaskCenter from "$lib/TaskCenter.svelte";
  import Settings from "$lib/Settings.svelte";
  let query = $state(""),
    trash = $state(page.url.searchParams.has("trash")),
    creating = $state(false),
    duplicate = $state(false),
    settings = $state(false);
  let people = $derived(
    librarySession.people.filter(
      (p) =>
        !!p.deletedAt === trash &&
        [p.name, p.description, ...p.aliases].some((n) =>
          normalize(n).includes(normalize(query)),
        ),
    ),
  );
  let selecting = $state(false),
    selected = $state<string[]>([]),
    busy = $state(false),
    confirmed = $state(false);
  function toggle(id: string) {
    selected = selected.includes(id)
      ? selected.filter((x) => x !== id)
      : [...selected, id];
    confirmed = false;
  }
  async function batch(restore = false) {
    if (busy || !selected.length) return;
    if (!restore && !confirmed) {
      confirmed = true;
      return;
    }
    busy = true;
    let done = 0;
    try {
      for (const p of people.filter((p) => selected.includes(p.id))) {
        await trashPerson(p, restore, trash && !restore);
        done++;
      }
      selected = [];
      confirmed = false;
      notify(
        restore ? `已恢复 ${done} 位人物` : `已删除 ${done} 位人物，拍立得保留`,
      );
    } catch (e) {
      notify(`已处理 ${done} 位人物；${e}`, "error");
    } finally {
      busy = false;
    }
  }
  $effect(() => {
    query;
    selected = [];
    confirmed = false;
  });
  onMount(() => {
    loadLibrary(true);
  });
</script>

<svelte:head><title>Cheki — 人物</title></svelte:head>
<main
  class="min-h-screen bg-canvas px-8 pb-40 pt-28 text-base-content"
  aria-label="人物总览"
>
  <header class="mx-auto mb-8 max-w-5xl">
    <h1 class="text-xs font-normal text-ink/40">
      {trash ? "人物回收站" : "人物"} · {people.length}
    </h1>
  </header>
  <section
    class="mx-auto grid max-w-5xl grid-cols-2 gap-6 md:grid-cols-3 lg:grid-cols-4"
    aria-label="人物列表"
  >
    {#each people as p}<a
        href={`/people/${p.id}`}
        onclick={(e) => {
          if (selecting) {
            e.preventDefault();
            toggle(p.id);
          }
        }}
        class={`group relative flex min-h-80 flex-col items-center rounded-2xl bg-surface/40 px-5 py-8 text-center shadow-sm transition hover:-translate-y-1 hover:bg-surface/65 ${selected.includes(p.id) ? "ring-2 ring-[#8c9d72]" : ""}`}
        aria-label={`打开人物 ${personLabel(p)}`}
      >
        {#if selecting}<span
            class="absolute right-4 top-4 grid size-5 place-items-center rounded-full bg-surface/65 text-accent-ink"
            >{#if selected.includes(p.id)}<Check size={14} />{/if}</span
          >{/if}
        <div
          class="mb-7 grid size-32 shrink-0 place-items-center rounded-full bg-tint/45 text-accent-ink/50"
        >
          <UserRound size={58} strokeWidth={1} />
        </div>
        <h2 class="max-w-full truncate text-lg">{p.name}</h2>
        <div class="mt-2 min-h-7">
          {#if p.description}<span
              class="badge badge-sm border-0 bg-tint/50 px-3 text-xs text-ink/50"
              >{p.description}</span
            >{/if}
        </div>
        <p class="mt-auto pt-5 text-xs text-ink/40">
          {librarySession.photos.filter(
            (c) => !c.deletedAt && c.peopleIds?.includes(p.id),
          ).length} 张拍立得
        </p>
      </a>{:else}<p
        class="col-span-full py-24 text-center text-sm text-ink/40"
      >
        {query
          ? "没有找到这个名字"
          : trash
            ? "人物回收站是空的"
            : "从收藏中添加人物，或在这里创建人物"}
      </p>{/each}
  </section>
  {#if selecting}<div
      class="glass-panel fixed bottom-28 left-1/2 z-20 flex w-max max-w-[95vw] -translate-x-1/2 items-center gap-3 rounded-2xl px-5 py-3"
    >
      <button
        class="btn btn-ghost btn-xs"
        disabled={busy}
        onclick={() => {
          selected =
            selected.length === people.length ? [] : people.map((p) => p.id);
          confirmed = false;
        }}>全选</button
      ><span class="whitespace-nowrap text-xs text-ink/45"
        >已选 {selected.length} 位</span
      >{#if trash}<button
          class="btn btn-ghost btn-sm rounded-full text-xs"
          disabled={busy || !selected.length}
          onclick={() => batch(true)}><Undo2 size={14} />恢复</button
        >{/if}<button
        class="btn btn-ghost btn-sm rounded-full text-xs text-danger-ink"
        disabled={busy || !selected.length}
        onclick={() => batch()}
        ><Trash2 size={14} />{confirmed
          ? "再次点击确认"
          : trash
            ? "永久删除（资料进入系统回收站）"
            : "移入人物回收站"}</button
      >
    </div>{/if}
  {#if librarySession.error}<p
      role="alert"
      class="mx-auto mt-6 max-w-5xl text-sm text-error"
    >
      {librarySession.error}
    </p>{/if}
  <nav
    aria-label="人物工具"
    class="glass-light fixed bottom-7 left-1/2 z-20 flex w-max max-w-[95vw] -translate-x-1/2 items-center gap-3 rounded-full px-4 py-2.5"
  >
    <label class="flex items-center gap-2"
      ><Search size={16} /><input
        class="w-44 bg-transparent text-xs outline-none"
        aria-label="搜索人物或别名"
        placeholder="名字、别名…"
        bind:value={query}
      /></label
    >

    <button
      class={`btn btn-ghost btn-sm btn-circle ${selecting ? "bg-[#8c9d72]/20" : ""}`}
      aria-label="批量选择人物"
      aria-pressed={selecting}
      disabled={busy}
      onclick={() => {
        selecting = !selecting;
        selected = [];
        confirmed = false;
      }}><CheckSquare size={16} /></button
    >
    <button
      class="btn glass-dark btn-sm rounded-full text-xs font-normal text-white"
      onclick={() => {
        duplicate =
          !!query.trim() &&
          librarySession.people.some(
            (p) => normalize(p.name) === normalize(query),
          );
        creating = true;
      }}
      ><Plus size={15} />{query.trim() &&
      librarySession.people.some((p) => normalize(p.name) === normalize(query))
        ? "创建同名人物"
        : "新建人物"}</button
    >
    <button
      class={`btn btn-ghost btn-sm btn-circle ${trash ? "bg-[#b28b83]/20 text-danger-ink" : ""}`}
      aria-label="人物回收站"
      aria-pressed={trash}
      disabled={busy}
      onclick={() => {
        trash = !trash;
        selected = [];
        confirmed = false;
      }}><Trash2 size={16} /></button
    >
    <button
      class="btn btn-ghost btn-sm btn-circle"
      aria-label="图库设置"
      onclick={() => (settings = true)}><Settings2 size={16} /></button
    >
  </nav>
</main>
{#if creating}<PersonEditor
    initialName={query.trim()}
    {duplicate}
    onclose={() => (creating = false)}
    onsave={() => {
      creating = false;
      trash = false;
      query = "";
    }}
  />{/if}
{#if settings}<Settings onclose={() => (settings = false)} />{/if}
<TaskCenter />
