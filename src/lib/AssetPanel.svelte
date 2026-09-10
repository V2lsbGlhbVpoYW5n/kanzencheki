<script lang="ts">
  import { Crop as CropIcon, Plus, Star, Undo2 } from "@lucide/svelte";
  import type { Asset, Cheki } from "./model";
  import { formatBytes } from "./model";
  import { desktop, catalogCommand, librarySession } from "./session.svelte";
  import { notify } from "./tasks.svelte";
  import SelectMenu from "./SelectMenu.svelte";
  import CropEditor from "./CropEditor.svelte";
  let {
    asset,
    cheki,
    onversion,
  }: { asset: Asset; cheki: Cheki; onversion: () => void } = $props();
  let cropping = $state(false);
  let busy = $state(false);
  async function run(command: string, args: Record<string, unknown>) {
    busy = true;
    try {
      if (desktop) await catalogCommand(command, args);
      else if (command === "cheki_cover") {
        cheki.coverAssetId = args.assetId as string;
        cheki.coverManual = !!args.assetId;
      } else if (command === "asset_kind") asset.kind = args.kind as string;
      if (command !== "rendition_prefer") notify("影像设置已保存");
    } catch {
    } finally {
      busy = false;
    }
  }
</script>

<div class="space-y-3 border-t border-black/8 pt-4">
  <div class="flex items-center justify-between">
    <h3 class="text-xs">影像与文件版本</h3>
    <span class="text-[10px] text-black/40"
      >{cheki.coverAssetId === asset.id
        ? cheki.coverManual
          ? "指定封面"
          : "自动封面"
        : ""}</span
    >
  </div>
  <SelectMenu
    label="影像来源"
    value={asset.kind}
    options={[
      { value: "scan", label: "扫描件" },
      { value: "phone", label: "手机翻拍" },
      { value: "unknown", label: "未指定来源" },
    ]}
    onchange={(kind) => run("asset_kind", { assetId: asset.id, kind })}
    disabled={busy}
  />
  <div class="flex flex-wrap gap-1">
    <button
      class="btn btn-ghost btn-xs"
      disabled={!asset.src || busy}
      onclick={() => (cropping = true)}><CropIcon size={12} />裁切</button
    ><button
      class="btn btn-ghost btn-xs"
      disabled={busy}
      onclick={() =>
        run("cheki_cover", { chekiId: cheki.id, assetId: asset.id })}
      ><Star size={12} />设为封面</button
    >{#if cheki.coverManual}<button
        class="btn btn-ghost btn-xs"
        disabled={busy}
        onclick={() => run("cheki_cover", { chekiId: cheki.id, assetId: null })}
        >自动择优</button
      >{/if}
  </div>
  {#each asset.renditions as r}<div class="rounded-xl bg-white/20 p-3">
      <div class="flex items-center justify-between text-[11px]">
        <span
          >{r.role === "display"
            ? "裁切浏览图"
            : r.role === "base"
              ? "完整离线浏览图"
              : r.mimeType === "image/tiff"
                ? "TIFF 原件"
                : r.role === "original"
                  ? "原始文件"
                  : "同源文件版本"}</span
        ><span class="text-[10px] text-black/40"
          >{r.available === false ? "离线" : "可用"}</span
        >
      </div>
      <p class="mt-1 break-all text-[10px] text-black/40">
        {r.relativePath.split("/").pop()}
      </p>
      <p class="mt-1 text-[10px] text-black/40">
        {r.width && r.height ? `${r.width} × ${r.height} · ` : ""}{formatBytes(
          r.byteSize,
        )} · {librarySession.locations.find((l) => l.id === r.locationId)
          ?.name || "本机"}
      </p>
      {#if r.role === "original" || r.role.startsWith("version:")}<button
          class="btn btn-ghost btn-xs mt-2"
          disabled={!desktop || busy}
          onclick={() =>
            run("rendition_prefer", { assetId: asset.id, renditionId: r.id })}
          >{asset.preferredSource === r.id
            ? "已指定此版本"
            : "使用此版本生成浏览图"}</button
        >{/if}
    </div>{/each}
  <button
    class="btn btn-ghost btn-xs"
    disabled={!desktop || busy}
    onclick={() =>
      run("rendition_prefer", { assetId: asset.id, renditionId: null })}
    >文件版本自动择优</button
  >
  <button class="btn btn-ghost btn-xs" onclick={onversion}
    ><Plus size={12} />添加同源文件版本</button
  >
  {#if cheki.assets.length > 1}<button
      class="btn btn-ghost btn-xs"
      disabled={busy || !desktop}
      onclick={() =>
        run("asset_detach", { chekiId: cheki.id, assetId: asset.id })}
      ><Undo2 size={12} />拆成独立收藏</button
    >{/if}
</div>
{#if cropping}<CropEditor {asset} onclose={() => (cropping = false)} />{/if}
