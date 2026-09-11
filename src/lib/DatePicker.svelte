<script lang="ts">
  import { onMount } from "svelte";
  import { CalendarDays } from "@lucide/svelte";
  import Floating from "./Floating.svelte";
  let {
    value = $bindable(""),
    label = "日期",
    onselect,
  }: {
    value?: string;
    label?: string;
    onselect?: (date: string) => void;
  } = $props();
  let pop: Floating;
  let ready = $state(false);
  let typed = $state("");
  let invalid = $state(false);
  onMount(async () => {
    await import("cally");
    ready = true;
  });
  function connectCalendar(node: HTMLElement) {
    const change = async () => {
      const calendar = node as HTMLElement & {
        value: string;
        updated: Promise<void>;
      };
      await calendar.updated;
      choose(calendar.value);
      if (onselect) calendar.value = "";
    };
    node.addEventListener("change", change);
    return {
      destroy() {
        node.removeEventListener("change", change);
      },
    };
  }
  function choose(date: string) {
    pop.close();
    if (onselect) {
      typed = "";
      onselect(date);
    } else value = date;
  }
  function apply() {
    const d = new Date(typed + "T00:00:00Z");
    if (
      typed &&
      (!/^\d{4}-\d{2}-\d{2}$/.test(typed) ||
        !Number.isFinite(d.getTime()) ||
        d.toISOString().slice(0, 10) !== typed)
    ) {
      invalid = true;
      return;
    }
    invalid = false;
    choose(typed);
  }
</script>

<Floating bind:this={pop} {label} wide>
  {#snippet trigger()}<CalendarDays size={14} /><span class="flex-1 text-left"
      >{onselect ? label : value || "选择日期"}</span
    >{/snippet}
  {#if ready}<calendar-date
      class="cally w-full bg-transparent"
      {value}
      locale="zh-CN"
      first-day-of-week="1"
      use:connectCalendar
      ><span slot="previous">‹</span><span slot="next">›</span><calendar-month
      ></calendar-month></calendar-date
    >{/if}
  <div class="flex gap-2 border-t border-black/5 p-2">
    <input
      class="input input-sm min-w-0 flex-1 border-0 bg-white/30 text-xs"
      aria-label="直接输入日期"
      placeholder="YYYY-MM-DD"
      bind:value={typed}
      onfocus={() => (typed = value)}
      onkeydown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          apply();
        }
      }}
    /><button class="btn btn-ghost btn-sm" onclick={apply}
      >{onselect ? "跳转" : "确定"}</button
    >{#if !onselect}<button
        class="btn btn-ghost btn-sm"
        onclick={() => {
          value = "";
          pop.close();
        }}>清除</button
      >{/if}
  </div>
  {#if invalid}<p class="px-3 text-xs text-error">
      请输入有效日期，例如 2026-08-27
    </p>{/if}
</Floating>
