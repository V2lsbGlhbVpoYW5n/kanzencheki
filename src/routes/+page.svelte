<script lang="ts">
  import { goto } from "$app/navigation";
  import TaskCenter from "$lib/TaskCenter.svelte";
  import Settings from "$lib/Settings.svelte";
  import ImportSheet from "$lib/ImportSheet.svelte";
  import AssetPanel from "$lib/AssetPanel.svelte";
  import SelectMenu from "$lib/SelectMenu.svelte";
  import DatePicker from "$lib/DatePicker.svelte";
  import { notify } from "$lib/tasks.svelte";
  import { trashChekis, catalogCommand } from "$lib/session.svelte";
  import { Trash2, CheckSquare, Settings2, Undo2 } from "@lucide/svelte";
  import { onMount, tick } from "svelte";
  import {
    Images,
    Heart,
    Inbox,
    Search,
    Plus,
    X,
    ArrowDownWideNarrow,
    Info,
    ArrowLeft,
    Camera,
    Check,
    Link,
    RotateCw,
  } from "@lucide/svelte";
  import { appearance } from "$lib/appearance.svelte";
  import ImageViewer from "$lib/ImageViewer.svelte";
  import PhotoImage from "$lib/Photo.svelte";
  import TokenInput from "$lib/TokenInput.svelte";
  import PersonPicker from "$lib/PersonPicker.svelte";
  import { page } from "$app/state";
  import SearchInput from "$lib/SearchInput.svelte";
  import {
    librarySession,
    desktop,
    loadLibrary,
    saveCheki,
    setFavorite,
    importDesktop,
  } from "$lib/session.svelte";
  import {
    coverPhoto,
    type Cheki,
    type Metadata,
    type Asset,
    incomplete,
    metadata,
    title,
    unique,
    shotTypes,
    formatBytes,
  } from "$lib/model";
  import { matchesParsed, parseQuery } from "$lib/search";
  import { orderChekis, sortOptions, nearestDate } from "$lib/gallery-order";
  let photos = $derived(librarySession.photos);
  let mode = $state<"all" | "favorites" | "inbox" | "trash">("all");
  let inboxIds = $state<string[]>([]);
  let query = $state("");
  let parsedQuery = $derived(parseQuery(query));
  let typeFilter = $state("");
  let size = $state(210);
  let labels = $state(true);
  let sort = $state("date");
  let descending = $state(true);
  let filtersOpen = $state(false);
  let selectedId = $state<string | null>(null);
  let imageIndex = $state(0);
  let original = $state(false);
  let showInfo = $state(true);
  let infoTab = $state<"metadata" | "assets">("metadata");
  let viewer = $state<HTMLDialogElement>();
  let draft = $state<Metadata | null>(null);
  let initial = $state("");
  let saving = $state(false);
  let error = $state("");
  let notice = $state("");
  let selecting = $state(false);
  let checked = $state<string[]>([]);
  let confirmDelete = $state(false);
  let lastChecked = $state<string | null>(null);
  let settingsOpen = $state(false);
  let importOpen = $state(false);
  let targetCheki = $state<string | null>(null);
  let mergeId = $state("");
  let mergeConfirm = $state(false);

  $effect(() => {
    if (notice) {
      notify(notice);
      notice = "";
    }
  });
  function toggle(c: Cheki, e?: MouseEvent) {
    confirmDelete = false;
    if (e?.shiftKey && lastChecked) {
      const a = visible.findIndex((p) => p.id === lastChecked),
        b = visible.findIndex((p) => p.id === c.id);
      if (a >= 0)
        checked = unique([
          ...checked,
          ...visible.slice(Math.min(a, b), Math.max(a, b) + 1).map((p) => p.id),
        ]);
    } else
      checked = checked.includes(c.id)
        ? checked.filter((id) => id !== c.id)
        : [...checked, c.id];
    lastChecked = c.id;
  }
  async function remove(ids = checked) {
    if (saving || librarySession.busy) return;
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    await trashChekis(ids);
    checked = [];
    confirmDelete = false;
    selectedId = null;
    draft = null;
  }
  async function restore(ids = checked) {
    if (saving || librarySession.busy) return;
    await trashChekis(ids, true);
    checked = [];
  }
  let merging = $state(false);
  let purgeConfirm = $state(false);
  $effect(() => {
    checked.join();
    mergeConfirm = false;
    purgeConfirm = false;
  });
  async function merge() {
    if (checked.length < 2 || saving) return;
    if (!mergeId || !checked.includes(mergeId)) mergeId = checked[0];
    if (!mergeConfirm) {
      mergeConfirm = true;
      return;
    }
    saving = true;
    try {
      const sources = checked.filter((id) => id !== mergeId);
      if (desktop)
        await catalogCommand("cheki_merge", { target: mergeId, sources });
      else {
        const target = photos.find((c) => c.id === mergeId)!;
        for (const c of photos.filter((c) => sources.includes(c.id))) {
          target.assets = [
            ...new Map(
              [...target.assets, ...c.assets].map((a) => [a.id, a]),
            ).values(),
          ];
          c.deletedAt = new Date().toISOString();
        }
      }
      checked = [];
      merging = false;
      mergeConfirm = false;
      notify("已归并影像；其余收藏资料保留在回收站");
    } catch {
    } finally {
      saving = false;
    }
  }
  async function purge() {
    if (saving || !checked.length) return;
    if (!purgeConfirm) {
      purgeConfirm = true;
      return;
    }
    saving = true;
    try {
      if (desktop) await catalogCommand("cheki_purge", { ids: checked });
      else
        librarySession.photos = photos.filter((c) => !checked.includes(c.id));
      checked = [];
      purgeConfirm = false;
      notify("已从图库删除；不再使用的原件已移入系统回收站");
    } catch {
    } finally {
      saving = false;
    }
  }
  function prepareImport(cheki: string | null = null) {
    targetCheki = cheki;
    importOpen = true;
  }
  async function beginImport(options: {
    reference: boolean;
    grouping?: string;
  }) {
    if (desktop) {
      try {
        await importDesktop(targetCheki, options);
        if (mode === "inbox")
          inboxIds = unique([
            ...inboxIds,
            ...photos
              .filter((c) => !c.deletedAt && incomplete(c))
              .map((c) => c.id),
          ]);
      } catch {}
    } else {
      importTarget = targetCheki;
      (targetCheki ? imageInput : input)?.click();
    }
  }

  let peopleInput = $state<PersonPicker>();
  let tagsInput = $state<TokenInput>();
  let input: HTMLInputElement;
  let imageInput = $state<HTMLInputElement>();
  let importTarget: string | null = null;
  let selected = $derived(photos.find((c) => c.id === selectedId));

  let fullView = $state(false);
  let currentAsset = $derived(selected?.assets[imageIndex]);
  let visible = $derived(
    orderChekis(
      photos.filter(
        (c) =>
          (mode === "trash" ? !!c.deletedAt : !c.deletedAt) &&
          (mode === "favorites"
            ? c.favorite
            : mode === "inbox"
              ? inboxIds.includes(c.id)
              : true) &&
          (!typeFilter || c.shotType === typeFilter) &&
          matchesParsed(c, parsedQuery, librarySession.people),
      ),
      sort,
      descending,
      photos,
    ),
  );
  let allTags = $derived(unique(photos.flatMap((c) => c.tags)).sort());
  let allPeople = $derived(
    unique(
      photos
        .filter((c) => !c.deletedAt && c.shotType !== "团切")
        .flatMap((c) => c.people),
    ).sort(),
  );
  let allEvents = $derived(
    unique(photos.filter((c) => !c.deletedAt).map((c) => c.event)).sort(),
  );

  let dirty = $derived(!!draft && JSON.stringify(draft) !== initial);
  $effect(() => {
    if (selectedId && viewer && !viewer.open) viewer.showModal();
  });
  let openedReference = $state("");
  $effect(() => {
    const id = page.url.searchParams.get("cheki");
    if (id && librarySession.loaded && openedReference !== id) {
      const c = photos.find((c) => c.id === id);
      if (c) {
        openedReference = id;
        open(c);
      }
    }
  });
  onMount(() => {
    void loadLibrary();
    const timer = setInterval(() => {
      if (desktop && !librarySession.busy && !selectedId)
        void loadLibrary(true);
    }, 30000);
    return () => clearInterval(timer);
  });
  function changeMode(next: typeof mode) {
    checked = [];
    confirmDelete = false;
    lastChecked = null;
    mode = mode === next ? "all" : next;
    if (mode === "inbox")
      inboxIds = photos
        .filter((c) => !c.deletedAt && incomplete(c))
        .map((c) => c.id);
    else inboxIds = [];
  }
  async function jumpToDate(date: string) {
    if (!date || parsedQuery.error) return;
    const target = nearestDate(visible, date);
    if (!target) {
      notify("当前结果中没有填写日期的收藏", "error");
      return;
    }
    // Preserve the result set and sort order: date navigation is not a filter.
    filtersOpen = false;
    await tick();
    const element = document.getElementById(`cheki-${target.id}`);
    element?.scrollIntoView({
      behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches
        ? "instant"
        : "smooth",
      block: "start",
    });
  }
  function open(c: Cheki) {
    selectedId = c.id;
    draft = metadata(c);
    draft.peopleIds ??= [];
    initial = JSON.stringify(draft);
    imageIndex = Math.max(
      0,
      c.assets.findIndex((a) => a.id === c.coverAssetId),
    );
    original = false;
    error = "";
    filtersOpen = false;
    confirmDelete = false;
    mergeId = "";
    mergeConfirm = false;
  }
  function close() {
    peopleInput?.flush();
    tagsInput?.flush();
    if (
      draft &&
      JSON.stringify(draft) !== initial &&
      !window.confirm("放弃尚未保存的收藏信息？")
    )
      return;
    viewer?.close();
    selectedId = null;
    draft = null;
    error = "";
    const origin = page.url.searchParams.get("returnTo");
    if (
      origin &&
      /^\/people\/[a-zA-Z0-9-]+(?:\?section=(?:chekis|files|documents))?$/.test(
        origin,
      )
    )
      void goto(origin, { replaceState: true });
  }
  async function save() {
    if (!selected || !draft || saving || librarySession.busy) return;
    if (peopleInput && !peopleInput.flush()) {
      error = "请先选择或创建输入的人物";
      return;
    }
    tagsInput?.flush();
    saving = true;
    error = "";
    try {
      await saveCheki(selected.id, metadata(draft));
      draft = metadata(librarySession.photos.find((c) => c.id === selectedId)!);
      initial = JSON.stringify(draft);
      notice = desktop ? "收藏信息已保存" : "示例修改已保存到本次会话";
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
  async function favorite(c: Cheki) {
    saving = true;
    error = "";
    try {
      await setFavorite(c);
      if (draft) {
        const now = photos.find((p) => p.id === c.id)!;
        draft.favorite = now.favorite;
        const baseline = JSON.parse(initial);
        baseline.favorite = now.favorite;
        initial = JSON.stringify(baseline);
      }
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
  function key(e: KeyboardEvent) {
    if (
      e.isComposing ||
      e.defaultPrevented ||
      (e.target as HTMLElement)?.closest("[popover]") ||
      document.querySelectorAll("dialog[open]").length > 1
    )
      return;
    if (
      !selected &&
      selecting &&
      (e.ctrlKey || e.metaKey) &&
      e.key === "a" &&
      !(e.target as HTMLElement)?.matches("input,textarea")
    ) {
      e.preventDefault();
      checked = visible.map((c) => c.id);
      confirmDelete = false;
      return;
    }

    if ((e.ctrlKey || e.metaKey) && e.key === "Enter" && selected) {
      e.preventDefault();
      void save();
      return;
    }
    if (e.key === "Escape" && !selected) filtersOpen = false;
    if ((e.target as HTMLElement)?.matches("input,textarea,select")) return;
    if (selected && (e.key === "ArrowLeft" || e.key === "ArrowRight")) {
      e.preventDefault();
      imageIndex =
        (imageIndex +
          (e.key === "ArrowLeft" ? -1 : 1) +
          selected.assets.length) %
        selected.assets.length;
      original = false;
    }
  }
  async function browserImport(e: Event) {
    const files = [...((e.target as HTMLInputElement).files ?? [])];
    const target = photos.find((c) => c.id === importTarget);
    for (const file of files) {
      if (!["image/jpeg", "image/png", "image/webp"].includes(file.type)) {
        notice = "浏览器示例支持 JPEG/PNG/WebP；TIFF 请在桌面端导入。";
        continue;
      }
      const src = URL.createObjectURL(file);
      const dimensions = await new Promise<{
        width: number;
        height: number;
      } | null>((resolve) => {
        const img = new window.Image();
        img.onload = () =>
          resolve({ width: img.naturalWidth, height: img.naturalHeight });
        img.onerror = () => resolve(null);
        img.src = src;
      });
      if (!dimensions) {
        URL.revokeObjectURL(src);
        continue;
      }
      const id = crypto.randomUUID();
      const asset: Asset = {
        id,
        src,
        originalPath: src,
        filename: file.name,
        originalFilename: file.name,
        ...dimensions,
        byteSize: file.size,
        previewError: null,
        renditions: [],
      };
      if (target) target.assets.push(asset);
      else {
        photos.push({
          id,
          coverAssetId: id,
          date: "",
          group: "",
          people: [],
          event: "",
          tags: [],
          shotType: "其他",
          notes: "",
          favorite: false,
          assets: [asset],
        });
        if (mode === "inbox") inboxIds.push(id);
      }
    }
    (e.target as HTMLInputElement).value = "";
    notice = "浏览器临时预览；正式持久化请启动桌面程序。";
  }
</script>

<svelte:head><title>Cheki — 相册</title></svelte:head>
<svelte:window onkeydown={key} />
<input
  class="hidden"
  bind:this={input}
  type="file"
  multiple
  accept="image/jpeg,image/png,image/webp"
  onchange={browserImport}
  aria-label="选择照片"
/>
<main
  aria-label="全览相册"
  class="h-screen overflow-y-auto bg-canvas text-base-content"
>
  <div class="absolute left-8 top-8 text-xs text-ink/50 sm:left-12">
    <span
      >{mode === "inbox"
        ? "Inbox"
        : mode === "trash"
          ? "回收站"
          : mode === "favorites"
            ? "喜欢"
            : "全部收藏"}</span
    ><span class="ml-3 text-ink/30">{visible.length}</span>
  </div>
  <section class="px-8 pb-36 pt-28 sm:px-12" aria-label="收藏网格">
    {#if librarySession.error || (error && !selected)}<div
        role="alert"
        class="glass-panel mb-5 rounded-xl p-4 text-sm"
      >
        {librarySession.error || error}<button
          class="btn btn-ghost btn-sm"
          onclick={() => {
            librarySession.error = "";
            void loadLibrary();
          }}>重试</button
        >
      </div>{/if}
    <div
      class="grid gap-x-8 gap-y-10"
      style:grid-template-columns={`repeat(auto-fill,minmax(min(${size}px,100%),1fr))`}
    >
      {#each visible as c (c.id)}
        <div id={`cheki-${c.id}`} class="min-w-0 scroll-mt-28 rounded-sm">
          <button
            class="relative block w-full rounded-sm text-left outline-offset-8 transition-transform duration-200 hover:-translate-y-1 focus-visible:outline-2 focus-visible:outline-[#798468] motion-reduce:transform-none"
            aria-label={`${selecting ? "选择" : "查看"} ${title(c)}`}
            aria-pressed={selecting ? checked.includes(c.id) : undefined}
            onclick={(e) =>
              selecting || e.shiftKey
                ? ((selecting = true), toggle(c, e))
                : open(c)}
          >
            <div class="aspect-square overflow-hidden rounded-sm bg-surface/5">
              <PhotoImage photo={coverPhoto(c, desktop)} fit={appearance.fit} />
            </div>
            {#if selecting}<span
                class={`absolute left-3 top-3 flex h-6 w-6 items-center justify-center rounded-full shadow ${checked.includes(c.id) ? "bg-[#697957] text-white" : "glass-panel"}`}
                ><Check size={14} /></span
              >{/if}
            {#if c.favorite}<span
                class="glass-panel absolute bottom-3 right-3 rounded-full p-1.5 text-danger-ink"
                ><Heart size={12} fill="currentColor" /></span
              >{/if}
            {#if c.assets.length > 1}<span
                class="glass-panel absolute bottom-3 left-3 flex items-center gap-1 rounded-full px-2 py-1 text-[10px]"
                ><Images size={11} />{c.assets.length}</span
              >{/if}
            {#if mode === "inbox" && !incomplete(c)}<span
                class="absolute inset-0 flex flex-col items-center justify-center gap-2 bg-tint/60 backdrop-blur-[2px]"
                ><span
                  class="glass-panel flex items-center gap-2 rounded-full px-4 py-2 text-xs"
                  ><Check size={14} />信息已补全</span
                ><span class="text-[10px] text-ink/55"
                  >离开 Inbox 后归档 · 点击继续编辑</span
                ></span
              >{/if}
          </button>{#if labels}<div
              class="mt-3 flex items-center justify-between gap-2"
            >
              <span class="truncate text-[11px] text-ink/65">{title(c)}</span
              ><span class="shrink-0 text-[10px] text-ink/40"
                >{c.date
                  ? c.date.slice(5).replace("-", " / ")
                  : "日期待补充"}</span
              >
            </div>{/if}
        </div>
      {:else}<div
          class="col-span-full flex h-80 flex-col items-center justify-center gap-4 text-ink/45"
        >
          <Images size={30} strokeWidth={1} />
          <p class="text-sm">
            {!librarySession.loaded
              ? "正在打开本地图库…"
              : photos.length
                ? "没有匹配的收藏"
                : "导入第一张拍立得"}
          </p>
          {#if photos.length}<button
              class="btn btn-ghost btn-sm text-xs"
              onclick={() => {
                mode = "all";
                inboxIds = [];
                query = "";
                typeFilter = "";
              }}>查看全部收藏</button
            >{:else}<button
              class="btn glass-dark rounded-full text-xs text-white"
              disabled={librarySession.busy || !librarySession.loaded}
              onclick={() => prepareImport()}><Plus size={14} />导入照片</button
            >{/if}
        </div>{/each}
    </div>
    <p class="mt-12 text-center text-[10px] text-ink/40">
      {desktop
        ? `本地图库 · ${librarySession.root}`
        : "浏览器示例 · 不写入磁盘；请启动 Tauri 桌面版使用本地图库"}
    </p>
  </section>
  {#if filtersOpen}<section
      aria-label="搜索与筛选"
      class="glass-panel fixed bottom-[106px] left-1/2 z-30 w-[min(560px,calc(100vw-32px))] -translate-x-1/2 rounded-2xl p-5"
    >
      <div class="flex items-center gap-3">
        <SearchInput
          bind:value={query}
          tags={allTags}
          people={librarySession.people}
          error={parsedQuery.error}
        /><button
          class="btn btn-ghost btn-sm btn-circle"
          aria-label="收起搜索与筛选"
          onclick={() => (filtersOpen = false)}><X size={16} /></button
        >
      </div>
      <p class="mt-3 text-[10px] text-ink/45">
        @人物 / #标签 精确匹配 · 空格默认为 AND · 优先级 NOT → AND → OR
        <br />例：(@小明 OR @小蓝) AND NOT #重复 · 名称含空格用引号
      </p>
      <div
        class="mt-4 flex flex-wrap items-center gap-3 border-t border-ink/8 pt-3"
      >
        <div class="flex items-center gap-1">
          <div class="w-40">
            <SelectMenu
              label="相册排序"
              bind:value={sort}
              options={sortOptions}
            />
          </div>
          <button
            class={`btn btn-ghost btn-sm btn-circle ${descending ? "bg-ink/5" : ""}`}
            aria-label={descending
              ? "当前倒序，切换为正序"
              : "当前正序，切换为倒序"}
            title={sort === "date"
              ? descending
                ? "新到旧"
                : "旧到新"
              : sort === "meetings"
                ? descending
                  ? "见面天数：多到少（团切最后）"
                  : "见面天数：少到多（团切最后）"
                : descending
                  ? "名称倒序"
                  : "名称正序"}
            aria-pressed={descending}
            onclick={() => (descending = !descending)}
          >
            <ArrowDownWideNarrow
              size={16}
              class={descending ? "" : "rotate-180"}
            />
          </button>
        </div>
        <div class="w-36">
          <SelectMenu
            label="拍摄类型筛选"
            bind:value={typeFilter}
            options={[{ value: "", label: "所有拍摄类型" }, ...shotTypes]}
          />
        </div>
        <div class="w-36">
          <DatePicker label="跳转到日期" onselect={jumpToDate} />
        </div>
        <button
          class="btn btn-ghost btn-xs ml-auto"
          onclick={() => {
            typeFilter = "";
            query = "";
          }}>重置</button
        >
      </div>
    </section>{/if}
  {#if selecting}<div
      class="glass-panel fixed bottom-24 left-1/2 z-20 flex w-max max-w-[95vw] -translate-x-1/2 items-center gap-3 rounded-full px-4 py-2 text-xs"
    >
      <span>已选 {checked.length} 张</span><button
        class="btn btn-ghost btn-xs"
        onclick={() => {
          checked = visible.map((c) => c.id);
          confirmDelete = false;
        }}>全选当前结果</button
      ><button
        class="btn btn-ghost btn-xs"
        onclick={() => {
          checked = [];
          confirmDelete = false;
        }}>取消选择</button
      >{#if mode === "trash"}<button
          class="btn btn-ghost btn-sm"
          disabled={!checked.length}
          onclick={() => restore()}><Undo2 size={14} />恢复</button
        ><button
          class="btn btn-ghost btn-sm text-error"
          disabled={!checked.length || saving}
          onclick={purge}
          >{purgeConfirm ? "再次确认：移至系统回收站" : "永久删除…"}</button
        >{:else}<button
          class="btn btn-ghost btn-sm"
          disabled={checked.length < 2 || saving}
          onclick={() => {
            merging = !merging;
            mergeId = checked[0];
            mergeConfirm = false;
          }}>归并…</button
        ><button
          class={`btn btn-sm rounded-full ${confirmDelete ? "bg-[#bd8273]/25" : "btn-ghost"}`}
          disabled={!checked.length}
          onclick={() => remove()}
          ><Trash2 size={14} />{confirmDelete
            ? "再次点击确认删除"
            : "移入回收站"}</button
        >{/if}
    </div>{/if}
  {#if selecting && merging && mode !== "trash" && checked.length >= 2}
    <section
      class="glass-panel fixed bottom-40 left-1/2 z-30 w-[min(560px,90vw)] -translate-x-1/2 space-y-3 rounded-2xl p-5"
      aria-label="批量归并"
    >
      <div class="flex items-center justify-between">
        <h2 class="text-sm">将 {checked.length} 张收藏归并</h2>
        <button class="btn btn-ghost btn-xs" onclick={() => (merging = false)}
          >取消</button
        >
      </div>
      <p class="text-xs text-ink/50">
        选择保留资料的收藏。其他收藏的影像归入此处，其原资料仍可从回收站恢复。
      </p>
      <div
        class="max-h-[40vh] space-y-1 overflow-auto"
        role="group"
        aria-label="保留哪张收藏的资料"
      >
        {#each photos.filter((c) => checked.includes(c.id)) as c}{@const a =
            c.assets.find((a) => a.id === c.coverAssetId) ?? c.assets[0]}
          <button
            class={`flex w-full items-center gap-3 rounded-xl p-2 text-left ${mergeId === c.id ? "bg-tint/50 ring-1 ring-[#8c9d72]" : "hover:bg-surface/30"}`}
            aria-pressed={mergeId === c.id}
            aria-label={`保留 ${c.date} ${title(c)} ${a?.filename ?? c.id}`}
            disabled={saving}
            onclick={() => {
              mergeId = c.id;
              mergeConfirm = false;
            }}
          >
            <div class="grid h-16 w-14 shrink-0 place-items-center">
              {#if a?.src}<PhotoImage
                  photo={coverPhoto(c, desktop)}
                />{:else}<Images size={20} />{/if}
            </div>
            <div class="min-w-0 flex-1">
              <p class="text-xs">{c.date || "未定日期"} · {title(c)}</p>
              <p class="mt-1 truncate text-[11px] text-ink/45">
                {a?.filename ?? "无影像"} · {c.id.slice(0, 8)}
              </p>
              {#if c.event}<p class="mt-1 truncate text-[11px] text-ink/40">
                  {c.event}
                </p>{/if}
            </div>
            {#if mergeId === c.id}<Check size={16} />{/if}
          </button>
        {/each}
      </div>
      <button
        class="btn glass-dark btn-sm w-full rounded-full text-white"
        disabled={saving}
        onclick={merge}>{mergeConfirm ? "确认归并" : "归并所选影像"}</button
      >
    </section>
  {/if}
  <nav
    aria-label="相册工具"
    class="glass-light fixed bottom-7 left-1/2 z-20 flex -translate-x-1/2 items-center gap-2 rounded-full px-4 py-2.5 sm:gap-3"
  >
    <button
      class={`btn btn-ghost btn-sm btn-circle ${filtersOpen ? "bg-ink/5" : ""}`}
      aria-label="搜索与筛选"
      aria-expanded={filtersOpen}
      onclick={() => (filtersOpen = !filtersOpen)}><Search size={17} /></button
    >
    <button
      class={`btn btn-ghost btn-sm btn-circle ${mode === "favorites" ? "bg-[#b28b83]/20 text-danger-ink" : ""}`}
      aria-label="筛选喜欢"
      aria-pressed={mode === "favorites"}
      onclick={() => changeMode("favorites")}
      ><Heart
        size={17}
        fill={mode === "favorites" ? "currentColor" : "none"}
      /></button
    >
    <button
      class={`btn btn-ghost btn-sm btn-circle ${mode === "inbox" ? "bg-[#8c9d72]/20" : ""}`}
      aria-label="筛选 Inbox"
      aria-pressed={mode === "inbox"}
      onclick={() => changeMode("inbox")}><Inbox size={17} /></button
    ><span class="h-5 border-l border-ink/10"></span>
    <input
      class="range range-xs w-20 text-accent-ink"
      aria-label="照片大小"
      type="range"
      min="150"
      max="290"
      step="10"
      bind:value={size}
    /><button
      class="btn btn-ghost btn-sm btn-circle"
      aria-label="切换照片标题"
      aria-pressed={labels}
      onclick={() => (labels = !labels)}><Info size={16} /></button
    >
    <span class="h-5 border-l border-ink/10"></span><button
      class="btn btn-sm glass-dark gap-2 rounded-full px-4 text-xs font-normal text-white"
      disabled={librarySession.busy || !librarySession.loaded}
      onclick={() => prepareImport()}><Plus size={15} />导入</button
    >
    <button
      class={`btn btn-ghost btn-sm btn-circle ${selecting ? "bg-[#8c9d72]/20 text-accent-ink" : ""}`}
      aria-label="选择收藏"
      aria-pressed={selecting}
      onclick={() => {
        selecting = !selecting;
        checked = [];
        confirmDelete = false;
      }}><CheckSquare size={16} /></button
    >
    <button
      class={`btn btn-ghost btn-sm btn-circle ${mode === "trash" ? "bg-[#b28b83]/20 text-danger-ink" : ""}`}
      aria-label="回收站"
      aria-pressed={mode === "trash"}
      onclick={() => changeMode("trash")}><Trash2 size={16} /></button
    >
    <button
      class="btn btn-ghost btn-sm btn-circle"
      aria-label="图库设置"
      onclick={() => (settingsOpen = true)}><Settings2 size={16} /></button
    >
  </nav>
</main>
{#if selected && draft && currentAsset}
  <dialog
    bind:this={viewer}
    oncancel={(e) => {
      e.preventDefault();
      if (!saving && !librarySession.busy) close();
    }}
    onclose={() => {
      selectedId = null;
      draft = null;
    }}
    class="fixed inset-0 m-0 h-screen max-h-none w-screen max-w-none overflow-hidden border-0 bg-transparent p-0 text-base-content backdrop:bg-transparent"
    aria-label="收藏聚焦预览"
  >
    <input
      class="hidden"
      bind:this={imageInput}
      type="file"
      multiple
      accept="image/jpeg,image/png,image/webp"
      onchange={browserImport}
      aria-label="选择影像文件"
    />
    <button
      class="absolute inset-0 h-full w-full cursor-default bg-tint/25 backdrop-blur-[28px] backdrop-saturate-75"
      aria-label="点击背景返回相册"
      tabindex="-1"
      disabled={saving || librarySession.busy}
      onclick={close}
    ></button>
    <div class="pointer-events-none relative flex h-full flex-col px-6 py-5">
      <header class="flex shrink-0 items-center justify-between">
        <span class="text-xs text-ink/40">{title(selected)}</span>
        <div
          class="glass-light pointer-events-auto flex items-center gap-2 rounded-full p-1"
        >
          {#if selected.deletedAt}<button
              class="btn btn-ghost btn-sm rounded-full text-xs"
              onclick={() => restore([selected!.id])}
              ><Undo2 size={14} />恢复</button
            >{:else}<button
              class="btn btn-ghost btn-sm rounded-full text-xs"
              aria-label="删除当前收藏"
              onclick={() => remove([selected!.id])}
              ><Trash2 size={14} />{confirmDelete ? "确认删除" : ""}</button
            >{/if}
          <button
            class="btn btn-ghost btn-sm btn-circle text-danger-ink"
            aria-label={selected.favorite ? "取消喜欢" : "设为喜欢"}
            disabled={saving}
            onclick={() => favorite(selected!)}
            ><Heart
              size={17}
              fill={selected.favorite ? "currentColor" : "none"}
            /></button
          ><button
            class="btn btn-ghost btn-sm btn-circle"
            aria-label="切换收藏信息"
            aria-pressed={showInfo}
            onclick={() => (showInfo = !showInfo)}><Info size={17} /></button
          ><button
            class="btn btn-ghost btn-sm btn-circle"
            aria-label="关闭预览"
            disabled={saving || librarySession.busy}
            onclick={close}><X size={18} /></button
          >
        </div>
      </header>
      <div class="flex min-h-0 flex-1 items-center justify-center gap-8 py-5">
        <div
          class="pointer-events-auto flex h-full min-w-0 flex-1 flex-col items-center justify-center"
        >
          <div
            class="min-h-0 w-full max-w-[760px] cursor-zoom-in overflow-hidden bg-transparent"
            role="button"
            tabindex="0"
            aria-label="全窗口查看图像"
            onclick={() => (fullView = true)}
            onkeydown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                e.stopPropagation();
                fullView = true;
              }
            }}
            style:height="min(100%,660px)"
          >
            <PhotoImage
              photo={{
                src: original
                  ? currentAsset.baseSrc || currentAsset.src
                  : currentAsset.src,
                title: title(selected),
                crop:
                  !desktop && currentAsset.crop
                    ? {
                        x: currentAsset.crop.x * 100,
                        y: currentAsset.crop.y * 100,
                        w: currentAsset.crop.w * 100,
                        h: currentAsset.crop.h * 100,
                      }
                    : currentAsset.crop === null
                      ? undefined
                      : imageIndex === 0
                        ? selected.crop
                        : undefined,
              }}
              full={original}
            />
          </div>
        </div>
        {#if showInfo}<aside
            class="glass-panel pointer-events-auto flex max-h-full w-80 shrink-0 flex-col overflow-clip rounded-2xl"
          >
            <div
              class="flex shrink-0 items-center justify-between px-5 pb-3 pt-5"
            >
              <div role="tablist" class="tabs tabs-box bg-black/4 p-0.5">
                <button
                  role="tab"
                  aria-selected={infoTab === "metadata"}
                  class={`tab h-7 px-3 text-xs ${infoTab === "metadata" ? "tab-active" : ""}`}
                  onclick={() => (infoTab = "metadata")}>收藏信息</button
                ><button
                  role="tab"
                  aria-selected={infoTab === "assets"}
                  class={`tab h-7 px-3 text-xs ${infoTab === "assets" ? "tab-active" : ""}`}
                  onclick={() => (infoTab = "assets")}>影像</button
                >
              </div>
              <span class="text-[10px] text-ink/40"
                >{dirty ? "未保存" : desktop ? "已保存到本地" : "示例"}</span
              >
            </div>
            <fieldset
              disabled={saving || librarySession.busy}
              class="min-h-0 space-y-4 overflow-y-auto px-5 pb-5"
            >
              {#if infoTab === "metadata"}
                <div>
                  <p class="mb-1.5 text-[10px] text-ink/50">日期</p>
                  <DatePicker label="收藏日期" bind:value={draft.date} />
                </div>
                <div>
                  <p class="mb-1.5 text-[10px] text-ink/50">
                    {draft.shotType === "团切" ? "团体" : "人物"}
                  </p>
                  {#if draft.shotType === "团切"}<input
                      class="input input-sm w-full border-0 bg-surface/25 text-xs"
                      aria-label="团体"
                      placeholder="团体名称，不关联人物"
                      bind:value={draft.group}
                    />
                  {:else}<PersonPicker
                      bind:this={peopleInput}
                      bind:ids={draft.peopleIds!}
                      onchange={() => {
                        if (draft)
                          draft.people = (draft.peopleIds ?? []).map(
                            (id) =>
                              librarySession.people.find((p) => p.id === id)
                                ?.name ?? "",
                          );
                      }}
                    />{/if}
                </div>
                <label class="block text-[10px] text-ink/50"
                  >活动<input
                    aria-label="活动"
                    class="input input-sm mt-1.5 w-full border-transparent bg-surface/25 text-xs shadow-[inset_0_1px_3px_#28301e12]"
                    placeholder="活动名称（可选）"
                    bind:value={draft.event}
                  /></label
                >
                <div>
                  <p class="mb-1.5 text-[10px] text-ink/50">标签</p>
                  <TokenInput
                    bind:this={tagsInput}
                    bind:values={draft.tags}
                    suggestions={allTags}
                    label="标签"
                    prefix="#"
                    placeholder="添加标签…"
                  />
                </div>
                <div>
                  <p class="mb-1.5 text-[10px] text-ink/50">拍摄类型</p>
                  <SelectMenu
                    label="拍摄类型"
                    value={draft.shotType}
                    options={[...shotTypes]}
                    onchange={(v) => {
                      if (draft) draft.shotType = v as Metadata["shotType"];
                    }}
                  />
                </div>
                <label class="block text-[10px] text-ink/50"
                  >备注<textarea
                    aria-label="备注"
                    class="textarea mt-1.5 w-full border-transparent bg-surface/25 text-xs"
                    rows="2"
                    placeholder="写点什么…"
                    bind:value={draft.notes}></textarea></label
                >
              {:else}
                <AssetPanel
                  asset={currentAsset}
                  cheki={selected}
                  ondelete={() => {
                    imageIndex = 0;
                    if (!selected?.assets.length) {
                      selectedId = null;
                      draft = null;
                    }
                  }}
                />
              {/if}
              <div
                class="space-y-1.5 border-t border-ink/8 pt-4 text-[10px] leading-4 text-ink/40"
              >
                <p class="break-all">{currentAsset.filename}</p>
                {#if currentAsset.originalFilename !== currentAsset.filename}<p
                    class="break-all"
                  >
                    原文件：{currentAsset.originalFilename}
                  </p>{/if}
                <p>
                  {currentAsset.width && currentAsset.height
                    ? `${currentAsset.width} × ${currentAsset.height}`
                    : "分辨率未知"} · {currentAsset.byteSize
                    ? formatBytes(currentAsset.byteSize)
                    : "大小未知"}
                </p>
                <p>
                  {currentAsset.renditions.length
                    ? currentAsset.renditions
                        .map((r) => (r.role === "original" ? "原件" : "浏览图"))
                        .join(" · ")
                    : "示例影像"}
                </p>
                {#if currentAsset.previewError}<p>
                    {currentAsset.previewError}
                  </p>
                {/if}
              </div>
              {#if selected.source}<a
                  href={selected.source}
                  target="_blank"
                  rel="noreferrer"
                  class="block text-[10px] text-ink/40 underline underline-offset-4"
                  >示例图片来源 ↗</a
                >{/if}
              {#if error}<p
                  role="alert"
                  class="whitespace-pre-wrap text-xs text-danger-ink"
                >
                  {error}
                </p>{/if}
            </fieldset>
            <div
              class="flex shrink-0 items-center justify-between gap-2 border-t border-ink/5 px-5 py-3"
            >
              <span class="text-[10px] text-ink/40"
                >{incomplete(draft)
                  ? draft.shotType === "团切"
                    ? "补全日期与团体后离开 Inbox"
                    : "补全日期与人物后离开 Inbox"
                  : "信息完整"}</span
              ><button
                class="btn btn-sm glass-dark rounded-full px-4 text-xs font-normal text-white"
                disabled={saving || librarySession.busy}
                onclick={save}
                >{#if saving}<span class="loading loading-spinner loading-xs"
                  ></span>{:else}<Check size={13} />{/if}保存</button
              >
            </div>
          </aside>{/if}
      </div>
      <footer
        class="glass-dark pointer-events-auto relative mx-auto flex max-w-full shrink-0 items-center gap-4 rounded-2xl px-5 py-3 text-white/85"
      >
        <div class="shrink-0">
          <p class="text-[11px]">这张收藏的影像</p>
          <p class="mt-1 text-[10px] text-white/60">
            {imageIndex + 1} / {selected.assets.length}
          </p>
        </div>
        <div class="flex max-w-[35vw] gap-3 overflow-x-auto p-1">
          {#each selected.assets as asset, i}<button
              class={`h-14 w-11 shrink-0 overflow-hidden rounded-sm ${i === imageIndex ? "ring-1 ring-white/75 ring-offset-2 ring-offset-[#555d4e]" : "opacity-40 hover:opacity-90"}`}
              aria-label={`切换影像 ${asset.originalFilename}`}
              aria-pressed={i === imageIndex}
              onclick={() => {
                imageIndex = i;
                original = false;
              }}
              >{#if asset.src}<img
                  src={asset.src}
                  alt={asset.originalFilename}
                  class="h-full w-full object-contain"
                />{:else}<Camera size={18} />{/if}</button
            >{/each}
        </div>
        <button
          class="btn btn-ghost btn-sm btn-circle bg-surface/10 text-white/80"
          aria-label="为当前收藏添加影像"
          disabled={librarySession.busy || saving}
          onclick={() => prepareImport(selectedId)}
          >{#if librarySession.busy}<span
              class="loading loading-spinner loading-xs"
            ></span>{:else}<Plus size={17} />{/if}</button
        >
        {#if selected.crop || currentAsset.crop}<button
            class="btn btn-ghost btn-sm text-[11px] font-normal text-white/85"
            onclick={() => (original = !original)}
            >{original ? "收藏封面" : "完整影像"}</button
          >{/if}
      </footer>
      {#if librarySession.busy}<p
          role="status"
          class="mt-2 text-center text-xs"
        >
          正在保存原件并生成预览…
        </p>{/if}
    </div>
    {#if fullView}<ImageViewer
        asset={currentAsset}
        displayCrop={coverPhoto(
          { ...selected, coverAssetId: currentAsset.id },
          desktop,
        ).crop}
        onclose={() => (fullView = false)}
      />{/if}
    <TaskCenter />
    {#if importOpen}<ImportSheet
        attached={!!targetCheki}
        onsubmit={beginImport}
        onclose={() => (importOpen = false)}
      />{/if}
  </dialog>
{/if}

{#if !selected}<TaskCenter />{#if importOpen}<ImportSheet
      onsubmit={beginImport}
      onclose={() => (importOpen = false)}
    />{/if}{/if}
{#if settingsOpen}<Settings onclose={() => (settingsOpen = false)} />{/if}
