<script lang="ts">
  import '../app.css';
  import { page } from '$app/state';
  import { ChartNoAxesColumn, Images, UsersRound } from '@lucide/svelte';
  let { children } = $props();
  const destinations = [
    { href: '/stats', label: '统计', icon: ChartNoAxesColumn },
    { href: '/', label: '相册', icon: Images },
    { href: '/people', label: '人物', icon: UsersRound }
  ];
</script>
<nav aria-label="主导航" class="glass-light fixed left-1/2 top-5 z-40 flex -translate-x-1/2 items-center gap-1 rounded-full p-1.5 text-[#3c4037]">
  {#each destinations as destination}
    <a href={destination.href} aria-current={page.url.pathname === destination.href ? 'page' : undefined} class={`btn btn-ghost btn-sm h-9 gap-2 rounded-full px-5 text-xs font-normal transition-colors ${page.url.pathname === destination.href ? 'bg-white/55 text-[#404d35] shadow-sm' : 'text-black/50 hover:bg-white/30'}`}><destination.icon size={15} strokeWidth={1.6}/>{destination.label}</a>
  {/each}
</nav>
{@render children()}
