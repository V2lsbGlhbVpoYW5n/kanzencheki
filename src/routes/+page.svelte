<script lang="ts">
  import { Images, Heart, Plus, Search, X, ArrowDownWideNarrow, Check, Info, Pencil, ArrowLeft, Camera } from '@lucide/svelte';
  import PhotoImage from '$lib/Photo.svelte';
  import type { Photo } from '$lib/demo';
  import { librarySession } from '$lib/session.svelte';
  let photos = $derived(librarySession.photos);
  let scope = $state('全部');
  let tag = $state('所有标签');
  let query = $state('');
  let month = $state('');
  let size = $state(210);
  let selectedId = $state<string | null>(null);
  let imageIndex = $state(0);
  let original = $state(false);
  let showInfo = $state(true);
  let editing = $state(false);
  let ascending = $state(false);
  let labels = $state(true);
  let filtersOpen = $state(false);
  let input: HTMLInputElement;
  let imageInput = $state<HTMLInputElement>();
  let importTarget: string | null = null;
  let notice = $state('');
  let viewer = $state<HTMLDialogElement>();
  $effect(() => { if (selectedId && viewer && !viewer.open) viewer.showModal(); });
  let visible = $derived(photos.filter(p => (scope === '喜欢' ? p.favorite : scope === '待整理' ? p.inbox : scope === '本次导入' ? !p.source : true) && (tag === '所有标签' || p.tag === tag) && (!month || p.date.startsWith(month)) && `${p.title} ${p.tag} ${p.date}`.toLowerCase().includes(query.toLowerCase())).sort((a,b) => ascending ? a.date.localeCompare(b.date) : b.date.localeCompare(a.date)));
  let selected = $derived(photos.find(p => p.id === selectedId));
  let currentImage = $derived(selected?.images?.[imageIndex]);
  let displayed = $derived(selected && currentImage ? {...selected, src:currentImage.src, crop:imageIndex === 0 ? selected.crop : undefined} : selected);
  function close() { viewer?.close(); selectedId = null; }
  function open(p: Photo) { filtersOpen = false; selectedId = p.id; imageIndex = 0; original = false; editing = false; }
  function switchImage(index: number) { imageIndex = index; original = index !== 0; }
  function keyboard(e: KeyboardEvent) {
    if(e.key === 'Escape' && !selected) filtersOpen = false;
    if ((e.target as HTMLElement)?.matches('input, textarea, select')) return;
    if(selected && ['ArrowLeft','ArrowRight'].includes(e.key)) { e.preventDefault(); const n = selected.images?.length ?? 1; switchImage((imageIndex + (e.key === 'ArrowLeft' ? -1 : 1) + n) % n); }
  }
  function chooseFiles(target: string | null = null) { importTarget = target; (target ? imageInput : input)?.click(); }
  function importFiles(e: Event) {
    const files = [...((e.target as HTMLInputElement).files ?? [])];
    if(!files.length) return;
    let skipped = 0;
    const target = photos.find(p => p.id === importTarget);
    for(const file of files) {
      if(!['image/jpeg','image/png','image/webp'].includes(file.type)) { skipped++; continue; }
      const src = URL.createObjectURL(file);
      const image = {id:crypto.randomUUID(),src,name:file.name};
      if(target) { target.images ??= []; target.images.push(image); }
      else photos.push({ id:crypto.randomUUID(), title:file.name, date:'', tag:'未分类', src, favorite:false, inbox:true, notes:'', images:[image] });
    }
    if(!target) { scope = '本次导入'; query = ''; tag = '所有标签'; month = ''; }
    notice = `已加入 ${files.length-skipped} 份${target ? '关联影像' : '收藏预览'}${skipped ? `，跳过 ${skipped} 个不支持的文件` : ''}。仅在本次会话保留，原文件不变。`;
    (e.target as HTMLInputElement).value = '';
  }
</script>
<svelte:head><title>Cheki — 全览相册</title></svelte:head>
<svelte:window onkeydown={keyboard}/>
<input class="hidden" bind:this={input} type="file" multiple accept="image/jpeg,image/png,image/webp" onchange={importFiles} aria-label="选择照片"/>
<main aria-label="全览相册" class="h-screen overflow-y-auto bg-[#eeede8] text-[#3c4037]">
  <section class="px-8 pb-36 pt-28 sm:px-12" aria-label="收藏网格">
    {#if notice}<div role="status" class="mb-5 flex items-center justify-between glass-panel rounded-lg border px-4 py-3 text-xs">{notice}<button class="btn btn-ghost btn-xs btn-circle" aria-label="关闭提示" onclick={() => notice = ''}><X size={14}/></button></div>{/if}
    <div class="grid gap-x-8 gap-y-10" style:grid-template-columns={`repeat(auto-fill, minmax(min(${size}px, 100%), 1fr))`}>
      {#each visible as photo}
        <div class="group min-w-0">
          <button class="relative block w-full rounded-sm text-left outline-offset-8 transition-transform duration-200 hover:-translate-y-1 focus-visible:outline-2 focus-visible:outline-[#798468] motion-reduce:transform-none" aria-label={`查看 ${photo.title}`} onclick={() => open(photo)}>
            <div class="aspect-[3/4] w-full overflow-hidden bg-white shadow-[0_2px_4px_#00000010,0_12px_22px_-12px_#00000040]"><PhotoImage {photo}/></div>
            {#if photo.favorite}<span class="absolute bottom-3 right-3 glass-panel rounded-full border p-1.5 text-[#8b655f]"><Heart size={12} fill="currentColor"/></span>{/if}
            {#if photo.inbox}<span class="absolute right-3 top-3 h-2 w-2 rounded-full bg-[#bc9d62] ring-2 ring-white"></span>{/if}
            {#if (photo.images?.length ?? 0)>1}<span class="absolute bottom-3 left-3 flex items-center gap-1 glass-panel rounded-full border px-2 py-1 text-[10px]"><Images size={11}/>{photo.images?.length}</span>{/if}
          </button>
          {#if labels}<div class="mt-3 flex items-center justify-between gap-2"><span class="truncate text-[11px] text-black/60">{photo.title}</span><span class="shrink-0 text-[10px] text-black/35">{photo.date ? photo.date.slice(5).replace('-',' / ') : '待整理'}</span></div>{/if}
        </div>
      {:else}
        <div class="col-span-full flex h-80 flex-col items-center justify-center text-black/45"><Images size={30} strokeWidth={1}/><p class="mt-5 text-sm">没有匹配的照片</p><button class="btn btn-ghost btn-sm mt-3 text-xs" onclick={() => { query = ''; scope = '全部'; tag = '所有标签'; month = ''; }}>查看全部收藏</button></div>
      {/each}
    </div>
    <p class="mt-12 text-center text-[10px] tracking-wider text-black/35">示例图库 · 网络图片，日期与标签为演示</p>
  </section>
    {#if filtersOpen}
      <section aria-label="搜索与筛选" class="fixed bottom-[106px] left-1/2 z-30 w-[min(560px,calc(100vw-32px))] -translate-x-1/2 rounded-2xl glass-panel border p-5">
        <div class="flex items-center gap-3"><label class="input input-sm h-10 flex-1 rounded-full border-transparent bg-white/35 px-4 shadow-[inset_0_1px_3px_#28301e12]"><Search size={15} class="text-black/40"/><input aria-label="搜索收藏" placeholder="搜索照片、标签…" bind:value={query}/>{#if query}<button class="btn btn-ghost btn-xs btn-circle" aria-label="清除搜索" onclick={() => query = ''}><X size={12}/></button>{/if}</label><button class="btn btn-ghost btn-sm btn-circle" aria-label="收起搜索与筛选" onclick={() => filtersOpen = false}><X size={16}/></button></div>
        <div class="mt-4 flex items-center justify-between"><div class="tabs tabs-box gap-1 rounded-full bg-black/4 p-1" aria-label="收藏筛选">{#each ['全部','喜欢','待整理'] as item}<button class={`tab h-7 rounded-full px-4 text-xs ${scope === item ? 'bg-white/55 shadow-sm' : 'text-black/45'}`} aria-pressed={scope === item} onclick={() => scope = item}>{item}</button>{/each}</div><span class="text-[11px] text-black/45">{visible.length} 张收藏</span></div>
        <div class="mt-4 flex flex-wrap items-center gap-3 border-t border-black/8 pt-3"><select aria-label="标签筛选" class="select select-ghost select-sm w-28 text-xs text-black/55" bind:value={tag}>{#each ['所有标签','演出','咖啡店','纪念','返切','未分类'] as item}<option>{item}</option>{/each}</select><label class="flex items-center gap-2 text-xs text-black/50">日期<input class="input input-ghost input-sm w-36 text-xs" type="month" aria-label="按月份筛选" bind:value={month}/></label>{#if month || tag !== '所有标签' || scope !== '全部' || query}<button class="btn btn-ghost btn-xs ml-auto text-black/50" onclick={() => {month = ''; tag = '所有标签'; scope = '全部'; query = '';}}>重置</button>{/if}</div>
      </section>
    {/if}
  <nav aria-label="相册工具" class="fixed bottom-7 left-1/2 z-20 flex -translate-x-1/2 items-center gap-4 rounded-full glass-light border px-5 py-2.5">
    <button class={`btn btn-ghost btn-sm gap-2 rounded-full px-2 text-xs font-normal ${filtersOpen ? 'bg-black/5' : ''}`} aria-label="搜索与筛选" aria-expanded={filtersOpen} onclick={() => filtersOpen = !filtersOpen}><Search size={17}/><span class="text-black/50">{visible.length}</span>{#if query || month || tag !== '所有标签' || scope !== '全部'}<span class="h-1.5 w-1.5 rounded-full bg-[#82936c]"></span>{/if}</button><span class="h-5 border-l border-black/10"></span>
    <Images size={17} strokeWidth={1.5} class="text-black/45"/><input class="range range-xs w-24 text-[#7b826e]" aria-label="照片大小" type="range" min="150" max="290" step="10" bind:value={size}/>
    <span class="h-5 border-l border-black/10"></span><button class={`btn btn-ghost btn-sm btn-circle ${labels ? 'bg-black/5' : ''}`} aria-label="切换照片标题" aria-pressed={labels} onclick={() => labels = !labels}><Info size={16}/></button>
    <button class="btn btn-ghost btn-sm btn-circle" aria-label={ascending ? '改为最近在前' : '改为最早在前'} title={ascending ? '最早在前' : '最近在前'} onclick={() => ascending = !ascending}><ArrowDownWideNarrow size={17} class={ascending ? 'rotate-180' : ''}/></button>
    <span class="h-5 border-l border-black/10"></span><button class="btn btn-sm gap-2 rounded-full glass-dark border px-4 text-xs font-normal text-white hover:brightness-110" onclick={() => chooseFiles()}><Plus size={15}/>导入照片</button>
  </nav>
</main>
{#if selected && displayed}
  <dialog bind:this={viewer} onclose={() => selectedId = null} class="fixed inset-0 m-0 h-screen max-h-none w-screen max-w-none overflow-hidden border-0 bg-transparent p-0 text-[#353a30] backdrop:bg-transparent" aria-label="收藏聚焦预览">
    <input class="hidden" bind:this={imageInput} type="file" multiple accept="image/jpeg,image/png,image/webp" onchange={importFiles} aria-label="选择关联影像"/>
    <button class="absolute inset-0 h-full w-full cursor-default bg-[#d6d8cf]/25 backdrop-blur-[28px] backdrop-saturate-75" aria-label="点击背景返回相册" tabindex="-1" onclick={close}></button>
    <div class="pointer-events-none relative flex h-full flex-col px-8 py-6">
      <header class="flex shrink-0 items-center justify-between"><button class="btn pointer-events-auto gap-2 rounded-full glass-light text-xs font-normal" onclick={close}><ArrowLeft size={15}/>返回相册</button><div class="pointer-events-auto flex items-center gap-2 rounded-full glass-light border p-1"><button class={`btn btn-ghost btn-sm btn-circle ${selected.favorite ? 'text-[#9a6c67]' : ''}`} aria-label={selected.favorite ? '取消喜欢' : '设为喜欢'} onclick={() => selected && (selected.favorite = !selected.favorite)}><Heart size={17} fill={selected.favorite ? 'currentColor' : 'none'}/></button><button class={`btn btn-ghost btn-sm btn-circle ${showInfo ? 'bg-black/5' : ''}`} aria-label="切换收藏信息" aria-pressed={showInfo} onclick={() => showInfo = !showInfo}><Info size={17}/></button><button class="btn btn-ghost btn-sm btn-circle" aria-label="关闭预览" onclick={close}><X size={18}/></button></div></header>
      <div class="flex min-h-0 flex-1 items-center justify-center gap-10 py-6">
        <div class="pointer-events-auto flex h-full min-w-0 flex-col items-center justify-center">
          <div class="aspect-[3/4] min-h-0 max-w-[50vw] overflow-hidden bg-[#fafaf7] shadow-[0_25px_65px_-15px_#30372a65,0_2px_8px_#00000015]" style:height="min(100%, 660px)"><PhotoImage photo={displayed} full={original}/></div>
        </div>
        {#if showInfo}
          <aside class="pointer-events-auto max-h-full w-60 shrink-0 overflow-y-auto rounded-2xl glass-panel border p-5">
            <p class="mb-3 text-[10px] tracking-[.18em] text-black/40">收藏</p><h2 class="text-base font-medium leading-6">{selected.title}</h2>
            <p class="mt-2 text-xs text-black/45">{selected.date || '日期待补充'}</p><span class="badge badge-sm mt-4 border-0 bg-[#dce2ce]/55 text-[10px] text-[#69725c]">{selected.tag}</span>
            {#if selected.notes}<p class="mt-5 whitespace-pre-wrap text-xs leading-6 text-black/60">{selected.notes}</p>{/if}
            <div class="mt-6 flex items-center justify-between border-t border-black/8 pt-4"><span class="flex items-center gap-2 text-[11px] text-black/45"><Camera size={13}/>{selected.images?.length ?? 1} 份影像</span><button class="btn btn-ghost btn-xs btn-circle text-black/45" aria-label="编辑收藏信息" aria-expanded={editing} onclick={() => editing = !editing}><Pencil size={13}/></button></div>
            {#if editing}<div class="mt-4 space-y-3"><label class="block text-[10px] text-black/50">日期<input type="date" class="input input-sm mt-1 w-full bg-white/40 text-xs" bind:value={selected.date}/></label><label class="block text-[10px] text-black/50">标签<select class="select select-sm mt-1 w-full bg-white/40 text-xs" bind:value={selected.tag}>{#each ['演出','咖啡店','纪念','返切','未分类'] as t}<option>{t}</option>{/each}</select></label><label class="block text-[10px] text-black/50">备注<textarea class="textarea mt-1 w-full bg-white/40 text-xs" rows="2" bind:value={selected.notes}></textarea></label>{#if selected.inbox}<button class="btn btn-sm w-full text-xs font-normal" onclick={() => selected && (selected.inbox = false)}><Check size={13}/>完成整理</button>{/if}<p class="text-[10px] text-black/40">修改仅在本次会话保留</p></div>{/if}
            {#if selected.source && imageIndex === 0}<a href={selected.source} target="_blank" rel="noreferrer" class="mt-4 block text-[10px] text-black/40 underline underline-offset-4">示例图片来源 ↗</a>{/if}
          </aside>
        {/if}
      </div>
      <footer class="pointer-events-auto mx-auto flex max-w-full shrink-0 items-center gap-5 rounded-2xl glass-dark border px-5 py-3 text-white/85">
        <div class="shrink-0"><p class="text-[11px]">这张收藏的影像</p><p class="mt-1 text-[10px] text-white/60">{imageIndex + 1} / {selected.images?.length ?? 1}</p></div>
        <div class="flex max-w-[40vw] gap-3 overflow-x-auto p-1">{#each selected.images ?? [] as asset, i}<button class={`h-14 w-11 shrink-0 overflow-hidden rounded-sm transition-opacity ${i === imageIndex ? 'ring-1 ring-white/75 ring-offset-2 ring-offset-[#555d4e]' : 'opacity-40 hover:opacity-90'}`} aria-label={`切换影像 ${asset.name}`} aria-pressed={i === imageIndex} title={asset.name} onclick={() => switchImage(i)}><img src={asset.src} alt={asset.name} class="h-full w-full object-contain"/></button>{/each}</div>
        <button class="btn btn-ghost btn-sm btn-circle border-transparent bg-white/10 text-white/80 shadow-sm" aria-label="为当前收藏添加影像" title="添加临时关联影像" onclick={() => chooseFiles(selected?.id)}><Plus size={17}/></button>
        <span class="h-7 border-l border-white/15"></span><button class="btn btn-ghost btn-sm text-[11px] font-normal text-white/85" onclick={() => original = !original}>{original ? '收藏封面' : '完整影像'}</button>
      </footer>
      {#if notice}<p role="status" class="mt-2 text-center text-[10px] text-black/65">{notice}</p>{/if}
    </div>
  </dialog>
{/if}
