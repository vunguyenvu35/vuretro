<script>
  import { app, chooseRomFolder } from '../lib/store.svelte.js';
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
  </nav>

  <div class="folder">
    <div class="label">ROM folder</div>
    <button class="path mono" onclick={chooseRomFolder} title="Change ROM folder">
      {app.overview?.rom_dir ?? 'Choose folder…'}
    </button>
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
  .path {
    display: block; width: 100%; text-align: left; background: none; border: 0; padding: 4px 0;
    color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .path:hover { color: var(--accent); }
</style>
