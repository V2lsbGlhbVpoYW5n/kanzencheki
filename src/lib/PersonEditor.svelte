<script lang="ts">
  import { message, tr } from "$lib/i18n.svelte";
  import { untrack } from "svelte";
  import { X } from "@lucide/svelte";
  import TokenInput from "./TokenInput.svelte";
  import { type Person, normalize } from "./model";
  import { librarySession } from "./session.svelte";
  import { savePerson } from "./people.svelte";
  let {
    person,
    initialName = "",
    duplicate = false,
    onclose,
    onsave,
  }: {
    person?: Person;
    initialName?: string;
    duplicate?: boolean;
    onclose: () => void;
    onsave: (p: Person) => void;
  } = $props();
  let dialog: HTMLDialogElement;
  let name = $state(untrack(() => person?.name ?? initialName)),
    description = $state(untrack(() => person?.description ?? "")),
    aliases = $state(untrack(() => [...(person?.aliases ?? [])])),
    notes = $state(untrack(() => person?.notes ?? ""));
  let allowDuplicate = $state(untrack(() => duplicate));
  let busy = $state(false),
    error = $state("");
  let aliasInput: TokenInput;
  let same = $derived(
    librarySession.people.filter(
      (p) => p.id !== person?.id && normalize(p.name) === normalize(name),
    ),
  );
  const initial = untrack(() =>
    JSON.stringify({ name, description, aliases, notes }),
  );
  $effect(() => dialog?.showModal());
  function close() {
    if (busy) return;
    if (
      JSON.stringify({ name, description, aliases, notes }) !== initial &&
      !window.confirm(tr("放弃尚未保存的人物资料？"))
    )
      return;
    onclose();
  }
  async function save() {
    aliasInput?.flush();
    busy = true;
    error = "";
    try {
      const p = await savePerson({
        id: person?.id,
        name,
        description,
        aliases,
        notes,
        allowDuplicate: allowDuplicate || !!person,
      });
      onsave(p);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  class="modal bg-scrim/20 backdrop-blur-xl"
  aria-label={person ? tr("编辑人物") : tr("创建人物")}
  oncancel={(e) => {
    e.preventDefault();
    e.stopPropagation();
    close();
  }}
>
  <div class="modal-box glass-panel max-w-lg rounded-3xl p-7 text-base-content">
    <header class="mb-5 flex items-center justify-between">
      <h2 class="text-lg">
        {person
          ? tr("人物资料")
          : duplicate
            ? tr("创建同名人物")
            : tr("认识一个新的人")}
      </h2>
      <button
        class="btn btn-ghost btn-circle btn-sm"
        aria-label={tr("关闭人物编辑")}
        onclick={close}><X size={18} /></button
      >
    </header>
    <form
      onsubmit={(e) => {
        e.preventDefault();
        save();
      }}
      class="space-y-4"
    >
      <label class="block text-xs text-ink/55"
        >{tr("主要名字")}<input
          class="input mt-2 w-full border-0 bg-surface/35"
          aria-label={tr("人物名字")}
          bind:value={name}
          maxlength="80"
          required
          disabled={busy}
        /></label
      >
      <label class="block text-xs text-ink/55"
        >{tr("区分描述 {0}", [
          same.length ? tr("· 重名时必填") : tr("· 可选，例如所属团体"),
        ])}<input
          class="input mt-2 w-full border-0 bg-surface/35 text-sm"
          aria-label={tr("人物区分描述")}
          bind:value={description}
          maxlength="60"
          required={same.length > 0}
          disabled={busy}
        /></label
      >
      {#if same.length}<p class="text-xs text-ink/50">
          {tr(
            "已有 {0} 位同名人物。请填写不同的简短描述，这会显示在人物选择候选中。",
            [same.length],
          )}
        </p>{/if}
      {#if same.length && !person && !allowDuplicate}<button
          type="button"
          class="btn btn-ghost btn-sm"
          onclick={() => (allowDuplicate = true)}
          >{tr("创建同名人物（需要区分说明）")}</button
        >{/if}
      <div>
        <p class="mb-2 text-xs text-ink/55">{tr("别名")}</p>
        <TokenInput
          bind:this={aliasInput}
          bind:values={aliases}
          suggestions={[]}
          label={tr("别名")}
          placeholder={tr("输入别名后按 Enter 添加")}
        />
      </div>
      <label class="block text-xs text-ink/55"
        >{tr("基本备注")}<textarea
          class="textarea mt-2 w-full border-0 bg-surface/35 text-sm"
          aria-label={tr("人物备注")}
          bind:value={notes}
          rows="3"
          disabled={busy}></textarea></label
      >
      {#if person}<p class="text-[11px] text-ink/40">
          {tr("改名后旧名会加入别名；不会批量修改拍立得原件文件名。")}
        </p>{/if}
      {#if error}<p class="text-xs text-error" role="alert">
          {message(error)}
        </p>{/if}
      <button
        class="btn glass-dark w-full rounded-full text-white"
        disabled={busy ||
          !name.trim() ||
          (same.length > 0 &&
            (!description.trim() || (!person && !allowDuplicate)))}
        >{busy ? tr("正在保存…") : tr("保存人物")}</button
      >
    </form>
  </div>
</dialog>
