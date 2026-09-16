<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, convertFileSrc } from "@tauri-apps/api/core";
  import { X, Plus, Minus, Scan, Move } from "@lucide/svelte";
  import { desktop } from "./session.svelte";
  import type { Asset } from "./model";
  let {
    asset,
    onclose,
    displayCrop,
  }: {
    asset: Asset;
    onclose: () => void;
    displayCrop?: { x: number; y: number; w: number; h: number };
  } = $props();
  let dialog: HTMLDialogElement, viewport: HTMLDivElement;
  let width = $state(1),
    height = $state(1),
    iw = $state(1),
    ih = $state(1),
    zoom = $state(1),
    x = $state(0),
    y = $state(0),
    view = $state<"crop" | "original">("crop"),
    loading = $state(false),
    error = $state(""),
    originalSrc = $state(""),
    isOriginal = $state(false);
  let cropSrc = $state("");
  let temporary = "";
  let disposed = false;
  let fit = $derived(
    Math.max(0.001, Math.min((width - 48) / iw, (height - 140) / ih, 1)),
  );
  let src = $derived(
    view === "original"
      ? originalSrc || asset.baseSrc || asset.src
      : cropSrc || asset.src,
  );
  function reset() {
    zoom = 1;
    x = 0;
    y = 0;
  }
  function scale(next: number, px = 0, py = 0) {
    next = Math.max(0.1, Math.min(Math.max(32, 8 / fit), next));
    x = px - ((px - x) * next) / zoom;
    y = py - ((py - y) * next) / zoom;
    zoom = next;
    bound();
  }
  function bound() {
    const mx = Math.max(0, (iw * fit * zoom) / 2 + width / 2 - 64),
      my = Math.max(0, (ih * fit * zoom) / 2 + height / 2 - 64);
    x = Math.max(-mx, Math.min(mx, x));
    y = Math.max(-my, Math.min(my, y));
  }
  function wheel(e: WheelEvent) {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      const r = viewport.getBoundingClientRect();
      scale(
        zoom * Math.exp(-e.deltaY * 0.002),
        e.clientX - r.left - width / 2,
        e.clientY - r.top - height / 2,
      );
    } else if (zoom > 1) {
      e.preventDefault();
      x -= e.deltaX;
      y -= e.deltaY;
      bound();
    }
  }
  function keys(e: KeyboardEvent) {
    e.stopPropagation();
    if (
      [
        "+",
        "=",
        "-",
        "0",
        "1",
        "ArrowLeft",
        "ArrowRight",
        "ArrowUp",
        "ArrowDown",
      ].includes(e.key)
    ) {
      e.preventDefault();
      if (e.key === "+" || e.key === "=") scale(zoom * 1.25);
      if (e.key === "-") scale(zoom / 1.25);
      if (e.key === "0") reset();
      if (e.key === "1") scale(1 / fit);
      const step = e.shiftKey ? 120 : 40;
      if (e.key === "ArrowLeft") x -= step;
      if (e.key === "ArrowRight") x += step;
      if (e.key === "ArrowUp") y -= step;
      if (e.key === "ArrowDown") y += step;
      bound();
    }
  }
  function drag(e: PointerEvent) {
    if ((e.target as HTMLElement).closest("button")) return;
    e.preventDefault();
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const px = e.clientX,
      py = e.clientY,
      sx = x,
      sy = y;
    const move = (e: PointerEvent) => {
      x = sx + e.clientX - px;
      y = sy + e.clientY - py;
      bound();
    };
    const end = () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", end);
      el.removeEventListener("pointercancel", end);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", end);
    el.addEventListener("pointercancel", end);
  }
  async function showOriginal() {
    view = "original";
    reset();
    if (originalSrc || loading) return;
    if (!desktop) {
      originalSrc = asset.baseSrc || asset.src;
      isOriginal = true;
      return;
    }
    loading = true;
    error = "";
    try {
      const result = await invoke<{
        path: string;
        original: boolean;
        temporary: boolean;
      }>("asset_view", { assetId: asset.id });
      if (disposed) {
        if (result.temporary)
          void invoke("asset_view_release", { path: result.path }).catch(
            () => {},
          );
        return;
      }
      isOriginal = result.original;
      originalSrc = convertFileSrc(result.path) + `?view=${Date.now()}`;
      if (result.temporary) temporary = result.path;
    } catch (e) {
      error = String(e);
      isOriginal = false;
    } finally {
      loading = false;
    }
  }
  onMount(() => {
    if (displayCrop && !desktop) {
      const crop = displayCrop,
        img = new Image();
      img.onload = () => {
        if (disposed) return;
        const canvas = document.createElement("canvas");
        canvas.width = Math.max(
          1,
          Math.round((img.naturalWidth * crop.w) / 100),
        );
        canvas.height = Math.max(
          1,
          Math.round((img.naturalHeight * crop.h) / 100),
        );
        canvas
          .getContext("2d")
          ?.drawImage(
            img,
            (img.naturalWidth * crop.x) / 100,
            (img.naturalHeight * crop.y) / 100,
            canvas.width,
            canvas.height,
            0,
            0,
            canvas.width,
            canvas.height,
          );
        cropSrc = canvas.toDataURL();
      };
      img.src = asset.src;
    }
    dialog.showModal();
    viewport.focus();
    viewport.addEventListener("wheel", wheel, { passive: false });
    return () => {
      disposed = true;
      viewport.removeEventListener("wheel", wheel);
      if (temporary)
        void invoke("asset_view_release", { path: temporary }).catch(() => {});
    };
  });
</script>

<dialog
  bind:this={dialog}
  class="fixed inset-0 m-0 h-dvh max-h-none w-screen max-w-none overflow-hidden border-0 bg-canvas/90 p-0 text-base-content backdrop-blur-2xl"
  aria-label="全窗口图像查看"
  {onclose}
  onkeydown={keys}
  oncancel={(e) => e.stopPropagation()}
>
  <!-- Interactive spatial viewport supports pointer drag and keyboard pan. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex a11y_no_noninteractive_element_interactions -->
  <div
    bind:this={viewport}
    bind:clientWidth={width}
    bind:clientHeight={height}
    role="application"
    aria-label="图像缩放与平移，Ctrl 加滚轮缩放，方向键移动"
    tabindex="0"
    class="absolute inset-0 touch-none overflow-hidden outline-none"
    onpointerdown={drag}
  >
    <img
      {src}
      alt={asset.filename}
      draggable="false"
      class="absolute left-1/2 top-1/2 max-w-none select-none cursor-grab active:cursor-grabbing"
      style:width={`${iw * fit}px`}
      style:height={`${ih * fit}px`}
      style:transform={`translate(-50%,-50%) translate(${x}px,${y}px) scale(${zoom})`}
      onload={(e) => {
        iw = (e.currentTarget as HTMLImageElement).naturalWidth;
        ih = (e.currentTarget as HTMLImageElement).naturalHeight;
      }}
      onerror={() => {
        if (view === "original" && originalSrc) {
          originalSrc = asset.baseSrc || asset.src;
          isOriginal = false;
          error = "原图无法显示，已切换本机压缩预览";
        } else error = "图像暂不可用";
      }}
    />
  </div>
  <header
    class="pointer-events-none absolute left-6 right-6 top-5 flex items-center gap-3"
  >
    <div class="glass-panel min-w-0 rounded-full px-4 py-2 text-xs">
      <p class="truncate">{asset.filename}</p>
    </div>
    <button
      class="btn glass-panel btn-sm btn-circle pointer-events-auto ml-auto"
      aria-label="关闭全窗口查看"
      onclick={() => dialog.close()}><X size={18} /></button
    >
  </header>
  <footer
    class="glass-panel absolute bottom-6 left-1/2 flex w-max max-w-[95vw] -translate-x-1/2 items-center gap-2 rounded-full px-4 py-2 text-xs"
  >
    <button
      class={`btn btn-ghost btn-sm rounded-full ${view === "crop" ? "bg-tint/40" : ""}`}
      aria-pressed={view === "crop"}
      onclick={() => {
        view = "crop";
        reset();
      }}>裁切图</button
    ><button
      class={`btn btn-ghost btn-sm rounded-full ${view === "original" ? "bg-tint/40" : ""}`}
      aria-pressed={view === "original"}
      onclick={showOriginal}
      >{loading
        ? "加载原图…"
        : view === "original" && !isOriginal
          ? "原图 · 压缩预览"
          : "原图"}</button
    ><span class="mx-1 h-5 border-l border-ink/15"></span><button
      class="btn btn-ghost btn-sm btn-circle"
      aria-label="缩小"
      onclick={() => scale(zoom / 1.25)}><Minus size={16} /></button
    ><span class="w-12 text-center tabular-nums"
      >{Math.round(zoom * fit * 100)}%</span
    ><button
      class="btn btn-ghost btn-sm btn-circle"
      aria-label="放大"
      onclick={() => scale(zoom * 1.25)}><Plus size={16} /></button
    ><button
      class="btn btn-ghost btn-sm btn-circle"
      aria-label="适应窗口"
      title="适应窗口（0）"
      onclick={reset}><Scan size={16} /></button
    ><button
      class="btn btn-ghost btn-sm rounded-full"
      onclick={() => scale(1 / fit)}>1:1</button
    >
  </footer>
  {#if error}<p
      class="glass-panel absolute bottom-24 left-1/2 max-w-xl -translate-x-1/2 rounded-xl px-4 py-2 text-xs"
      role="alert"
    >
      {error}
    </p>{:else}<p
      class="pointer-events-none absolute bottom-24 left-1/2 -translate-x-1/2 whitespace-nowrap text-[11px] text-ink/45"
    >
      <Move size={12} class="mr-1 inline" />拖拽 / 方向键移动 · Ctrl + 滚轮 /
      加减号缩放
    </p>{/if}
</dialog>
