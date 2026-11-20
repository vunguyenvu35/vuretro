<script>
  import { app, pickRomFiles, pickRomFolder, linkFolder, unlinkFolder, setSidebarWidth } from '../lib/store.svelte.js';
  import { colorOf } from '../lib/systems.js';
  import logo from '../lib/logo-mark.svg';
  import Icon from './Icon.svelte';
  import { SIDEBAR_DEFAULT, SIDEBAR_MIN, SIDEBAR_MAX } from '../lib/layout.js';

  let dragging = $state(false);
  function startDrag(e) {
    e.preventDefault();
    dragging = true;
    e.currentTarget.setPointerCapture(e.pointerId);
  }
  function drag(e) {
    if (dragging) setSidebarWidth(e.clientX);
  }
  function endDrag() {
    if (!dragging) return;
    dragging = false;
    setSidebarWidth(app.sidebarW, true);
  }
  /** Arrow keys resize by 16 px (64 with Shift); Home resets. */
  function resizeKey(e) {
    const step = e.shiftKey ? 64 : 16;
    const next = { ArrowLeft: app.sidebarW - step, ArrowRight: app.sidebarW + step, Home: SIDEBAR_DEFAULT }[e.key];
    if (next === undefined) return;
    e.preventDefault();
    setSidebarWidth(next, true);
  }

  // The app's own screens, as a 2×2 block of tiles at the bottom (a list of four rows was a lot
  // of height for four doors).
  const APP = [
    { view: 'cores', icon: 'cores', label: 'Cores', title: 'Cores & BIOS' },
    { view: 'controls', icon: 'controls', label: 'Controls', title: 'Controls' },
    { view: 'macros', icon: 'macros', label: 'Macros', title: 'Macros' },
    { view: 'settings', icon: 'settings', label: 'Settings', title: 'Settings' },
  ];

  // the "more ways to add games" menu
  let menu = $state(false);
  function run(action) {
    menu = false;
    action();
  }

  const total = $derived(app.overview?.games.length ?? 0);
  const recent = $derived(app.overview?.games.filter((g) => g.last_played).length ?? 0);
  const favs = $derived(app.overview?.games.filter((g) => g.favorite).length ?? 0);

  function go(filter, system = null) {
    app.view = 'library';
    app.filter = filter;
    app.system = system;
    app.collection = null;
  }
  function goCollection(name) {
    app.view = 'library';
    app.filter = 'all';
    app.system = null;
    app.collection = name;
  }
  const libActive = (f) => app.view === 'library' && !app.system && !app.collection && app.filter === f;
</script>

<aside>
  <button class="brand" onclick={() => go('all')} title="Vuretro: back to the library" aria-label="Vuretro, back to the library">
    <span class="mark"><img src={logo} alt="" width="40" height="40" /></span>
    <span class="words">
      <span class="name"><b>Vu</b><span class="re">retro</span></span>
      <span class="tag">Play<em>·</em>Save<em>·</em>Resume</span>
    </span>
  </button>

  <nav>
    <div class="label">Library</div>
    <button class:active={libActive('all')} onclick={() => go('all')}><Icon name="all" /><span class="nm">All</span> <small>{total}</small></button>
    <button class:active={libActive('recent')} onclick={() => go('recent')}><Icon name="recent" /><span class="nm">Recent</span> <small>{recent}</small></button>
    <button class:active={libActive('favorites')} onclick={() => go('favorites')}><Icon name="favorites" /><span class="nm">Favorites</span> <small>{favs}</small></button>

    <div class="label gap">Systems</div>
    {#each app.overview?.systems ?? [] as s (s.id)}
      <button
        class:active={app.view === 'library' && app.system === s.id}
        class:off={!s.enabled}
        onclick={() => go('all', s.id)}
      >
        <span class="sysico" style="color:{colorOf(s.id)}"><Icon name={s.id} /></span><span class="nm" title={s.name}>{s.name}</span> <small>{s.game_count}</small>
      </button>
    {/each}

    {#if app.overview?.collections?.length}
      <div class="label gap">Collections</div>
      {#each app.overview.collections as c (c)}
        <button class:active={app.view === 'library' && app.collection === c} onclick={() => goCollection(c)}>
          <Icon name="favorites" /><span class="nm" title={c}>{c}</span> <small>{app.overview.games.filter((g) => g.tags?.includes(c)).length}</small>
        </button>
      {/each}
    {/if}
  </nav>

  <div class="dock">
    <div class="tiles" role="group" aria-label="App">
      {#each APP as t (t.view)}
        <button class="tile" class:active={app.view === t.view} aria-current={app.view === t.view ? 'page' : undefined} title={t.title} onclick={() => (app.view = t.view)}>
          <Icon name={t.icon} size={20} />
          <span>{t.label}</span>
        </button>
      {/each}
    </div>

    <div class="addrow">
      <button class="addbtn" onclick={pickRomFiles} disabled={app.busy} aria-busy={app.busy}>
        {#if app.busy}
          <span class="spin" aria-hidden="true"></span> Importing…
        {:else}
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round" /></svg>
          Add games
        {/if}
      </button>
      <button class="more" aria-label="More ways to add games" aria-haspopup="menu" aria-expanded={menu} disabled={app.busy} onclick={(e) => { e.stopPropagation(); menu = !menu; }}>
        <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="3.5" cy="8" r="1.3" fill="currentColor" /><circle cx="8" cy="8" r="1.3" fill="currentColor" /><circle cx="12.5" cy="8" r="1.3" fill="currentColor" /></svg>
      </button>

      {#if menu}
        <div class="menu" role="menu" aria-label="Add games" tabindex="-1" onpointerdown={(e) => e.stopPropagation()}>
          <button role="menuitem" onclick={() => run(pickRomFiles)}>
            <Icon name="file" size={16} /><span>Import ROM files…</span>
          </button>
          <button role="menuitem" onclick={() => run(pickRomFolder)}>
            <Icon name="folder" size={16} /><span>Import a folder…</span>
          </button>
          <div class="sep" role="separator"></div>
          {#if app.overview?.rom_dir}
            <div class="linked">
              <div class="lhead"><Icon name="link" size={14} /><span>Linked folder (read in place)</span></div>
              <div class="path mono" title={app.overview.rom_dir}>{app.overview.rom_dir}</div>
              <div class="acts">
                <button onclick={() => run(linkFolder)}>Change</button>
                <button onclick={() => run(unlinkFolder)}>Remove</button>
              </div>
            </div>
          {:else}
            <button role="menuitem" onclick={() => run(linkFolder)} title="Read games from a folder without copying them">
              <Icon name="link" size={16} /><span>Link a folder…</span>
            </button>
          {/if}
        </div>
      {/if}
    </div>

    {#if app.busy}
      <p class="hint" aria-live="polite">Copying games. Unpacking a disc from an archive can take a minute.</p>
    {:else if app.system}
      <p class="hint">Archives (.zip) go to {app.overview?.systems.find((s) => s.id === app.system)?.name}.</p>
    {:else}
      <p class="hint">Or drop files on the library.</p>
    {/if}
  </div>
  <div
    class="grip"
    class:on={dragging}
    role="separator"
    aria-orientation="vertical"
    aria-label="Resize the sidebar"
    aria-valuemin={SIDEBAR_MIN}
    aria-valuemax={SIDEBAR_MAX}
    aria-valuenow={app.sidebarW}
    tabindex="0"
    title="Drag to resize, double-click to reset"
    onpointerdown={startDrag}
    onpointermove={drag}
    onpointerup={endDrag}
    onpointercancel={endDrag}
    ondblclick={() => setSidebarWidth(SIDEBAR_DEFAULT, true)}
    onkeydown={resizeKey}
  ></div>
</aside>

<svelte:window onblur={endDrag} onpointerdown={() => (menu = false)} onkeydown={(e) => e.key === 'Escape' && (menu = false)} />
{#if dragging}<div class="dragshield"></div>{/if}

<style>
  aside {
    position: relative; min-width: 0; container: side / inline-size;
    /* a faint pink light from the mark at the top left, so the head of the sidebar has depth */
    background: radial-gradient(260px 150px at 40px 30px, color-mix(in srgb, var(--brand-magenta) 8%, transparent), transparent 75%), var(--surface);
    border-right: 1px solid var(--border);
    display: flex; flex-direction: column; padding: 14px 10px; gap: 8px; min-height: 0;
  }
  /* a thin strip on the sidebar's right edge: easy to grab, invisible until hovered */
  .grip { position: absolute; top: 0; right: -3px; bottom: 0; width: 7px; cursor: col-resize; z-index: 5; touch-action: none; }
  .grip::after { content: ""; position: absolute; top: 0; bottom: 0; left: 3px; width: 1px; background: transparent; transition: background .12s, box-shadow .12s; }
  .grip:hover::after, .grip:focus-visible::after, .grip.on::after { background: var(--accent); box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 35%, transparent); }
  .grip:focus-visible { outline: none; }
  /* while dragging, keep the cursor and ignore what is underneath */
  .dragshield { position: fixed; inset: 0; z-index: 100; cursor: col-resize; user-select: none; }
  /* ---- the brand: the mark, the name, the tagline, over a soft light that comes from the mark ---- */
  .brand {
    position: relative; display: flex; align-items: center; gap: 12px; width: 100%;
    margin: 0 0 8px; padding: 6px 8px 18px; background: none; border: 0; border-radius: 12px; text-align: left;
  }
  /* the foot line starts in the brand's magenta and fades out into the sidebar's own border colour */
  .brand::after {
    content: ""; position: absolute; left: 8px; right: 8px; bottom: 0; height: 1px;
    background: linear-gradient(90deg, color-mix(in srgb, var(--brand-magenta) 50%, var(--border)), var(--border) 45%, transparent);
  }
  .brand:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  /* the tile is nearly the colour of the sidebar, so it gets a rim of light and a pink shadow to stand off it */
  .mark img {
    display: block; border-radius: 22%;
    box-shadow: 0 0 0 1px rgb(255 255 255 / .1), 0 2px 6px rgb(0 0 0 / .5), 0 10px 22px -10px rgb(255 79 139 / .6);
    transition: transform .18s ease, box-shadow .18s ease;
  }
  .brand:hover .mark img { transform: translateY(-1px); box-shadow: 0 0 0 1px rgb(255 255 255 / .16), 0 3px 8px rgb(0 0 0 / .5), 0 14px 26px -10px rgb(255 79 139 / .85); }
  .brand:active .mark img { transform: translateY(0); }
  .words { display: flex; flex-direction: column; min-width: 0; }
  .name { font: 700 22px/1 var(--font-head); letter-spacing: -.025em; white-space: nowrap; }
  .name b { color: var(--brand-magenta); font-weight: 700; }
  .name .re { color: #f4f1ff; font-weight: 500; }
  .tag { margin-top: 7px; font: 600 8.5px/1 var(--font-body); letter-spacing: .14em; text-transform: uppercase; color: var(--muted); white-space: nowrap; }
  .tag em { font-style: normal; color: var(--accent); margin: 0 .5em; }
  /* a narrow sidebar keeps the mark and the name and drops the tagline */
  @container side (max-width: 190px) { .tag { display: none; } }
  @media (prefers-reduced-motion: reduce) { .mark img { transition: none; } .brand:hover .mark img { transform: none; } }
  nav { display: flex; flex-direction: column; gap: 2px; overflow-y: auto; overflow-x: hidden; flex: 1; min-height: 0; scrollbar-gutter: stable; }
  .label { padding: 0 8px; margin: 4px 0; }
  .gap { margin-top: 16px; }
  nav button {
    display: flex; align-items: center; gap: 10px; text-align: left; background: none; border: 0; white-space: nowrap;
    border-left: 3px solid transparent; padding: 7px 8px; border-radius: 0 6px 6px 0; color: var(--muted);
  }
  nav button:hover { background: var(--surface-2); color: var(--text); }
  nav button.active { color: var(--accent); background: var(--surface-2); border-left-color: var(--accent); }
  nav button.off { opacity: .62; }
  /* the name takes the free space and is cut with an ellipsis; the count stays in its own column */
  .nm { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  small { flex: none; min-width: 2.2ch; text-align: right; color: var(--faint); font-variant-numeric: tabular-nums; }
  nav button :global(svg) { flex: none; opacity: .85; transition: opacity .12s; }
  nav button:hover :global(svg), nav button.active :global(svg) { opacity: 1; }
  .sysico { display: inline-flex; flex: none; }
  i { width: 8px; height: 8px; border-radius: 50%; flex: none; }
  /* ---- the dock: app screens and adding games, always at the bottom ---- */
  .dock { position: relative; display: flex; flex-direction: column; gap: 10px; padding: 12px 2px 2px; border-top: 1px solid var(--border); }
  .tiles { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; }
  .tile {
    display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 5px;
    padding: 10px 4px 8px; background: var(--surface-2); border: 1px solid var(--border); border-radius: 10px;
    color: var(--muted); font-size: 12px; transition: color .12s, border-color .12s, background .12s;
  }
  .tile :global(svg) { opacity: .9; }
  .tile:hover { color: var(--text); border-color: var(--faint); }
  .tile.active {
    color: var(--accent); border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
    background: color-mix(in srgb, var(--accent) 9%, var(--surface-2));
  }
  .addrow { position: relative; display: flex; gap: 6px; }
  .addbtn {
    flex: 1; min-width: 0; display: flex; align-items: center; justify-content: center; gap: 8px;
    padding: 10px 12px; border: 0; border-radius: 10px; background: var(--accent); color: #090b10;
    font-weight: 700; font-size: 13px; transition: filter .12s, transform .06s;
  }
  .addbtn:hover:not(:disabled) { filter: brightness(1.08); }
  .addbtn:active:not(:disabled) { transform: translateY(1px); }
  .addbtn:disabled { opacity: .7; }
  .spin { width: 13px; height: 13px; border-radius: 50%; border: 2px solid color-mix(in srgb, #090b10 30%, transparent); border-top-color: #090b10; animation: spin .8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .more {
    flex: none; width: 40px; display: grid; place-items: center; border-radius: 10px;
    background: var(--surface-2); border: 1px solid var(--border); color: var(--muted);
  }
  .more:hover:not(:disabled), .more[aria-expanded="true"] { color: var(--text); border-color: var(--faint); }
  .more:disabled { opacity: .55; }
  .menu {
    position: absolute; left: 0; right: 0; bottom: calc(100% + 8px); z-index: 20;
    background: var(--surface-2); border: 1px solid var(--border); border-radius: 12px; padding: 6px;
    box-shadow: 0 12px 32px rgba(0,0,0,.45); display: flex; flex-direction: column; gap: 2px;
  }
  .menu > button { display: flex; align-items: center; gap: 10px; text-align: left; background: none; border: 0; padding: 9px 10px; border-radius: 8px; color: var(--text); font-size: 13px; }
  .menu > button:hover { background: color-mix(in srgb, var(--accent) 12%, transparent); color: var(--accent); }
  .menu > button :global(svg) { flex: none; opacity: .85; }
  .sep { height: 1px; background: var(--border); margin: 4px 2px; }
  .hint { color: var(--faint); font-size: 11px; margin: 0 2px; line-height: 1.4; }
  .linked { padding: 6px 10px 6px; }
  .lhead { display: flex; align-items: center; gap: 6px; color: var(--muted); font-size: 11px; }
  .path { display: block; margin: 6px 0 4px; font-size: 11px; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .acts { display: flex; gap: 14px; }
  .acts button { background: none; border: 0; padding: 2px 0; color: var(--muted); font-size: 12px; }
  .acts button:hover { color: var(--accent); }
</style>
