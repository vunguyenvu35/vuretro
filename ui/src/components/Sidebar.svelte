<script>
  import { app, pickRomFiles, pickRomFolder, linkFolder, unlinkFolder } from '../lib/store.svelte.js';
  import { colorOf } from '../lib/systems.js';
  import logo from '../lib/logo-mark.svg';

  const total = $derived(app.overview?.games.length ?? 0);
  const recent = $derived(app.overview?.games.filter((g) => g.last_played).length ?? 0);
  const favs = $derived(app.overview?.games.filter((g) => g.favorite).length ?? 0);

  function go(filter, system = null) {
    app.view = 'library';
    app.filter = filter;
    app.system = system;
  }
  const libActive = (f) => app.view === 'library' && !app.system && app.filter === f;
</script>

<aside>
  <div class="brand">
    <img src={logo} alt="" width="28" height="28" />
    <span><b>Vu</b>retro</span>
  </div>

  <nav>
    <div class="label">Library</div>
    <button class:active={libActive('all')} onclick={() => go('all')}>All <small>{total}</small></button>
    <button class:active={libActive('recent')} onclick={() => go('recent')}>Recent <small>{recent}</small></button>
    <button class:active={libActive('favorites')} onclick={() => go('favorites')}>Favorites <small>{favs}</small></button>

    <div class="label gap">Systems</div>
    {#each app.overview?.systems ?? [] as s (s.id)}
      <button
        class:active={app.view === 'library' && app.system === s.id}
        class:off={!s.enabled}
        onclick={() => go('all', s.id)}
      >
        <i style="background:{colorOf(s.id)}"></i>{s.name} <small>{s.game_count}</small>
      </button>
    {/each}

    <div class="label gap">App</div>
    <button class:active={app.view === 'cores'} onclick={() => (app.view = 'cores')}>Cores &amp; BIOS</button>
    <button class:active={app.view === 'controls'} onclick={() => (app.view = 'controls')}>Controls</button>
    <button class:active={app.view === 'macros'} onclick={() => (app.view = 'macros')}>Macros</button>
    <button class:active={app.view === 'settings'} onclick={() => (app.view = 'settings')}>Settings</button>
  </nav>

  <div class="folder">
    <div class="label">Add games</div>
    <button class="btn primary add" onclick={pickRomFiles} disabled={app.busy}>
      <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round" /></svg>
      Import ROMs
    </button>
    <button class="btn add ghost" onclick={pickRomFolder} disabled={app.busy}>Import a folder</button>
    {#if app.system}<p class="hint">Archives (.zip) go to {app.overview?.systems.find((s) => s.id === app.system)?.name}.</p>{/if}
    {#if app.overview?.rom_dir}
      <div class="linked">
        <div class="lhead">
          <svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true"><path d="M2 4.5A1.5 1.5 0 013.5 3H6l1.5 1.5h5A1.5 1.5 0 0114 6v5.5a1.5 1.5 0 01-1.5 1.5h-9A1.5 1.5 0 012 11.5v-7z" fill="none" stroke="currentColor" stroke-width="1.4" /></svg>
          <span>Linked folder</span>
        </div>
        <div class="path mono" title={app.overview.rom_dir}>{app.overview.rom_dir}</div>
        <div class="acts">
          <button onclick={linkFolder}>Change</button>
          <button onclick={unlinkFolder}>Remove</button>
        </div>
      </div>
    {/if}
  </div>
</aside>

<style>
  aside {
    background: var(--surface); border-right: 1px solid var(--border);
    display: flex; flex-direction: column; padding: 14px 10px; gap: 8px; min-height: 0;
  }
  .brand { display: flex; align-items: center; gap: 10px; padding: 2px 8px 14px; font: 700 18px var(--font-head); }
  .brand b { color: var(--brand-magenta); font-weight: 700; }
  nav { display: flex; flex-direction: column; gap: 2px; overflow: auto; flex: 1; min-height: 0; }
  .label { padding: 0 8px; margin: 4px 0; }
  .gap { margin-top: 16px; }
  nav button {
    display: flex; align-items: center; gap: 8px; text-align: left; background: none; border: 0;
    border-left: 3px solid transparent; padding: 6px 8px; border-radius: 0 6px 6px 0; color: var(--muted);
  }
  nav button:hover { background: var(--surface-2); color: var(--text); }
  nav button.active { color: var(--accent); background: var(--surface-2); border-left-color: var(--accent); }
  nav button.off { opacity: .45; }
  small { margin-left: auto; color: var(--faint); }
  i { width: 8px; height: 8px; border-radius: 50%; flex: none; }
  .folder { padding: 10px 8px 0; border-top: 1px solid var(--border); }
  .folder .label { padding: 0; }
  .add { display: flex; align-items: center; justify-content: center; gap: 7px; width: 100%; margin-top: 8px; padding: 8px 10px; font-size: 13px; }
  .add.ghost { background: none; border: 1px solid var(--border); color: var(--muted); }
  .add.ghost:hover { color: var(--text); border-color: var(--faint); }
  .hint { color: var(--faint); font-size: 11px; margin: 8px 2px 0; }
  .linked { margin-top: 14px; padding: 10px 10px 8px; background: var(--surface-2); border: 1px solid var(--border); border-radius: 10px; }
  .lhead { display: flex; align-items: center; gap: 6px; color: var(--muted); font-size: 11px; text-transform: uppercase; letter-spacing: .07em; }
  .path { display: block; margin: 6px 0 4px; font-size: 11px; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .acts { display: flex; gap: 14px; }
  .acts button { background: none; border: 0; padding: 2px 0; color: var(--muted); font-size: 12px; }
  .acts button:hover { color: var(--accent); }
</style>
