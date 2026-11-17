<script>
  import { onMount } from 'svelte';
  import { app, refresh, covers, loadCovers, syncCovers, reloadLastPlayed, handleMenu } from './lib/store.svelte.js';
  import { api } from './lib/api.js';
  import Sidebar from './components/Sidebar.svelte';
  import Library from './components/Library.svelte';
  import Cores from './components/Cores.svelte';
  import Controls from './components/Controls.svelte';
  import Settings from './components/Settings.svelte';

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
    return () => off.forEach((f) => f());
  });
</script>

<div class="shell">
  <Sidebar />
  <main>
    {#if !app.loaded}
      <div class="loading">Loading…</div>
    {:else if app.view === 'library'}
      <Library />
    {:else if app.view === 'cores'}
      <Cores />
    {:else if app.view === 'settings'}
      <Settings />
    {:else}
      <Controls />
    {/if}
  </main>
  {#if app.toast}
    <div class="toast {app.toast.kind}" role="status">{app.toast.text}</div>
  {/if}
</div>

<style>
  .shell { display: grid; grid-template-columns: 200px 1fr; height: 100%; }
  main { min-width: 0; overflow: hidden; display: flex; }
  .loading { margin: auto; color: var(--muted); }
  .toast {
    position: fixed; left: 50%; bottom: 24px; transform: translateX(-50%);
    background: var(--surface-2); border: 1px solid var(--border); border-left: 3px solid var(--accent);
    padding: 10px 16px; border-radius: 8px; max-width: 70vw; white-space: pre-wrap;
    box-shadow: 0 8px 30px rgba(0,0,0,.5); z-index: 10; user-select: text;
  }
  .toast.error { border-left-color: var(--bad); }
</style>
