<script lang="ts">
  import { tr, sourceMessage, message } from "$lib/i18n.svelte";
  import { X, Check } from "@lucide/svelte";
  import DatePicker from "./DatePicker.svelte";
  import PersonPicker from "./PersonPicker.svelte";
  import SelectMenu from "./SelectMenu.svelte";
  import { librarySession, desktop, catalogCommand } from "./session.svelte";
  import { notify } from "./tasks.svelte";
  import { shotTypes, type Cheki, type ShotType } from "./model";
  let { items, onclose, onapplied }: { items: Cheki[]; onclose: () => void; onapplied: () => void } = $props();
  let dialog: HTMLDialogElement;
  let fields = $state({ date: false, people: false, event: false, shotType: false });
  let date = $state("");
  let peopleIds = $state<string[]>([]);
  let group = $state("");
  let event = $state("");
  let shotType = $state<ShotType>("solo");
  let working = $state(false), error = $state("");
  let picker = $state<PersonPicker>();
  let hasGroups = $derived(items.some(c => c.shotType === "团切"));
  let resultIsGroup = $derived(fields.shotType && shotType === "团切");
  $effect(() => dialog?.showModal());
  async function apply() {
    if (working || !Object.values(fields).some(Boolean)) return;
    picker?.flush();
    if (fields.people && picker?.hasPendingInput()) { error = tr("请先选择或创建输入的人物"); return; }
    if (fields.people && (resultIsGroup || (hasGroups && !fields.shotType))) {
      error = tr("团切不关联人物；请先将类别改为非团切，或取消人物修改"); return;
    }
    const patch = {
      ...(fields.date ? { date } : {}),
      ...(fields.people ? { peopleIds } : {}),
      ...(fields.event ? { event } : {}),
      ...(fields.shotType ? { shotType, ...(resultIsGroup ? { group } : {}) } : {}),
    };
    working = true;
    error = "";
    try {
      if (desktop) await catalogCommand("cheki_batch_update", { ids: items.map(c => c.id), patch });
      else for (const c of items) Object.assign(c, {
        ...(fields.date ? { date } : {}),
        ...(fields.people ? { peopleIds: [...peopleIds], people: peopleIds.map(id => librarySession.people.find(p => p.id === id)?.name ?? "") } : {}),
        ...(fields.event ? { event } : {}),
        ...(fields.shotType ? { shotType, ...(resultIsGroup ? { group, people: [], peopleIds: [] } : {}) } : {}),
      });
      notify(sourceMessage("已修改 {0} 张收藏", [items.length]));
      onapplied();
    } catch (e) { error = message(e); }
    finally { working = false; }
  }
</script>
<dialog bind:this={dialog} class="modal bg-scrim/20 backdrop-blur-xl" aria-label={tr("批量修改收藏")}
  onclose={onclose} oncancel={(e) => { e.stopPropagation(); if (working) e.preventDefault(); }}>
  <div class="modal-box glass-panel w-[min(560px,92vw)] max-w-none rounded-3xl p-6 text-base-content">
    <header class="mb-5 flex items-center justify-between">
      <div><h2 class="text-lg">{tr("批量修改收藏")}</h2><p class="mt-1 text-xs text-ink/45">{tr("已选 {0} 张；只修改勾选的字段", [items.length])}</p></div>
      <button class="btn btn-ghost btn-sm btn-circle" aria-label={tr("关闭")}
        disabled={working} onclick={() => dialog.close()}><X size={17}/></button>
    </header>
    <div class="space-y-4">
      <section class="rounded-2xl bg-surface/25 p-4">
        <label class="mb-3 flex items-center gap-2 text-sm"><input type="checkbox" class="checkbox checkbox-sm" bind:checked={fields.date}/>{tr("日期")}</label>
        {#if fields.date}<DatePicker label={tr("收藏日期")} bind:value={date}/>{/if}
      </section>
      <section class="rounded-2xl bg-surface/25 p-4">
        <label class="mb-3 flex items-center gap-2 text-sm"><input type="checkbox" class="checkbox checkbox-sm" bind:checked={fields.shotType}/>{tr("拍摄类型")}</label>
        {#if fields.shotType}<SelectMenu label={tr("拍摄类型")} value={shotType}
          options={shotTypes.map(value => ({ value, label: tr(value) }))} onchange={v => shotType = v as ShotType}/>
          {#if resultIsGroup}<input class="input input-sm mt-3 w-full border-0 bg-surface/30" aria-label={tr("团体")}
            placeholder={tr("团体名称，不关联人物")} bind:value={group}/>{/if}{/if}
      </section>
      <section class="rounded-2xl bg-surface/25 p-4">
        <label class="mb-3 flex items-center gap-2 text-sm"><input type="checkbox" class="checkbox checkbox-sm" bind:checked={fields.people}/>{tr("人物")}</label>
        {#if fields.people}<PersonPicker bind:this={picker} bind:ids={peopleIds}/>
          <p class="mt-2 text-[11px] text-ink/45">{tr("将所选收藏的人物统一替换为这里的人物；留空会清除人物")}</p>{/if}
      </section>
      <section class="rounded-2xl bg-surface/25 p-4">
        <label class="mb-3 flex items-center gap-2 text-sm"><input type="checkbox" class="checkbox checkbox-sm" bind:checked={fields.event}/>{tr("活动")}</label>
        {#if fields.event}<input class="input input-sm w-full border-0 bg-surface/30" aria-label={tr("活动")}
          placeholder={tr("留空会清除活动")} bind:value={event}/>{/if}
      </section>
    </div>
    <p class="mt-4 text-[11px] text-ink/45">{tr("修改日期、人物或类别时，在线原件的文件名也会更新；原件离线时操作会失败。")}</p>
    {#if error}<p class="mt-3 text-xs text-error" role="alert">{message(error)}</p>{/if}
    <footer class="mt-5 flex justify-end gap-2">
      <button class="btn btn-ghost btn-sm rounded-full" disabled={working} onclick={() => dialog.close()}>{tr("取消")}</button>
      <button class="btn glass-dark btn-sm rounded-full text-white" disabled={working || !Object.values(fields).some(Boolean)} onclick={apply}>
        {#if working}<span class="loading loading-spinner loading-xs"></span>{:else}<Check size={14}/>{/if}{tr("修改 {0} 张收藏", [items.length])}</button>
    </footer>
  </div>
</dialog>
