<script>
  import { onMount } from 'svelte';
  import { app, refresh, covers, loadCovers, syncCovers, reloadLastPlayed, handleMenu } from './lib/store.svelte.js';
  import { api } from './lib/api.js';
  import Sidebar from './components/Sidebar.svelte';
  import Library from './components/Library.svelte';
  import Cores from './components/Cores.svelte';
  import Controls from './components/Controls.svelte';
  import Settings from './components/Settings.svelte';
  import Macros from './components/Macros.svelte';
  import ContextMenu from './components/ContextMenu.svelte';
  import ConfirmDialog from './components/ConfirmDialog.svelte';
  import Toasts from './components/Toasts.svelte';
  import { startGamepadNav } from './lib/gamepadNav.js';

  onMount(() => {
    refresh();
    let off = [];
    api.onGameExited(async () => {
      await refresh();
      reloadLastPlayed();
    }).then((f) => off.push(f));
    api.onCoversUpdated((path) => loadCovers([path])).then((f) => off.push(f));
    api.onMenu(handleMenu).then((f) => off.push(f));
    api.onCoversDone(() => syncCovers()).then((f) => off.push(f));
    // a pad on the Controls screen is being recorded, not used to move around
    const stopNav = startGamepadNav(() => app.capturing);
    return () => {
      stopNav();
      off.forEach((f) => f());
    };
  });
</script>

<!-- No browser menu (Reload, Inspect…) anywhere: this is an app. Text fields keep the
     system's Cut / Copy / Paste menu. Screens that have their own menu stop the event. -->
<svelte:window oncontextmenu={(e) => { if (!(e.target instanceof HTMLElement && e.target.closest('input, textarea'))) e.preventDefault(); }} />

<div class="shell">
  <Sidebar />
  <main>
    {#if !app.loaded}
      <div class="loading">Loading…</div>
    {:else if app.view === 'library'}
      <Library />
    {:else if app.view === 'cores'}
      <Cores />
    {:else if app.view === 'macros'}
      <Macros />
    {:else if app.view === 'settings'}
      <Settings />
    {:else}
      <Controls />
    {/if}
  </main>
  <ContextMenu />
  <ConfirmDialog />
  <Toasts />
</div>

<style>
  .shell { display: grid; grid-template-columns: 224px 1fr; height: 100%; }
  main { min-width: 0; overflow: hidden; display: flex; }
  .loading { margin: auto; color: var(--muted); }
</style>
