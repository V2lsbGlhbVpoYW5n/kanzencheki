<script lang="ts">
  import {
    sourceMessage,
    collectionTitle,
    message,
    tr,
    displayPerson,
  } from "$lib/i18n.svelte";
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import {
    ArrowLeft,
    Images,
    Paperclip,
    FileText,
    Plus,
    UserRound,
    Pencil,
    Trash2,
    Undo2,
    X,
    Video,
    Music,
    File,
    Check,
    CheckSquare,
  } from "@lucide/svelte";
  import {
    librarySession,
    loadLibrary,
    desktop,
    trashChekis,
  } from "./session.svelte";
  import {
    getPersonSpace,
    importPersonFiles,
    demoImport,
    trashPerson,
    deletePersonFile,
    deleteDocument,
    renamePersonFile,
  } from "./people.svelte";
  import {
    coverPhoto,
    shotTypes,
    formatBytes,
    fileLabel,
    type PersonSpace,
    type PersonFile,
    type PersonDocument,
  } from "./model";
  import { notify } from "./tasks.svelte";
  import PersonEditor from "./PersonEditor.svelte";
  import PersonFilePreview from "./PersonFilePreview.svelte";
  import PersonDocumentEditor from "./PersonDocumentEditor.svelte";
  import PhotoImage from "./Photo.svelte";
  import FileCover from "./FileCover.svelte";
  import TaskCenter from "./TaskCenter.svelte";
  let { personId }: { personId: string } = $props();
  let person = $derived(librarySession.people.find((p) => p.id === personId));
  let chekis = $derived(
    librarySession.photos
      .filter((c) => !c.deletedAt && c.peopleIds?.includes(personId))
      .sort((a, b) => b.date.localeCompare(a.date)),
  );
  let first = $derived(
    chekis
      .map((c) => c.date)
      .filter(Boolean)
      .sort()[0],
  );
  let days = $derived(new Set(chekis.map((c) => c.date).filter(Boolean)).size);
  let section = $state<"chekis" | "files" | "documents">(
      page.url.searchParams.get("section") === "documents"
        ? "documents"
        : page.url.searchParams.get("section") === "files"
          ? "files"
          : "chekis",
    ),
    editing = $state(false),
    loading = $state(true),
    busy = $state(false),
    error = $state("");
  let space = $state<PersonSpace>({ files: [], documents: [] }),
    preview = $state<PersonFile | null>(null),
    article = $state<PersonDocument | "new" | null>(null);
  let removing = $state<"person" | PersonFile | PersonDocument | null>(null);
  let confirm = $state(false);
  let input: HTMLInputElement;
  let selecting = $state(false),
    selected = $state<string[]>([]),
    batchConfirm = $state(false);
  let renaming = $state<PersonFile | null>(null),
    newName = $state("");
  let entries = $derived(
    section === "chekis"
      ? chekis
      : section === "files"
        ? space.files
        : space.documents,
  );
  function toggle(id: string) {
    if (busy) return;
    selected = selected.includes(id)
      ? selected.filter((x) => x !== id)
      : [...selected, id];
    batchConfirm = false;
  }
  function switchSection(next: typeof section) {
    section = next;
    selected = [];
    batchConfirm = false;
    selecting = false;
  }
  async function batchRemove() {
    if (busy || !selected.length) return;
    if (!batchConfirm) {
      batchConfirm = true;
      return;
    }
    busy = true;
    let done = 0;
    try {
      if (section === "chekis") {
        await trashChekis(selected);
        done = selected.length;
      } else if (section === "files") {
        for (const f of space.files.filter((f) => selected.includes(f.id))) {
          space = await deletePersonFile(f);
          done++;
        }
      } else {
        for (const d of space.documents.filter((d) =>
          selected.includes(d.id),
        )) {
          space = await deleteDocument(d);
          done++;
        }
      }
      selected = [];
      batchConfirm = false;
      notify(sourceMessage("已处理 {0} 项", [done]));
    } catch (e) {
      error = tr("已处理 {0} 项；{1}", [done, e]);
      space = await getPersonSpace(personId);
    } finally {
      busy = false;
    }
  }
  function showRename(node: HTMLDialogElement) {
    node.showModal();
  }
  async function rename() {
    if (!renaming || busy) return;
    busy = true;
    error = "";
    try {
      space = await renamePersonFile(renaming, newName);
      renaming = null;
      notify(sourceMessage("附件已改名"));
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  onMount(async () => {
    await loadLibrary(true);
    try {
      if (librarySession.people.some((p) => p.id === personId))
        space = await getPersonSpace(personId);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  });
  async function addFile() {
    if (!desktop) {
      input.click();
      return;
    }
    busy = true;
    try {
      space = await importPersonFiles(personId);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  async function remove() {
    if (!person || !removing || busy) return;
    if (!confirm) {
      confirm = true;
      return;
    }
    busy = true;
    try {
      if (removing === "person") {
        const purging = !!person.deletedAt;
        await trashPerson(person, false, purging);
        goto(purging ? "/people?trash" : "/people");
        notify(
          purging
            ? sourceMessage("人物资料已移入系统回收站；拍立得保留")
            : sourceMessage("人物已移入回收站"),
        );
      } else if ("mimeType" in removing) {
        space = await deletePersonFile(removing);
        notify(sourceMessage("附件已移入系统回收站"));
      } else {
        space = await deleteDocument(removing);
        notify(sourceMessage("文章已移入系统回收站"));
      }
      removing = null;
      confirm = false;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  async function restore() {
    if (!person) return;
    busy = true;
    try {
      await trashPerson(person, true);
      notify(sourceMessage("人物已恢复"));
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  function requestRemove(item: typeof removing) {
    removing = item;
    confirm = false;
    error = "";
  }
</script>

<svelte:head><title>{person?.name ?? tr("人物")} — Cheki</title></svelte:head>
<input
  class="hidden"
  type="file"
  multiple
  bind:this={input}
  onchange={async (e) => {
    const files = e.currentTarget.files;
    if (files) space = await demoImport(personId, files);
    e.currentTarget.value = "";
  }}
/>
<main
  class="min-h-screen bg-canvas px-8 pb-44 pt-28 text-base-content"
  aria-label={tr("人物档案")}
>
  {#if person}<div class="mx-auto max-w-5xl">
      <header class="relative mx-auto mb-12 max-w-3xl text-center">
        <div class="absolute right-0 top-0 flex items-center gap-1">
          {#if !person.deletedAt}<button
              class="btn btn-ghost btn-sm btn-circle"
              aria-label={tr("编辑人物资料")}
              onclick={() => (editing = true)}><Pencil size={16} /></button
            >{/if}<button
            class="btn btn-ghost btn-sm btn-circle text-danger-ink"
            aria-label={person.deletedAt ? tr("永久删除人物") : tr("删除人物")}
            disabled={busy}
            onclick={() => requestRemove("person")}><Trash2 size={16} /></button
          >
        </div>
        <div
          class="mx-auto mb-5 grid size-28 place-items-center rounded-full bg-tint/50 text-accent-ink/55"
        >
          <UserRound size={52} strokeWidth={1} />
        </div>
        <h1 class="text-3xl font-normal">{person.name}</h1>
        <div class="mt-3 flex justify-center gap-2">
          {#if person.description}<span
              class="badge border-0 bg-tint/50 text-xs text-ink/50"
              >{person.description}</span
            >{/if}{#if person.deletedAt}<span
              class="badge border-0 bg-[#b28b83]/15 text-xs text-danger-ink"
              >{tr("人物回收站")}</span
            >{/if}
        </div>
        {#if person.aliases.length}<p class="mt-3 text-xs text-ink/40">
            {person.aliases.join(" / ")}
          </p>{/if}
        {#if person.notes}<p
            class="mx-auto mt-5 max-w-xl whitespace-pre-wrap text-sm leading-7 text-ink/60"
          >
            {person.notes}
          </p>{/if}
        <div
          class="mx-auto mt-7 grid max-w-lg grid-cols-3 divide-x divide-ink/8 rounded-2xl bg-surface/30 py-4"
        >
          <div>
            <p class="text-base tabular-nums">{first || "—"}</p>
            <p class="mt-2 text-[11px] text-ink/40">{tr("第一张拍立得")}</p>
          </div>
          <div>
            <p class="text-base tabular-nums">
              {days !== null && days >= 0 ? days : "—"}<span
                class="ml-1 text-xs text-ink/40">{tr("天")}</span
              >
            </p>
            <p class="mt-2 text-[11px] text-ink/40">{tr("见面天数")}</p>
          </div>
          <div>
            <p class="text-base tabular-nums">
              {chekis.length}<span class="ml-1 text-xs text-ink/40"
                >{tr("张")}</span
              >
            </p>
            <p class="mt-2 text-[11px] text-ink/40">{tr("拍立得收藏")}</p>
          </div>
        </div>
        {#if chekis.length}<div
            class="mt-4 flex flex-wrap justify-center gap-x-4 gap-y-2 text-[11px] text-ink/40"
          >
            {#each shotTypes as t}{@const n = chekis.filter(
                (c) => c.shotType === t,
              ).length}{#if n}<span
                  >{tr(t)} <span class="ml-1 text-ink/65">{n}</span></span
                >{/if}{/each}
          </div>{/if}
      </header>
      <div class="mb-5 flex items-center gap-2 text-xs text-ink/40">
        <h2>
          {section === "chekis"
            ? tr("拍立得")
            : section === "files"
              ? tr("附件")
              : tr("文章")}
        </h2>
        <span>· {entries.length}</span>
      </div>
      {#if error}<p
          class="mb-6 rounded-xl bg-error/10 p-4 text-xs text-error"
          role="alert"
        >
          {message(error)}
        </p>{/if}
      <section
        aria-label={section === "chekis"
          ? tr("人物拍立得")
          : section === "files"
            ? tr("人物附件")
            : tr("人物文章")}
        class="divide-y divide-ink/8"
      >
        {#if section === "chekis"}
          {#each chekis as c}{@const a =
              c.assets.find((a) => a.id === c.coverAssetId) ?? c.assets[0]}
            <a
              href={`/?cheki=${c.id}&returnTo=${encodeURIComponent(`/people/${personId}`)}`}
              aria-label={tr("打开拍立得 {0} {1}", [
                c.date,
                collectionTitle(c),
              ])}
              onclick={(e) => {
                if (selecting) {
                  e.preventDefault();
                  toggle(c.id);
                }
              }}
              class={`flex min-h-20 items-center gap-4 rounded-lg px-3 py-2 transition hover:bg-surface/30 ${selected.includes(c.id) ? "bg-tint/45" : ""}`}
            >
              {#if selecting}<span
                  class="grid size-5 shrink-0 place-items-center rounded border border-ink/20"
                  aria-hidden="true"
                  >{#if selected.includes(c.id)}<Check size={14} />{/if}</span
                >{/if}
              <div class="grid h-16 w-14 shrink-0 place-items-center overflow-hidden">
                {#if a?.src}<PhotoImage
                    photo={coverPhoto(c, desktop)}
                  />{:else}<Images size={24} />{/if}
              </div>
              <div class="min-w-0 flex-1">
                <p class="truncate text-sm">{c.event || collectionTitle(c)}</p>
                <p class="mt-1 truncate text-xs text-ink/45">
                  {tr(c.shotType)}{#if c.tags.length}
                    · {c.tags.map((t) => `#${t}`).join(" ")}{/if}
                </p>
              </div>
              <span class="shrink-0 text-xs tabular-nums text-ink/50"
                >{c.date || tr("日期待补充")}</span
              ><span class="w-16 shrink-0 text-right text-[11px] text-ink/35"
                >{tr("{0} 份影像", [c.assets.length])}</span
              >
            </a>
          {:else}<p class="py-12 text-center text-sm text-ink/40">
              {tr("这里会汇集所有关联到 {0} 的拍立得。", [person.name])}
            </p>{/each}
        {:else if section === "files"}
          {#each space.files as f}<article
              class={`flex items-center gap-3 rounded-lg px-3 py-2 transition hover:bg-surface/30 ${selected.includes(f.id) ? "bg-tint/45" : ""}`}
            >
              <button
                class="flex min-w-0 flex-1 items-center gap-4 text-left"
                aria-label={tr("预览附件 {0}", [fileLabel(f)])}
                onclick={() => (selecting ? toggle(f.id) : (preview = f))}
              >
                {#if selecting}<span
                    class="grid size-5 shrink-0 place-items-center rounded border border-ink/20"
                    >{#if selected.includes(f.id)}<Check size={14} />{/if}</span
                  >{/if}
                <div class="size-12 shrink-0 overflow-hidden rounded-lg">
                  <FileCover file={f} />
                </div>
                <div class="min-w-0 flex-1">
                  <p class="truncate text-sm">{fileLabel(f)}</p>
                  <p class="mt-1 text-[11px] text-ink/40">
                    {formatBytes(f.byteSize)}{#if !f.available}{tr(
                        "· 文件缺失",
                      )}{/if}
                  </p>
                </div>
                <span class="shrink-0 text-xs tabular-nums text-ink/45"
                  >{f.createdAt.slice(0, 10)}</span
                >
              </button>
              {#if !selecting && !person.deletedAt}<button
                  class="btn btn-ghost btn-xs btn-circle"
                  aria-label={tr("改名附件 {0}", [fileLabel(f)])}
                  onclick={() => {
                    renaming = f;
                    newName = fileLabel(f);
                    error = "";
                  }}><Pencil size={13} /></button
                ><button
                  class="btn btn-ghost btn-xs btn-circle"
                  aria-label={tr("删除附件 {0}", [fileLabel(f)])}
                  onclick={() => requestRemove(f)}><Trash2 size={13} /></button
                >{/if}
            </article>{:else}<p class="py-12 text-center text-sm text-ink/40">
              {tr("照片、视频、音频，都可以留在这里。")}
            </p>{/each}
        {:else}
          {#each space.documents as d}<article
              class={`flex items-center gap-3 rounded-lg px-3 py-3 transition hover:bg-surface/30 ${selected.includes(d.id) ? "bg-tint/45" : ""}`}
            >
              <button
                class="flex min-w-0 flex-1 items-center gap-4 text-left"
                onclick={() => (selecting ? toggle(d.id) : (article = d))}
                >{#if selecting}<span
                    class="grid size-5 shrink-0 place-items-center rounded border border-ink/20"
                    >{#if selected.includes(d.id)}<Check size={14} />{/if}</span
                  >{/if}<span
                  class="grid size-10 shrink-0 place-items-center rounded-lg bg-tint/25 text-accent-ink"
                  ><FileText size={20} strokeWidth={1.5} /></span
                >
                <h2 class="min-w-0 flex-1 truncate text-sm">{d.title}</h2>
                <span class="shrink-0 text-xs tabular-nums text-ink/45"
                  >{d.updatedAt.slice(0, 10)}</span
                ></button
              >{#if !selecting && !person.deletedAt}<button
                  class="btn btn-ghost btn-xs btn-circle"
                  aria-label={tr("删除文章 {0}", [d.title])}
                  onclick={() => requestRemove(d)}><Trash2 size={13} /></button
                >{/if}
            </article>
          {:else}<p class="py-12 text-center text-sm text-ink/40">
              {tr("演出后的心情，想对他说的话，写成一篇篇文章。")}
            </p>{/each}
        {/if}
      </section>
      {#if loading}<p class="py-6 text-center text-xs text-ink/40">
          {tr("正在打开人物文件夹…")}
        </p>{/if}
    </div>
    {#if selecting}<div
        class="glass-panel fixed bottom-28 left-1/2 z-20 flex w-max max-w-[95vw] -translate-x-1/2 items-center gap-3 rounded-2xl px-5 py-3"
      >
        <button
          class="btn btn-ghost btn-xs"
          disabled={busy}
          onclick={() => {
            selected =
              selected.length === entries.length
                ? []
                : entries.map((e) => e.id);
            batchConfirm = false;
          }}>{tr("全选")}</button
        ><span class="whitespace-nowrap text-xs text-ink/45"
          >{tr("已选 {0} 项", [selected.length])}</span
        ><button
          class="btn btn-ghost btn-sm rounded-full text-xs text-danger-ink"
          disabled={busy || !selected.length}
          onclick={batchRemove}
          ><Trash2 size={14} />{batchConfirm
            ? tr("再次点击确认")
            : section === "chekis"
              ? tr("移入相册回收站")
              : tr("移入系统回收站")}</button
        >
      </div>{/if}
    <nav
      aria-label={tr("人物档案工具")}
      class="glass-light fixed bottom-7 left-1/2 z-20 flex w-max max-w-[95vw] -translate-x-1/2 items-center gap-2 rounded-full px-4 py-2.5"
    >
      <a
        href="/people"
        class="btn btn-ghost btn-sm btn-circle"
        aria-label={tr("返回人物列表")}><ArrowLeft size={17} /></a
      >
      {#each [{ id: "chekis", name: tr("拍立得"), icon: Images }, { id: "files", name: tr("附件"), icon: Paperclip }, { id: "documents", name: tr("文章"), icon: FileText }] as item}<button
          class={`btn btn-ghost btn-sm rounded-full text-xs font-normal ${section === item.id ? "bg-[#8c9d72]/20" : ""}`}
          aria-pressed={section === item.id}
          disabled={busy}
          onclick={() => switchSection(item.id as typeof section)}
          ><item.icon size={15} />{item.name}</button
        >{/each}
      <span class="h-5 border-l border-ink/10"></span>
      {#if person.deletedAt}<button
          class="btn btn-ghost btn-sm rounded-full text-xs"
          disabled={busy}
          onclick={restore}><Undo2 size={15} />{tr("恢复")}</button
        >{:else if section === "files"}<button
          class="btn glass-dark btn-sm rounded-full text-xs text-white"
          disabled={busy || librarySession.busy}
          onclick={addFile}><Plus size={15} />{tr("添加附件")}</button
        >{:else if section === "documents"}<button
          class="btn glass-dark btn-sm rounded-full text-xs text-white"
          disabled={busy}
          onclick={() => (article = "new")}
          ><Plus size={15} />{tr("写文章")}</button
        >{/if}
      {#if !person.deletedAt}<button
          class={`btn btn-ghost btn-sm btn-circle ${selecting ? "bg-[#8c9d72]/20" : ""}`}
          aria-label={tr("批量选择")}
          aria-pressed={selecting}
          disabled={busy}
          onclick={() => {
            selecting = !selecting;
            selected = [];
            batchConfirm = false;
          }}><CheckSquare size={16} /></button
        >{/if}
    </nav>
  {:else}<div class="py-28 text-center text-sm text-ink/40">
      {loading ? tr("正在打开人物…") : tr("人物不存在或已永久删除")}<a
        href="/people"
        class="mt-5 block underline">{tr("返回人物列表")}</a
      >
    </div>{/if}
</main>
{#if removing && person}<div
    class="glass-panel fixed bottom-28 left-1/2 z-30 w-96 -translate-x-1/2 rounded-2xl p-5"
    role="alertdialog"
    aria-label={tr("删除确认")}
  >
    <div class="flex justify-between">
      <p class="text-sm">
        {removing === "person"
          ? person.deletedAt
            ? tr("永久删除人物")
            : tr("删除人物")
          : tr("移入系统回收站")}
      </p>
      <button
        class="btn btn-ghost btn-xs btn-circle"
        aria-label={tr("取消删除")}
        disabled={busy}
        onclick={() => (removing = null)}><X size={14} /></button
      >
    </div>
    <p class="my-3 text-xs leading-6 text-ink/55">
      {removing === "person"
        ? person.deletedAt
          ? tr(
              "人物附件和文章会移入系统回收站，收藏仅解除关联，拍立得原件保留。失去全部人物的收藏会回到 Inbox。",
            )
          : tr(
              "人物进入回收站后仍保留文章、附件及收藏关系，可随时恢复。拍立得不会删除。",
            )
        : tr("文件会移入系统回收站，文章中的相关引用可能显示为已删除。")}
    </p>
    <button
      class="btn btn-sm rounded-full bg-[#b28b83]/20 text-danger-ink"
      disabled={busy}
      onclick={remove}
      >{busy
        ? tr("正在处理…")
        : confirm
          ? tr("再次点击确认删除")
          : tr("我已了解，继续")}</button
    >{#if error}<p class="mt-3 text-xs text-error">{message(error)}</p>{/if}
  </div>{/if}
{#if renaming}<dialog
    use:showRename
    oncancel={(e) => {
      e.preventDefault();
      if (!busy) renaming = null;
    }}
    class="modal bg-scrim/30 backdrop-blur-xl"
    aria-label={tr("附件改名")}
  >
    <form
      class="modal-box glass-panel max-w-md rounded-2xl"
      onsubmit={(e) => {
        e.preventDefault();
        rename();
      }}
    >
      <h2 class="mb-4 text-base">{tr("附件改名")}</h2>
      <input
        class="input w-full border-0 bg-surface/40"
        aria-label={tr("附件文件名")}
        bind:value={newName}
        disabled={busy}
      />
      <p class="mt-3 text-xs text-ink/40">
        {tr("保留扩展名。磁盘文件会同步改名，文章引用保持有效。")}
      </p>
      {#if error}<p class="mt-3 text-xs text-error">{message(error)}</p>{/if}
      <div class="modal-action">
        <button
          type="button"
          class="btn btn-ghost btn-sm rounded-full"
          disabled={busy}
          onclick={() => (renaming = null)}>{tr("取消")}</button
        ><button
          class="btn glass-dark btn-sm rounded-full text-white"
          disabled={busy || !newName.trim()}>{tr("保存文件名")}</button
        >
      </div>
    </form>
  </dialog>{/if}
{#if editing && person}<PersonEditor
    {person}
    onclose={() => (editing = false)}
    onsave={() => (editing = false)}
  />{/if}
{#if preview}<PersonFilePreview
    file={preview}
    onclose={() => (preview = null)}
  />{/if}
{#if article && person}<PersonDocumentEditor
    {personId}
    document={article === "new" ? undefined : article}
    {space}
    readonly={!!person.deletedAt}
    onclose={() => (article = null)}
    onsave={(s) => (space = s)}
  />{/if}
<TaskCenter />
