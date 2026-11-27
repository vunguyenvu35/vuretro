<script>
  import { onMount } from 'svelte';
  import { app, refresh, covers, loadCovers, scheduleCovers, handleMenu, installLine } from './lib/store.svelte.js';
  import { api } from './lib/api.js';
  import { checkAtStart } from './lib/update.svelte.js';
  import Sidebar from './components/Sidebar.svelte';
  import Library from './components/Library.svelte';
  import NativeGames from './components/NativeGames.svelte';
  import Cores from './components/Cores.svelte';
  import Controls from './components/Controls.svelte';
  import Settings from './components/Settings.svelte';
  import Macros from './components/Macros.svelte';
  import ContextMenu from './components/ContextMenu.svelte';
  import ConfirmDialog from './components/ConfirmDialog.svelte';
  import Toasts from './components/Toasts.svelte';
  import DownloadBanner from './components/DownloadBanner.svelte';
  import PlayingBanner from './components/PlayingBanner.svelte';
  import AboutDialog from './components/AboutDialog.svelte';
  import SupportDialog from './components/SupportDialog.svelte';
  import { startGamepadNav } from './lib/gamepadNav.js';

  onMount(() => {
    refresh();
    checkAtStart();
    let off = [];
    // closing a game no longer replaces its picture: it keeps the one it had
    api.onGameExited(() => {
      app.playing = null;
      app.playingNote = '';
      refresh();
    }).then((f) => off.push(f));
    api.onPcPreparing((line) => (app.preparing = line)).then((f) => off.push(f));
    api.onPcPrepared(() => (app.preparing = null)).then((f) => off.push(f));
    api.onCoversUpdated((path) => loadCovers([path])).then((f) => off.push(f));
    api.onMenu(handleMenu).then((f) => off.push(f));
    // Play fetches the core a game needs: show how far it is
    api.onCoreDownloading((core) => (app.downloading = { core, percent: 0 })).then((f) => off.push(f));
    api
      .onCoreProgress((p) => {
        if (app.downloading?.core === p.core) app.downloading.percent = p.percent;
      })
      .then((f) => off.push(f));
    api.onCoversDone(() => scheduleCovers()).then((f) => off.push(f));
    // the lines of the build while Road Rash is being set up
    api.onNativeInstallLog(installLine).then((f) => off.push(f));
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

<div class="shell" style="grid-template-columns: {app.sidebarW}px 1fr">
  <Sidebar />
  <main>
    {#if !app.loaded}
      <div class="loading">Loading…</div>
    {:else if app.view === 'library'}
      <Library />
    {:else if app.view === 'native'}
      <NativeGames />
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
  {#if app.dialog === 'about'}
    <AboutDialog onclose={() => (app.dialog = null)} />
  {:else if app.dialog === 'support'}
    <SupportDialog onclose={() => (app.dialog = null)} />
  {/if}
  <Toasts />
  <DownloadBanner />
  <PlayingBanner />
</div>

<style>
  .shell { display: grid; height: 100%; }
  main { min-width: 0; overflow: hidden; display: flex; }
  .loading { margin: auto; color: var(--muted); }
</style>
