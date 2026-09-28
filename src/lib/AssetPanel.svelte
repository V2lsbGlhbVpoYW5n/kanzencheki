<script lang="ts">
  import { sourceMessage, tr } from "$lib/i18n.svelte";
  import { Crop as CropIcon, Star, Trash2, RotateCw } from "@lucide/svelte";
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
  async function rotate() {
    if (busy) return;
    busy = true;
    try {
      if (!desktop) {
        notify(sourceMessage("写入原件的旋转请使用桌面版"), "error");
        return;
      }
      await catalogCommand("asset_rotate", { assetId: asset.id });
      notify(sourceMessage("原件与浏览图已顺时针旋转 90°"));
    } catch {
    } finally {
      busy = false;
    }
  }
  async function rebuildPreview() {
    if (busy || !desktop) return;
    busy = true;
    try {
      await catalogCommand("asset_refresh_preview", { assetId: asset.id });
      notify(sourceMessage("浏览图已重新生成"));
    } catch {
    } finally {
      busy = false;
    }
  }
  async function cover(assetId: string | null) {
    busy = true;
    try {
      if (desktop)
        await catalogCommand("cheki_cover", { chekiId: cheki.id, assetId });
      else {
        cheki.coverAssetId = assetId;
        cheki.coverManual = !!assetId;
      }
      notify(sourceMessage("封面设置已保存"));
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
      notify(sourceMessage("已删除此影像；不再使用的原件已移入系统回收站"));
    } catch {
    } finally {
      busy = false;
      confirming = false;
    }
  }
</script>

<div class="space-y-4 border-t border-ink/8 pt-4">
  <div class="flex items-center justify-between">
    <h3 class="text-xs">{tr("当前影像")}</h3>
    <span class="text-[10px] text-ink/40"
      >{cheki.coverAssetId === asset.id
        ? cheki.coverManual
          ? tr("指定封面")
          : tr("自动封面")
        : ""}</span
    >
  </div>
  <div class="flex flex-wrap gap-1">
    <button
      class="btn btn-ghost btn-xs"
      disabled={!asset.src || busy}
      onclick={() => (cropping = true)}
      ><CropIcon size={12} />{tr("裁切 / 变形")}</button
    >
    <button
      class="btn btn-ghost btn-xs"
      disabled={busy || source?.available === false}
      title={tr("将原件及所有浏览图顺时针旋转 90°")}
      onclick={rotate}
      ><RotateCw size={12} />{busy ? tr("处理中…") : tr("顺时针旋转")}</button
    >
    {#if desktop && (asset.previewError || !asset.src)}<button
      class="btn btn-ghost btn-xs"
      disabled={busy || source?.available === false}
      onclick={rebuildPreview}
      ><RotateCw size={12} />{tr("重新生成浏览图")}</button
    >{/if}
    <button
      class="btn btn-ghost btn-xs"
      disabled={busy}
      onclick={() => cover(asset.id)}><Star size={12} />{tr("设为封面")}</button
    >
    {#if cheki.coverManual}<button
        class="btn btn-ghost btn-xs"
        disabled={busy}
        onclick={() => cover(null)}>{tr("自动择优")}</button
      >{/if}
  </div>
  <p class="text-[10px] text-ink/40">
    {tr("裁切与变形保留原件；旋转写回原件，JPEG 会重新编码。")}
  </p>
  <div class="space-y-2 rounded-xl bg-surface/20 p-3 text-[11px] text-ink/45">
    <p class="break-all">{asset.filename}</p>
    <p>
      {asset.width && asset.height
        ? `${asset.width} × ${asset.height} · `
        : ""}{formatBytes(asset.byteSize)}
    </p>
    <p>
      {librarySession.locations.find((l) => l.id === source?.locationId)
        ?.name || tr("本机")} · {source?.available === false
        ? tr("原件离线")
        : tr("原件可用")}
    </p>
    {#if source?.available === false}<p>
        {tr("仍可浏览已保存的本机预览。需要原件的操作会失败，请先连接目录。")}
      </p>{/if}
  </div>
  <div class="border-t border-ink/8 pt-4">
    {#if confirming}<p class="mb-3 text-xs leading-5 text-error">
        {tr(
          "从此收藏删除这份影像。不再被其他收藏使用的原件会移入系统回收站，浏览缓存会清除。{0}",
          [
            cheki.assets.length === 1
              ? tr("这是最后一份影像，收藏也会删除。")
              : "",
          ],
        )}
      </p>{/if}
    <button
      class="btn btn-ghost btn-xs text-error"
      disabled={busy}
      onclick={remove}
      ><Trash2 size={12} />{confirming
        ? tr("确认删除此影像")
        : tr("删除此影像…")}</button
    >
    {#if confirming}<button
        class="btn btn-ghost btn-xs"
        disabled={busy}
        onclick={() => (confirming = false)}>{tr("取消")}</button
      >{/if}
  </div>
</div>
{#if cropping}<CropEditor {asset} onclose={() => (cropping = false)} />{/if}
