<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { X, ScanLine, RotateCcw } from "@lucide/svelte";
  import SelectMenu from "./SelectMenu.svelte";
  import { desktop, catalogCommand } from "./session.svelte";
  import { notify } from "./tasks.svelte";
  import type { Asset, Crop } from "./model";
  let { asset, onclose }: { asset: Asset; onclose: () => void } = $props();
  let dialog: HTMLDialogElement;
  let box: HTMLDivElement;
  let shape = $state("54/86");
  let aspect = $state(1);
  let region = $state<Crop>({ x: 0, y: 0, w: 1, h: 1 });
  let busy = $state(false);
  let initialized = false;
  $effect(() => {
    dialog?.showModal();
    if (!initialized) {
      region = asset.crop ? { ...asset.crop } : { x: 0, y: 0, w: 1, h: 1 };
      initialized = true;
    }
  });
  function fit() {
    const [w, h] = shape.split("/").map(Number);
    const normalized = w / h / aspect;
    const nw = Math.min(region.w, region.h * normalized, 1);
    const nh = nw / normalized;
    region = {
      x: Math.min(region.x, 1 - nw),
      y: Math.min(region.y, 1 - nh),
      w: nw,
      h: nh,
    };
  }
  function drag(e: PointerEvent, resize = false) {
    e.preventDefault();
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const start = { ...region };
    const r = box.getBoundingClientRect();
    const x = e.clientX,
      y = e.clientY;
    function move(ev: PointerEvent) {
      const dx = (ev.clientX - x) / r.width,
        dy = (ev.clientY - y) / r.height;
      if (resize) {
        const [w, h] = shape.split("/").map(Number);
        const ratio = w / h / aspect;
        const nw = Math.max(
          0.04,
          Math.min(1 - start.x, (1 - start.y) * ratio, start.w + dx),
        );
        region = { ...start, w: nw, h: nw / ratio };
      } else {
        region = {
          ...start,
          x: Math.max(0, Math.min(1 - start.w, start.x + dx)),
          y: Math.max(0, Math.min(1 - start.h, start.y + dy)),
        };
      }
    }
    function end() {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", end);
      el.removeEventListener("pointercancel", end);
    }
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", end);
    el.addEventListener("pointercancel", end);
  }
  async function suggest() {
    busy = true;
    try {
      region = await invoke<Crop>("crop_suggest", { assetId: asset.id });
      notify("已生成边界建议，请检查后保存");
    } catch (e) {
      notify(String(e), "error");
    } finally {
      busy = false;
    }
  }
  async function save(reset = false) {
    busy = true;
    try {
      if (desktop)
        await catalogCommand("asset_crop", {
          assetId: asset.id,
          crop: reset ? null : region,
        });
      else asset.crop = reset ? null : { ...region };
      notify(reset ? "已恢复完整影像" : "裁切已保存，原件未修改");
      dialog.close();
    } finally {
      busy = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  class="modal bg-tint/30 backdrop-blur-xl"
  aria-label="裁切拍立得"
  {onclose}
  oncancel={(e) => {
    e.stopPropagation();
    if (busy) e.preventDefault();
  }}
>
  <div
    class="modal-box glass-panel w-[min(900px,95vw)] max-w-none rounded-3xl p-6 text-base-content"
  >
    <header class="mb-4 flex items-center justify-between">
      <div>
        <h2>裁切拍立得</h2>
        <p class="mt-1 text-xs text-ink/45">
          拖动选区移动，拖右下角调整大小 · 原件不变
        </p>
      </div>
      <button
        class="btn btn-ghost btn-sm btn-circle"
        aria-label="关闭裁切"
        disabled={busy}
        onclick={() => dialog.close()}><X size={18} /></button
      >
    </header>
    <div
      class="flex min-h-0 items-center justify-center rounded-xl bg-ink/5 p-3"
    >
      <div
        bind:this={box}
        class="relative overflow-hidden"
        style:width={`min(100%,${aspect * 360}px)`}
      >
        <img
          src={asset.baseSrc || asset.src}
          alt="完整扫描，用于调整裁切"
          class="block w-full"
          onload={(e) => {
            aspect =
              (e.currentTarget as HTMLImageElement).naturalWidth /
              (e.currentTarget as HTMLImageElement).naturalHeight;
            if (!asset.crop) fit();
          }}
        />
        <button
          class="absolute touch-none border border-white/80 bg-transparent shadow-[0_0_0_9999px_#0006]"
          style:left={`${region.x * 100}%`}
          style:top={`${region.y * 100}%`}
          style:width={`${region.w * 100}%`}
          style:height={`${region.h * 100}%`}
          aria-label="移动裁切选区"
          onpointerdown={(e) => drag(e)}
          onkeydown={(e) => {
            const delta = 0.005;
            if (e.key === "ArrowLeft") region.x = Math.max(0, region.x - delta);
            if (e.key === "ArrowRight")
              region.x = Math.min(1 - region.w, region.x + delta);
            if (e.key === "ArrowUp") region.y = Math.max(0, region.y - delta);
            if (e.key === "ArrowDown")
              region.y = Math.min(1 - region.h, region.y + delta);
            e.stopPropagation();
          }}
        ></button>
        <button
          class="absolute h-5 w-5 -translate-x-1/2 -translate-y-1/2 touch-none rounded-full bg-surface shadow"
          style:left={`${(region.x + region.w) * 100}%`}
          style:top={`${(region.y + region.h) * 100}%`}
          aria-label="调整裁切大小"
          onpointerdown={(e) => drag(e, true)}
        ></button>
      </div>
    </div>
    <div class="mt-4 flex flex-wrap items-center gap-3">
      <div class="w-48">
        <SelectMenu
          label="拍立得尺寸比例"
          bind:value={shape}
          onchange={fit}
          options={[
            { value: "54/86", label: "Instax Mini · 54 × 86" },
            { value: "72/86", label: "Instax Square · 72 × 86" },
            { value: "108/86", label: "Instax Wide · 108 × 86" },
            { value: "86/54", label: "Mini 横向 · 86 × 54" },
            { value: "86/72", label: "Square 横向 · 86 × 72" },
            { value: "86/108", label: "Wide 竖向 · 86 × 108" },
          ]}
        />
      </div>
      <button
        class="btn btn-ghost btn-sm"
        disabled={!desktop || busy}
        onclick={suggest}><ScanLine size={14} />自动建议</button
      ><button
        class="btn btn-ghost btn-sm"
        disabled={busy}
        onclick={() => save(true)}><RotateCcw size={14} />取消裁切</button
      ><button
        class="btn glass-dark btn-sm ml-auto rounded-full text-white"
        disabled={busy}
        onclick={() => save()}>保存裁切</button
      >
    </div>
  </div>
</dialog>
