<script lang="ts">
  import {
    sourceMessage,
    message,
    tr,
    language,
    languages,
    setLanguage,
    type Locale,
  } from "$lib/i18n.svelte";
  import { X, FolderPlus, HardDrive, RefreshCw } from "@lucide/svelte";
  import {
    librarySession,
    desktop,
    addLocation,
    importDesktop,
    loadLibrary,
    catalogCommand,
  } from "./session.svelte";
  import { notify } from "./tasks.svelte";
  import {
    appearance,
    setAppearance,
    setImageFit,
    type Appearance,
  } from "./appearance.svelte";
  import SelectMenu from "./SelectMenu.svelte";
  import TaskCenter from "./TaskCenter.svelte";
  let { onclose }: { onclose: () => void } = $props();
  let dialog: HTMLDialogElement;
  let working = $state(false);
  let removing = $state<string | null>(null);
  function locationCount(id: string) {
    return new Set(
      librarySession.photos
        .flatMap((c) => c.assets)
        .filter((a) =>
          a.renditions.some(
            (r) => r.role === "original" && r.locationId === id,
          ),
        )
        .map((a) => a.id),
    ).size;
  }
  async function removeLocation(id: string) {
    if (working || librarySession.busy) return;
    working = true;
    try {
      await catalogCommand("location_remove", { locationId: id });
      removing = null;
      notify(sourceMessage("已移除目录及相关影像记录和缓存，原始照片保留"));
    } catch {
    } finally {
      working = false;
    }
  }
  let confirmation = $state("");
  let clearStep = $state(0);
  $effect(() => {
    confirmation;
    language.current;
    clearStep = 0;
  });
  let localCount = $derived(
    new Set(
      librarySession.photos
        .flatMap((c) => c.assets)
        .filter((a) =>
          a.renditions.some(
            (r) => r.role === "original" && r.locationId === "local",
          ),
        )
        .map((a) => a.id),
    ).size,
  );
  async function clearLocal() {
    if (confirmation !== tr("清空本机仓库") || working || librarySession.busy)
      return;
    if (!clearStep) {
      clearStep = 1;
      return;
    }
    working = true;
    try {
      await catalogCommand("library_clear_local", {
        confirmation: "清空本机仓库",
      });
      confirmation = "";
      clearStep = 0;
      notify(
        sourceMessage("本机仓库已清空，原件已移入系统回收站；外置影像保留"),
      );
    } catch {
    } finally {
      working = false;
    }
  }
  $effect(() => {
    dialog?.showModal();
  });
  async function add(id: string | null = null) {
    working = true;
    try {
      await addLocation(id);
    } catch (e) {
      notify(String(e), "error");
    } finally {
      working = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  {onclose}
  class="modal bg-scrim/20 backdrop-blur-xl"
  aria-label={tr("图库设置")}
>
  <div
    class="modal-box glass-panel w-[min(680px,90vw)] max-w-none rounded-3xl p-7 text-base-content"
  >
    <header class="mb-6 flex items-center justify-between">
      <div>
        <h2 class="text-lg">{tr("图库设置")}</h2>
        <p class="mt-1 text-xs text-ink/45">
          {tr("原件可以远行，收藏留在这里。")}
        </p>
      </div>
      <button
        class="btn btn-ghost btn-circle btn-sm"
        aria-label={tr("关闭设置")}
        onclick={() => dialog.close()}><X size={18} /></button
      >
    </header>
    <section
      class="mb-6 flex flex-wrap items-center justify-between gap-4 rounded-2xl bg-surface/25 p-4"
      aria-label={tr("语言设置")}
    >
      <div>
        <h3 class="text-sm">{tr("语言")}</h3>
        <p class="mt-1 text-xs text-ink/45">
          {tr("立即应用，并记住此设备的选择")}
        </p>
      </div>
      <div class="w-44">
        <SelectMenu
          label={tr("界面语言")}
          value={language.current}
          options={languages}
          onchange={(value) => setLanguage(value as Locale)}
        />
      </div>
    </section>

    <section
      class="mb-6 flex flex-wrap items-center justify-between gap-4 rounded-2xl bg-surface/25 p-4"
      aria-label={tr("外观设置")}
    >
      <div>
        <h2 class="text-sm">{tr("外观")}</h2>
        <p class="mt-1 text-xs text-ink/45">{tr("选择浅色、深色或跟随系统")}</p>
      </div>
      <div class="w-40 shrink-0">
        <SelectMenu
          label={tr("外观模式")}
          value={appearance.mode}
          onchange={(v) => setAppearance(v as Appearance)}
          options={[
            { value: "system", label: tr("跟随系统") },
            { value: "light", label: tr("浅色") },
            { value: "dark", label: tr("深色") },
          ]}
        />
      </div>
    </section>
    <section
      class="mb-6 flex flex-wrap items-center justify-between gap-4 rounded-2xl bg-surface/25 p-4"
    >
      <div>
        <h2 class="text-sm">{tr("相册图片展示")}</h2>
        <p class="mt-1 text-xs text-ink/45">
          {tr("完整显示保留边缘；铺满会隐藏部分边缘")}
        </p>
      </div>
      <div class="w-40 shrink-0">
        <SelectMenu
          label={tr("图片展示方式")}
          value={appearance.fit}
          onchange={(v) => setImageFit(v as "cover" | "contain")}
          options={[
            { value: "contain", label: tr("完整显示") },
            { value: "cover", label: tr("铺满") },
          ]}
        />
      </div>
    </section>
    <p class="mb-4 text-xs leading-6 text-ink/55">
      {tr(
        "本机保存数据库和浏览图。添加外部目录后，导入时可选择引用原件；目录离线不影响已有缓存和收藏资料。原件保持在原文件夹内，文件名随收藏日期和人物／团体更新。离线操作失败后不会排队重试。",
      )}
    </p>
    {#each librarySession.locations as location}<div
        class="mb-3 rounded-2xl bg-surface/25 p-4"
      >
        <div class="flex items-center gap-2 text-sm">
          <HardDrive size={16} />{location.id === "local"
            ? tr("本机图库与离线缓存")
            : location.name}<span class="ml-auto text-[10px] text-ink/40"
            >{location.online ? tr("已连接") : tr("离线")}</span
          >
        </div>
        <p class="mt-2 break-all text-[11px] text-ink/40">{location.path}</p>
        {#if location.id !== "local"}<div class="mt-3 flex gap-2">
            <button
              class="btn btn-ghost btn-xs"
              disabled={!location.online || librarySession.busy}
              onclick={() =>
                importDesktop(null, { reference: true }, location.id).catch(
                  () => {},
                )}>{tr("扫描目录并登记照片")}</button
            ><button
              class="btn btn-ghost btn-xs"
              disabled={working || librarySession.busy}
              onclick={() => add(location.id)}>{tr("重新定位目录")}</button
            ><button
              class="btn btn-ghost btn-xs text-error"
              disabled={working || librarySession.busy}
              onclick={() =>
                (removing = removing === location.id ? null : location.id)}
              >{tr("移除目录…")}</button
            >
          </div>
          {#if removing === location.id}<div
              class="mt-3 space-y-3 rounded-xl bg-error/12 p-4"
            >
              <p class="text-xs leading-6 text-danger-ink">
                {tr(
                  "将从图库移除该目录的 {0} 份影像，包括回收站中的相关记录和本机浏览缓存。空收藏也会移除。",
                  [locationCount(location.id)],
                )}<strong>{tr("原始照片完整保留，不移入系统回收站。")}</strong
                >{tr("目录离线时也可操作。")}
              </p>
              <button
                class="btn btn-error btn-sm rounded-full"
                disabled={working || librarySession.busy}
                onclick={() => removeLocation(location.id)}
                >{tr("确认移除，保留原片")}</button
              >
              <button
                class="btn btn-ghost btn-sm"
                disabled={working}
                onclick={() => (removing = null)}>{tr("取消")}</button
              >
            </div>{/if}{/if}
      </div>{/each}
    {#if !desktop}<p class="rounded-xl bg-surface/30 p-4 text-xs">
        {tr("浏览器为交互示例；文件夹设置请使用桌面版。")}
      </p>{/if}
    <div class="mt-5 flex flex-wrap gap-2">
      <button
        class="btn btn-sm glass-dark rounded-full text-white"
        disabled={!desktop || working || librarySession.busy}
        onclick={() => add()}
        ><FolderPlus size={15} />{tr("添加原件目录")}</button
      ><button
        class="btn btn-ghost btn-sm rounded-full"
        disabled={!desktop || librarySession.busy}
        onclick={async () => {
          await loadLibrary(true);
        }}><RefreshCw size={14} />{tr("刷新连接状态")}</button
      >
    </div>
    <p class="mt-5 text-[10px] text-ink/40">
      {tr(
        "扫描包含子文件夹，跳过符号链接。已登记文件不会重复导入；处理进度在左下角。",
      )}
    </p>
    <section
      class="mt-7 space-y-3 rounded-2xl bg-error/12 p-5 shadow-sm"
      aria-label={tr("清空本机仓库")}
    >
      <h2 class="text-sm font-semibold text-danger-ink">
        {tr("清空本机仓库")}
      </h2>
      <p class="text-xs leading-6 text-danger-ink">
        {tr(
          "将 {0} 份本机原件移入系统回收站，并删除对应影像和浏览缓存。没有其他影像的收藏也会移除。外置原件、收藏资料及其离线缓存保留。",
          [localCount],
        )}
      </p>
      <label class="block text-xs text-danger-ink"
        >{tr("输入「清空本机仓库」后，还需点击两次确认。")}<input
          class="input mt-2 w-full border-0 bg-surface/50 text-sm shadow-sm"
          aria-label={tr("清空确认文字")}
          bind:value={confirmation}
          disabled={working}
          placeholder={tr("清空本机仓库")}
          autocomplete="off"
        />
      </label>
      {#if clearStep}<p
          role="alert"
          class="text-xs font-semibold text-danger-ink"
        >
          {tr("最后确认：下一次点击将立即清空本机仓库。")}
        </p>{/if}
      <button
        class="btn btn-error btn-sm rounded-full"
        disabled={!desktop ||
          confirmation !== tr("清空本机仓库") ||
          working ||
          librarySession.busy ||
          !localCount}
        onclick={clearLocal}
        >{working
          ? tr("正在清空…")
          : clearStep
            ? tr("第二次确认：立即清空")
            : tr("第一次确认：我已了解范围")}</button
      >
      {#if clearStep}<button
          class="btn btn-ghost btn-sm"
          disabled={working}
          onclick={() => {
            confirmation = "";
            clearStep = 0;
          }}>{tr("取消")}</button
        >{/if}
    </section>
  </div>
  <TaskCenter />
</dialog>
