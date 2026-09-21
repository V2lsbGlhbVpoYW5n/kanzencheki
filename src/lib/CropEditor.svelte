<script lang="ts">
  import { sourceMessage, message, tr } from "$lib/i18n.svelte";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    X,
    ScanLine,
    RotateCcw,
    Eye,
    Check,
    Crop as CropIcon,
    Scan,
  } from "@lucide/svelte";
  import SelectMenu from "./SelectMenu.svelte";
  import { desktop, catalogCommand } from "./session.svelte";
  import { notify } from "./tasks.svelte";
  import type { Asset, Crop, Point } from "./model";
  let { asset, onclose }: { asset: Asset; onclose: () => void } = $props();
  let dialog: HTMLDialogElement, box: HTMLDivElement;
  let previewKey = $state("");
  const lensSize = 144,
    lensZoom = 4;
  let lens = $state<{
    index: number;
    left: number;
    top: number;
    width: number;
    height: number;
  } | null>(null);
  let stopDrag: (() => void) | undefined;
  function showLens(e: PointerEvent, index: number, bounds: DOMRect) {
    const gap = 24,
      margin = 12;
    const left =
      e.clientX + gap + lensSize <= window.innerWidth - margin
        ? e.clientX + gap
        : e.clientX - gap - lensSize;
    const top =
      e.clientY - gap - lensSize >= margin
        ? e.clientY - gap - lensSize
        : e.clientY + gap;
    lens = {
      index,
      left: Math.max(
        margin,
        Math.min(window.innerWidth - lensSize - margin, left),
      ),
      top: Math.max(
        margin,
        Math.min(window.innerHeight - lensSize - margin, top),
      ),
      width: bounds.width * lensZoom,
      height: bounds.height * lensZoom,
    };
  }
  let mode = $state<"rect" | "quad">("rect"),
    shape = $state("free"),
    aspect = $state(1),
    busy = $state(false),
    error = $state(""),
    preview = $state("");
  let region = $state<Crop>({ x: 0, y: 0, w: 1, h: 1 });
  let quad = $state<Point[]>([
    { x: 0, y: 0 },
    { x: 1, y: 0 },
    { x: 1, y: 1 },
    { x: 0, y: 1 },
  ]);
  let corners = $derived(
    mode === "quad"
      ? quad
      : [
          { x: region.x, y: region.y },
          { x: region.x + region.w, y: region.y },
          { x: region.x + region.w, y: region.y + region.h },
          { x: region.x, y: region.y + region.h },
        ],
  );
  let ratio = $derived(
    shape === "free"
      ? null
      : Number(shape.split("/")[0]) / Number(shape.split("/")[1]),
  );
  let crop = $derived<Crop>(
    mode === "quad"
      ? {
          x: 0,
          y: 0,
          w: 1,
          h: 1,
          quad: quad as [Point, Point, Point, Point],
          ratio,
        }
      : { ...region, quad: null, ratio: null },
  );
  let valid = $derived(
    mode === "rect" ||
      quad.every((a, i) => {
        const b = quad[(i + 1) % 4],
          c = quad[(i + 2) % 4];
        return (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x) > 0.0001;
      }),
  );
  let outline = $derived(
    corners.map((p) => `${p.x * 100},${p.y * 100}`).join(" "),
  );
  onMount(() => {
    if (asset.crop) {
      region = { ...asset.crop };
      if (asset.crop.quad) {
        quad = asset.crop.quad.map((p) => ({ ...p }));
        mode = "quad";
      }
      if (asset.crop.ratio)
        shape =
          ["54/86", "72/86", "108/86", "86/54", "86/72", "86/108", "1/1"].find(
            (s) =>
              Math.abs(
                Number(s.split("/")[0]) / Number(s.split("/")[1]) -
                  asset.crop!.ratio!,
              ) < 0.000001,
          ) ?? "free";
    }
    dialog.showModal();
    return () => {
      stopDrag?.();
      if (preview) URL.revokeObjectURL(preview);
    };
  });
  let previewStale = $derived(
    preview !== "" && previewKey !== JSON.stringify(crop),
  );
  const clamp = (n: number) => Math.max(0, Math.min(1, n));
  function fit() {
    if (!ratio || mode === "quad") return;
    const r = ratio / aspect,
      w = Math.min(region.w, region.h * r);
    region = { ...region, w, h: w / r };
  }
  function switchMode(next: typeof mode) {
    if (next === "quad") quad = corners.map((p) => ({ ...p }));
    else {
      const xs = corners.map((p) => p.x),
        ys = corners.map((p) => p.y);
      region = {
        x: Math.min(...xs),
        y: Math.min(...ys),
        w: Math.max(...xs) - Math.min(...xs),
        h: Math.max(...ys) - Math.min(...ys),
      };
    }
    mode = next;
    error = "";
    fit();
  }
  function moveCorner(index: number, p: Point, start: Point[]) {
    if (mode === "quad") {
      quad = quad.map((old, i) =>
        i === index ? { x: clamp(p.x), y: clamp(p.y) } : old,
      );
      return;
    }
    const opposite = start[(index + 2) % 4],
      left = index === 0 || index === 3,
      top = index === 0 || index === 1;
    let w = Math.max(0.01, left ? opposite.x - p.x : p.x - opposite.x),
      h = Math.max(0.01, top ? opposite.y - p.y : p.y - opposite.y);
    const mw = left ? opposite.x : 1 - opposite.x,
      mh = top ? opposite.y : 1 - opposite.y;
    w = Math.min(w, mw);
    h = Math.min(h, mh);
    if (ratio) {
      const r = ratio / aspect;
      w = Math.min(w, h * r, mw, mh * r);
      h = w / r;
    }
    if (w >= 0.01 && h >= 0.01)
      region = {
        x: left ? opposite.x - w : opposite.x,
        y: top ? opposite.y - h : opposite.y,
        w,
        h,
      };
  }
  function drag(e: PointerEvent, index: number | null) {
    if (busy || e.button !== 0) return;
    stopDrag?.();
    e.preventDefault();
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const bounds = box.getBoundingClientRect(),
      start = corners.map((p) => ({ ...p })),
      r = { ...region },
      x = e.clientX,
      y = e.clientY;
    if (index !== null) showLens(e, index, bounds);
    const move = (v: PointerEvent) => {
      const dx = (v.clientX - x) / bounds.width,
        dy = (v.clientY - y) / bounds.height;
      if (index === null)
        region = {
          ...r,
          x: Math.max(0, Math.min(1 - r.w, r.x + dx)),
          y: Math.max(0, Math.min(1 - r.h, r.y + dy)),
        };
      else
        moveCorner(
          index,
          { x: start[index].x + dx, y: start[index].y + dy },
          start,
        );
      if (index !== null) showLens(v, index, bounds);
    };
    const end = () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", end);
      el.removeEventListener("pointercancel", end);
      el.removeEventListener("lostpointercapture", end);
      if (el.hasPointerCapture(e.pointerId))
        el.releasePointerCapture(e.pointerId);
      lens = null;
      stopDrag = undefined;
    };
    stopDrag = end;
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", end);
    el.addEventListener("pointercancel", end);
    el.addEventListener("lostpointercapture", end);
  }
  function rectKeys(e: KeyboardEvent) {
    if (!e.key.startsWith("Arrow")) return;
    e.preventDefault();
    e.stopPropagation();
    region = {
      ...region,
      x: Math.max(
        0,
        Math.min(
          1 - region.w,
          region.x +
            (e.key === "ArrowRight"
              ? 0.005
              : e.key === "ArrowLeft"
                ? -0.005
                : 0),
        ),
      ),
      y: Math.max(
        0,
        Math.min(
          1 - region.h,
          region.y +
            (e.key === "ArrowDown" ? 0.005 : e.key === "ArrowUp" ? -0.005 : 0),
        ),
      ),
    };
  }
  async function suggest() {
    busy = true;
    error = "";
    try {
      region = await invoke<Crop>("crop_suggest", { assetId: asset.id });
      mode = "rect";
      shape = "free";
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  async function render() {
    if (!valid) return;
    busy = true;
    error = "";
    try {
      if (!desktop) throw Error(tr("四角校正预览请使用桌面版"));
      const bytes = await invoke<number[]>("crop_preview", {
        assetId: asset.id,
        crop,
      });
      if (preview) URL.revokeObjectURL(preview);
      previewKey = JSON.stringify(crop);
      preview = URL.createObjectURL(
        new Blob([new Uint8Array(bytes)], { type: "image/jpeg" }),
      );
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  async function save(reset = false) {
    if (!reset && !valid) return;
    busy = true;
    error = "";
    try {
      if (desktop)
        await catalogCommand("asset_crop", {
          assetId: asset.id,
          crop: reset ? null : crop,
        });
      else if (mode === "quad" && !reset)
        throw Error(tr("四角校正请使用桌面版"));
      else asset.crop = reset ? null : { ...crop };
      notify(
        reset
          ? sourceMessage("已恢复完整影像")
          : sourceMessage("裁切与校正已保存，原件未修改"),
      );
      dialog.close();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  class="modal bg-scrim/30 backdrop-blur-xl"
  aria-label={tr("裁切与透视校正")}
  onclose={() => {
    stopDrag?.();
    onclose();
  }}
  oncancel={(e) => {
    e.stopPropagation();
    if (busy) e.preventDefault();
  }}
>
  <div
    class="modal-box glass-panel w-[min(1080px,95vw)] max-w-none rounded-3xl p-6 text-base-content"
  >
    <header class="mb-4 flex items-center gap-3">
      <div class="flex-1">
        <h2>{tr("裁切与透视校正")}</h2>
        <p class="mt-1 text-xs text-ink/50">
          {tr("{0} · 原件不变", [
            mode === "rect"
              ? tr("拖动选区移动，拖动四角调整边缘")
              : tr("依次对准拍立得的左上、右上、右下、左下角，再预览校正结果"),
          ])}
        </p>
      </div>
      <button
        class="btn btn-ghost btn-sm btn-circle"
        aria-label={tr("关闭裁切")}
        disabled={busy}
        onclick={() => dialog.close()}><X size={18} /></button
      >
    </header>
    <div class="mb-4 flex flex-wrap items-center gap-2">
      <button
        class={`btn btn-sm rounded-full ${mode === "rect" ? "bg-tint/50" : "btn-ghost"}`}
        aria-pressed={mode === "rect"}
        disabled={busy}
        onclick={() => switchMode("rect")}
        ><CropIcon size={14} />{tr("裁切")}</button
      ><button
        class={`btn btn-sm rounded-full ${mode === "quad" ? "bg-tint/50" : "btn-ghost"}`}
        aria-pressed={mode === "quad"}
        disabled={busy}
        onclick={() => switchMode("quad")}
        ><Scan size={14} />{tr("四角校正")}</button
      >
      <div class="ml-auto w-52">
        <SelectMenu
          label={tr("输出比例")}
          bind:value={shape}
          disabled={busy}
          onchange={fit}
          options={[
            {
              value: "free",
              label:
                mode === "quad" ? tr("自由 · 根据边长估算") : tr("自由比例"),
            },
            { value: "54/86", label: "Instax Mini · 54 × 86" },
            { value: "72/86", label: "Instax Square · 72 × 86" },
            { value: "108/86", label: "Instax Wide · 108 × 86" },
            { value: "86/54", label: tr("Mini 横向") },
            { value: "86/72", label: tr("Square 横向") },
            { value: "86/108", label: tr("Wide 竖向") },
            { value: "1/1", label: tr("正方形 · 1:1") },
          ]}
        />
      </div>
    </div>
    <div class="flex min-h-0 items-center justify-center gap-6 py-4">
      <div
        bind:this={box}
        class="relative shrink-0 select-none"
        style:width={`min(${preview ? "48%" : "95%"},${aspect * 380}px)`}
      >
        <img
          src={asset.baseSrc || asset.src}
          alt={tr("完整影像，拖动四角调整")}
          draggable="false"
          class="block w-full"
          onload={(e) =>
            (aspect =
              (e.currentTarget as HTMLImageElement).naturalWidth /
              (e.currentTarget as HTMLImageElement).naturalHeight)}
        />
        <svg
          class="pointer-events-none absolute inset-0 h-full w-full"
          viewBox="0 0 100 100"
          preserveAspectRatio="none"
          aria-hidden="true"
          ><path
            d={`M0 0H100V100H0Z M${outline.replaceAll(" ", " L")}Z`}
            fill="#0008"
            fill-rule="evenodd"
          /><polygon
            points={outline}
            fill="none"
            stroke={valid ? "#fff" : "#f88"}
            stroke-width="1.5"
            vector-effect="non-scaling-stroke"
          /></svg
        >
        {#if mode === "rect"}<button
            class="absolute touch-none"
            style:left={`${region.x * 100}%`}
            style:top={`${region.y * 100}%`}
            style:width={`${region.w * 100}%`}
            style:height={`${region.h * 100}%`}
            aria-label={tr("移动选区")}
            disabled={busy}
            onpointerdown={(e) => drag(e, null)}
            onkeydown={rectKeys}
          ></button>{/if}
        {#each corners as p, i}<button
            class="absolute grid size-6 -translate-x-1/2 -translate-y-1/2 touch-none place-items-center rounded-full bg-white text-[10px] text-black shadow-md ring-2 ring-black/20"
            style:left={`${p.x * 100}%`}
            style:top={`${p.y * 100}%`}
            aria-label={tr("调整{0}角", [
              tr(["左上", "右上", "右下", "左下"][i]),
            ])}
            disabled={busy}
            onpointerdown={(e) => drag(e, i)}
            onkeydown={(e) => {
              if (e.key.startsWith("Arrow")) {
                e.preventDefault();
                e.stopPropagation();
                const delta = e.shiftKey ? 0.02 : 0.005;
                moveCorner(
                  i,
                  {
                    x:
                      p.x +
                      (e.key === "ArrowRight"
                        ? delta
                        : e.key === "ArrowLeft"
                          ? -delta
                          : 0),
                    y:
                      p.y +
                      (e.key === "ArrowDown"
                        ? delta
                        : e.key === "ArrowUp"
                          ? -delta
                          : 0),
                  },
                  corners,
                );
              }
            }}>{i + 1}</button
          >{/each}
      </div>
      {#if preview}<figure
          class="flex min-w-0 max-w-[48%] flex-1 flex-col items-center gap-3"
        >
          <img
            src={preview}
            alt={tr("校正结果预览")}
            class="max-h-96 max-w-full object-contain"
          />
          <figcaption class="text-xs text-ink/45">
            {previewStale ? tr("选区已改变，点击预览更新") : tr("结果预览")}
          </figcaption>
        </figure>{/if}
    </div>
    {#if !valid}<p class="mt-3 text-xs text-error">
        {tr("四角不能交叉、重叠或形成凹角。")}
      </p>{/if}{#if error}<p class="mt-3 text-xs text-error" role="alert">
        {message(error)}
      </p>{/if}
    <footer class="mt-5 flex flex-wrap items-center gap-2">
      <button
        class="btn btn-ghost btn-sm rounded-full"
        disabled={!desktop || busy}
        onclick={suggest}><ScanLine size={14} />{tr("边界建议")}</button
      ><button
        class="btn btn-ghost btn-sm rounded-full"
        disabled={busy}
        onclick={() => save(true)}
        ><RotateCcw size={14} />{tr("恢复完整影像")}</button
      ><button
        class="btn btn-ghost btn-sm ml-auto rounded-full"
        disabled={busy || !valid}
        onclick={render}><Eye size={14} />{tr("预览结果")}</button
      ><button
        class="btn glass-dark btn-sm rounded-full text-white"
        disabled={busy || !valid}
        onclick={() => save()}
        >{#if busy}<span class="loading loading-spinner loading-xs"
          ></span>{:else}<Check size={14} />{/if}{tr("保存")}</button
      >
    </footer>
  </div>
  {#if lens}
    <div
      class="pointer-events-none fixed z-50 overflow-hidden rounded-xl bg-neutral shadow-2xl"
      style:left={`${lens.left}px`}
      style:top={`${lens.top}px`}
      style:width={`${lensSize}px`}
      style:height={`${lensSize}px`}
      aria-hidden="true"
    >
      <img
        src={asset.baseSrc || asset.src}
        alt=""
        draggable="false"
        class="absolute max-w-none"
        style:width={`${lens.width}px`}
        style:height={`${lens.height}px`}
        style:left={`${lensSize / 2 - corners[lens.index].x * lens.width}px`}
        style:top={`${lensSize / 2 - corners[lens.index].y * lens.height}px`}
      />
      <svg class="absolute inset-0 h-full w-full" viewBox="0 0 144 144">
        <path
          d="M72 52V66M72 78V92M52 72H66M78 72H92"
          fill="none"
          stroke="#0009"
          stroke-width="3"
        />
        <path
          d="M72 52V66M72 78V92M52 72H66M78 72H92"
          fill="none"
          stroke="white"
          stroke-width="1"
        />
        <circle cx="72" cy="72" r="2" fill="white" stroke="#0009" />
      </svg>
      <span
        class="absolute bottom-2 right-2 rounded-full bg-black/45 px-2 py-0.5 text-[10px] text-white backdrop-blur-sm"
        >{lensZoom}×</span
      >
    </div>
  {/if}
</dialog>
