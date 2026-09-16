<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { goto } from "$app/navigation";
  import { X, Save, Pencil, FileText } from "@lucide/svelte";
  import type { PersonDocument, PersonSpace, PersonFile } from "./model";
  import { title as chekiTitle, fileLabel } from "./model";
  import { librarySession } from "./session.svelte";
  import { readDocument, saveDocument } from "./people.svelte";
  import { notify } from "./tasks.svelte";
  import MarkdownEditor from "./MarkdownEditor.svelte";
  import ReferencePicker from "./ReferencePicker.svelte";
  import PersonFilePreview from "./PersonFilePreview.svelte";
  let {
    personId,
    document: doc,
    space,
    onclose,
    onsave,
    readonly = false,
  }: {
    personId: string;
    document?: PersonDocument;
    space: PersonSpace;
    onclose: () => void;
    onsave: (space: PersonSpace) => void;
    readonly?: boolean;
  } = $props();
  let dialog: HTMLDialogElement;
  let editor = $state<MarkdownEditor>();
  let title = $state(untrack(() => doc?.title ?? "")),
    body = $state(""),
    initial = $state(""),
    revision = $state<string | null>(null),
    ready = $state(false),
    editing = $state(untrack(() => !doc)),
    busy = $state(false),
    error = $state(""),
    picking = $state(false);
  let preview = $state<PersonFile | null>(null);
  let dirty = $derived(ready && JSON.stringify({ title, body }) !== initial);
  let references = $derived([
    ...new Set(
      [...body.matchAll(/\]\((cheki|attachment):([a-zA-Z0-9-]+)\)/g)].map(
        (m) => `${m[1]}:${m[2]}`,
      ),
    ),
  ]);
  onMount(async () => {
    dialog.showModal();
    try {
      if (doc) {
        const d = await readDocument(doc);
        body = d.body;
        revision = d.revision;
      }
      initial = JSON.stringify({ title, body });
      ready = true;
    } catch (e) {
      error = String(e);
    }
  });
  function close() {
    if (busy) return;
    if (dirty && !window.confirm("放弃尚未保存的文章？")) return;
    onclose();
  }
  async function save() {
    if (!title.trim() || busy) return;
    busy = true;
    error = "";
    try {
      const next = await saveDocument({
        id: doc?.id,
        personId,
        title,
        body,
        revision,
      });
      onsave(next);
      notify("文章已保存为 Markdown 文件");
      onclose();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  function openReference(kind: string, id: string) {
    if (kind === "cheki") {
      if (!librarySession.photos.some((c) => c.id === id && !c.deletedAt)) {
        notify("引用的收藏已删除或在回收站中", "error");
        return;
      }
      if (dirty && !window.confirm("离开文章并放弃未保存修改？")) return;
      onclose();
      goto(
        `/?cheki=${encodeURIComponent(id)}&returnTo=${encodeURIComponent(`/people/${personId}?section=documents`)}`,
      );
    } else {
      const file = space.files.find((f) => f.id === id);
      if (file) preview = file;
      else notify("引用的附件已删除", "error");
    }
  }
</script>

<dialog
  bind:this={dialog}
  class="modal bg-scrim/25 backdrop-blur-2xl"
  aria-label="人物文章"
  oncancel={(e) => {
    e.preventDefault();
    e.stopPropagation();
    close();
  }}
>
  <div
    class="modal-box glass-panel flex max-h-[90vh] w-[min(850px,92vw)] max-w-none flex-col rounded-3xl p-7 text-base-content"
  >
    <header class="mb-5 flex shrink-0 items-center gap-3">
      <FileText size={18} /><span class="flex-1 text-xs text-ink/45"
        >{doc ? "人物文章" : "写一篇小作文"}</span
      >{#if !readonly && doc && !editing}<button
          class="btn btn-ghost btn-sm rounded-full"
          onclick={() => (editing = true)}><Pencil size={14} />编辑</button
        >{/if}<button
        class="btn btn-ghost btn-sm btn-circle"
        aria-label="关闭文章"
        onclick={close}><X size={18} /></button
      >
    </header>
    <div class="min-h-0 flex-1 overflow-auto px-1">
      {#if editing && !readonly}<input
          class="input mb-5 w-full border-0 bg-surface/20 text-xl"
          aria-label="文章标题"
          placeholder="写个标题…"
          bind:value={title}
          maxlength="150"
          disabled={busy}
        />{:else}<h1 class="mb-6 text-2xl">{title}</h1>{/if}
      {#if ready}<MarkdownEditor
          bind:this={editor}
          bind:value={body}
          editable={editing && !readonly && !busy}
          onreference={openReference}
          onpickreference={() => (picking = true)}
        />{:else if !error}<span class="loading loading-spinner"></span>{/if}
      {#if references.length}<div
          class="mt-6 grid grid-cols-2 gap-3 border-t border-ink/8 pt-4"
        >
          {#each references as ref}{@const [kind, id] =
              ref.split(":")}{@const c = librarySession.photos.find(
              (c) => c.id === id,
            )}{@const f = space.files.find((f) => f.id === id)}<button
              class="flex items-center gap-3 rounded-xl bg-surface/25 p-3 text-left text-xs"
              onclick={() => openReference(kind, id)}
              >{#if kind === "cheki" && c && !c.deletedAt}{@const a =
                  c.assets.find((a) => a.id === c.coverAssetId) ??
                  c.assets[0]}{#if a?.src}<img
                    src={a.src}
                    alt=""
                    class="h-16 w-12 object-cover"
                    loading="lazy"
                  />{/if}<span
                  >{chekiTitle(c)}<small class="mt-1 block text-ink/40"
                    >{c.date}</small
                  ></span
                >{:else if kind === "attachment" && f}<FileText
                  size={18}
                /><span class="truncate">{fileLabel(f)}</span>{:else}<span
                  class="text-ink/40">引用对象已删除</span
                >{/if}</button
            >{/each}
        </div>{/if}
      {#if error}<p class="mt-4 text-xs text-error" role="alert">
          {error}
        </p>{/if}
    </div>
    {#if editing && !readonly}<footer
        class="mt-5 flex shrink-0 items-center justify-between border-t border-ink/5 pt-4"
      >
        <span class="text-[11px] text-ink/40"
          >{dirty ? "尚未保存" : "Markdown · 本地文件"}</span
        ><button
          class="btn glass-dark btn-sm rounded-full text-white"
          disabled={!ready || busy || !title.trim()}
          onclick={save}
          ><Save size={14} />{busy ? "保存中…" : "保存文章"}</button
        >
      </footer>{/if}
  </div>
  {#if picking}<ReferencePicker
      files={space.files}
      onclose={() => (picking = false)}
      onselect={(uri, label) => {
        editor?.insertReference(label, uri);
        picking = false;
      }}
    />{/if}
  {#if preview}<PersonFilePreview
      file={preview}
      onclose={() => (preview = null)}
    />{/if}
</dialog>
