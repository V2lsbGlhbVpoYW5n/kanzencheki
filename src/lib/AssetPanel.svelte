<script lang="ts">
  import { Crop as CropIcon, Star, Trash2 } from "@lucide/svelte";
  import type { Asset, Cheki } from "./model";
  import { formatBytes } from "./model";
  import { desktop, catalogCommand, librarySession } from "./session.svelte";
  import { notify } from "./tasks.svelte";
  import CropEditor from "./CropEditor.svelte";
  let {
    asset,
    cheki,
    ondelete,
  }: { asset: Asset; cheki: Cheki; ondelete: () => void } = $props();
  let cropping = $state(false);
  let busy = $state(false);
  let confirming = $state(false);
  let source = $derived(asset.renditions.find((r) => r.role === "original"));
  $effect(() => {
    asset.id;
    confirming = false;
  });
  async function cover(assetId: string | null) {
    busy = true;
    try {
      if (desktop)
        await catalogCommand("cheki_cover", { chekiId: cheki.id, assetId });
      else {
        cheki.coverAssetId = assetId;
        cheki.coverManual = !!assetId;
      }
      notify("封面设置已保存");
    } catch {
    } finally {
      busy = false;
    }
  }
  async function remove() {
    if (!confirming) {
      confirming = true;
      return;
    }
    busy = true;
    try {
      if (desktop)
        await catalogCommand("asset_delete", {
          chekiId: cheki.id,
          assetId: asset.id,
        });
      else {
        cheki.assets = cheki.assets.filter((a) => a.id !== asset.id);
        if (!cheki.assets.length)
          librarySession.photos = librarySession.photos.filter(
            (c) => c.id !== cheki.id,
          );
        else cheki.coverAssetId = cheki.assets[0].id;
      }
      ondelete();
      notify("已删除此影像；不再使用的原件已移入系统回收站");
    } catch {
    } finally {
      busy = false;
      confirming = false;
    }
  }
</script>

<div class="space-y-4 border-t border-black/8 pt-4">
  <div class="flex items-center justify-between">
    <h3 class="text-xs">当前影像</h3>
    <span class="text-[10px] text-black/40"
      >{cheki.coverAssetId === asset.id
        ? cheki.coverManual
          ? "指定封面"
          : "自动封面"
        : ""}</span
    >
  </div>
  <div class="flex flex-wrap gap-1">
    <button
      class="btn btn-ghost btn-xs"
      disabled={!asset.src || busy}
      onclick={() => (cropping = true)}><CropIcon size={12} />裁切</button
    >
    <button
      class="btn btn-ghost btn-xs"
      disabled={busy}
      onclick={() => cover(asset.id)}><Star size={12} />设为封面</button
    >
    {#if cheki.coverManual}<button
        class="btn btn-ghost btn-xs"
        disabled={busy}
        onclick={() => cover(null)}>自动择优</button
      >{/if}
  </div>
  <div class="space-y-2 rounded-xl bg-white/20 p-3 text-[11px] text-black/45">
    <p class="break-all">{asset.filename}</p>
    <p>
      {asset.width && asset.height
        ? `${asset.width} × ${asset.height} · `
        : ""}{formatBytes(asset.byteSize)}
    </p>
    <p>
      {librarySession.locations.find((l) => l.id === source?.locationId)
        ?.name || "本机"} · {source?.available === false
        ? "原件离线"
        : "原件可用"}
    </p>
    {#if source?.available === false}<p>
        仍可浏览已保存的本机预览。需要原件的操作会失败，请先连接目录。
      </p>{/if}
  </div>
  <div class="border-t border-black/8 pt-4">
    {#if confirming}<p class="mb-3 text-xs leading-5 text-error">
        从此收藏删除这份影像。不再被其他收藏使用的原件会移入系统回收站，浏览缓存会清除。{cheki
          .assets.length === 1
          ? "这是最后一份影像，收藏也会删除。"
          : ""}
      </p>{/if}
    <button
      class="btn btn-ghost btn-xs text-error"
      disabled={busy}
      onclick={remove}
      ><Trash2 size={12} />{confirming
        ? "确认删除此影像"
        : "删除此影像…"}</button
    >
    {#if confirming}<button
        class="btn btn-ghost btn-xs"
        disabled={busy}
        onclick={() => (confirming = false)}>取消</button
      >{/if}
  </div>
</div>
{#if cropping}<CropEditor {asset} onclose={() => (cropping = false)} />{/if}
