<script>
  import { app, pickRomFiles, pickRomFolder, linkFolder, unlinkFolder, setSidebarWidth, toggleSidebar, toggleSection } from '../lib/store.svelte.js';
  import { colorOf } from '../lib/systems.js';
  import logo from '../lib/logo-mark.svg';
  import Icon from './Icon.svelte';
  import { tick } from 'svelte';
  import { slide, fade } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import { update } from '../lib/update.svelte.js';
  import { SIDEBAR_DEFAULT, SIDEBAR_MIN, SIDEBAR_MAX, filterSystems } from '../lib/layout.js';

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

  // the search box of the Systems group, and the switch that hides the systems with no games
  let query = $state('');
  let onlyWithGames = $state(false);
  let searching = $state(false);
  let field = $state(null);
  /** the magnifier opens the field (and the group, if it was closed) and puts the cursor in it */
  async function openSearch() {
    searching = true;
    if (app.folded.systems) toggleSection('systems');
    await tick();
    field?.focus();
  }
  /** closing the field also clears what it filtered, so the list is whole again */
  function closeSearch() {
    searching = false;
    query = '';
    onlyWithGames = false;
  }
  const systems = $derived(filterSystems(app.overview?.systems ?? [], query, onlyWithGames, app.view === 'library' ? app.system : null));
  const allSystems = $derived(app.overview?.systems.length ?? 0);
  const rail = $derived(app.sidebarFolded);
  // no motion for people who asked the system for less of it
  const calm = typeof matchMedia !== 'undefined' && matchMedia('(prefers-reduced-motion: reduce)').matches;
  const ms = (n) => (calm ? 0 : n);
  /** a group is shown open in the rail (it has no headings) and when it was not closed */
  const open = (name) => rail || !app.folded[name];

  /** ⌘B / Ctrl+B folds or opens the sidebar. */
  function key(e) {
    if (e.key === 'Escape') menu = false;
    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'b' && !(e.target instanceof HTMLElement && e.target.closest('input, textarea'))) {
      e.preventDefault();
      toggleSidebar();
    }
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

<aside class:rail>
  <button class="brand" onclick={() => go('all')} title="Vuretro: back to the library" aria-label="Vuretro, back to the library">
    <span class="mark"><img src={logo} alt="" width="40" height="40" /></span>
    <span class="words">
      <span class="name"><b>Vu</b><span class="re">retro</span></span>
      <span class="tag">Play<em>·</em>Save<em>·</em>Resume</span>
    </span>
  </button>

  <button class="fold" onclick={toggleSidebar} aria-label={rail ? 'Open the sidebar' : 'Fold the sidebar'} title={rail ? 'Open the sidebar (Ctrl/⌘ B)' : 'Fold the sidebar to icons (Ctrl/⌘ B)'}>
    <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d={rail ? 'M6 3l5 5-5 5' : 'M10 3L5 8l5 5'} fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>
  </button>

  <nav>
    <button class="label head" aria-expanded={open('library')} onclick={() => toggleSection('library')}><span class="chev" class:shut={!open('library')} aria-hidden="true">▾</span>Library</button>
    {#if open('library')}
    <div class="grp" transition:slide={{ duration: ms(170) }}>
    <button class:active={libActive('all')} onclick={() => go('all')}><Icon name="all" /><span class="nm">All</span> <small>{total}</small></button>
    <button class:active={libActive('recent')} onclick={() => go('recent')}><Icon name="recent" /><span class="nm">Recent</span> <small>{recent}</small></button>
    <button class:active={libActive('favorites')} onclick={() => go('favorites')}><Icon name="favorites" /><span class="nm">Favorites</span> <small>{favs}</small></button>
    </div>
    {/if}

    <button class="label head gap" aria-expanded={open('native')} onclick={() => toggleSection('native')}><span class="chev" class:shut={!open('native')} aria-hidden="true">▾</span>Native games</button>
    {#if open('native')}
    <div class="grp" transition:slide={{ duration: ms(170) }}>
    <button class:active={app.view === 'native'} aria-current={app.view === 'native' ? 'page' : undefined} onclick={() => (app.view = 'native')}>
      <Icon name="all" /><span class="nm" title="Games that are programs of their own">Programs</span> <small>{app.native.length}</small>
    </button>
    </div>
    {/if}

    <div class="headrow gap">
      {#if searching}
        <div class="find" class:filled={query || onlyWithGames} transition:slide={{ axis: 'x', duration: ms(200) }}>
          <svg class="mag" viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" stroke-width="1.6" /><path d="M10.5 10.5L14 14" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg>
          <input type="text" placeholder="Find a system" aria-label="Find a system" autocomplete="off" spellcheck="false" bind:value={query} bind:this={field} onkeydown={(e) => e.key === 'Escape' && closeSearch()} />
          <button class="only" class:on={onlyWithGames} aria-pressed={onlyWithGames} aria-label="Only systems that have games" title={onlyWithGames ? 'Showing only systems with games' : 'Show only systems with games'} onclick={() => (onlyWithGames = !onlyWithGames)}>
            <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M2.5 4h11M4.5 8h7M6.5 12h3" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg>
          </button>
          <button class="x" aria-label="Close the search" title="Close (Esc)" onclick={closeSearch}>
            <svg viewBox="0 0 16 16" width="10" height="10" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" /></svg>
          </button>
        </div>
      {:else}
        <button class="label head" aria-expanded={open('systems')} onclick={() => toggleSection('systems')}>
          <span class="chev" class:shut={!open('systems')} aria-hidden="true">▾</span>Systems
        </button>
        {#if !rail}
          <button class="seek" aria-label="Find a system" title="Find a system" onclick={openSearch}>
            <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" stroke-width="1.6" /><path d="M10.5 10.5L14 14" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg>
          </button>
        {/if}
        <span class="cnt">{allSystems}</span>
      {/if}
    </div>
    {#if open('systems') || searching}
    <div class="grp" transition:slide={{ duration: ms(190) }}>
    {#each systems as s (s.id)}
      <button
        animate:flip={{ duration: ms(180) }}
        in:fade={{ duration: ms(140) }}
        style="--c:{colorOf(s.id)}"
        class:active={app.view === 'library' && app.system === s.id}
        class:off={!s.enabled}
        title={rail ? `${s.name} (${s.game_count})` : undefined}
        onclick={() => go('all', s.id)}
      >
        <span class="sysico" style="color:{colorOf(s.id)}"><Icon name={s.id} /></span><span class="nm" title={s.name}>{s.name}</span> <small>{s.game_count}</small>
      </button>
    {:else}
      <p class="none" in:fade={{ duration: ms(140) }}>No system matches “{query}”.</p>
    {/each}
    </div>
    {/if}

    {#if app.overview?.collections?.length}
      <button class="label head gap" aria-expanded={open('collections')} onclick={() => toggleSection('collections')}><span class="chev" class:shut={!open('collections')} aria-hidden="true">▾</span>Collections</button>
      {#if open('collections')}
      <div class="grp" transition:slide={{ duration: ms(170) }}>
      {#each app.overview.collections as c (c)}
        <button class:active={app.view === 'library' && app.collection === c} onclick={() => goCollection(c)}>
          <Icon name="favorites" /><span class="nm" title={c}>{c}</span> <small>{app.overview.games.filter((g) => g.tags?.includes(c)).length}</small>
        </button>
      {/each}
      </div>
      {/if}
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

    {#if update.status === 'available' || update.status === 'installing'}
      <button class="upill" onclick={() => (app.dialog = 'about')} title="Open About to update">
        <span class="dot"></span>{update.status === 'installing' ? 'Updating…' : `Update ${update.version} available`}
      </button>
    {/if}
    {#if app.busy}
      <p class="hint" aria-live="polite">Copying games. Unpacking a disc from an archive can take a minute.</p>
    {:else if app.system}
      <p class="hint">Archives (.zip) go to {app.overview?.systems.find((s) => s.id === app.system)?.name}.</p>
    {:else}
      <p class="hint">Or drop files on the library.</p>
    {/if}
  </div>
  {#if !rail}<div
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
  ></div>{/if}
</aside>

<svelte:window onblur={endDrag} onpointerdown={() => (menu = false)} onkeydown={key} />
{#if dragging}<div class="dragshield"></div>{/if}

<style>
  aside {
    position: relative; min-width: 0; container: side / inline-size;
    /* a faint pink light from the mark at the top left, so the head of the sidebar has depth */
    background: radial-gradient(260px 150px at 40px 28px, color-mix(in srgb, var(--brand-magenta) 8%, transparent), transparent 75%), var(--surface);
    border-right: 1px solid var(--border);
    display: flex; flex-direction: column; padding: 0 10px 14px; gap: 8px; min-height: 0;
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
    flex: none; box-sizing: border-box; height: var(--strip); margin: 0 0 8px; padding: 0 8px; background: none; border: 0; border-radius: 12px; text-align: left;
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
  .grp { display: flex; flex-direction: column; gap: 2px; }
  /* effects: a system's own colour lights its row when it is open, and its icon lifts on hover */
  nav button[style*="--c"].active {
    background: linear-gradient(90deg, color-mix(in srgb, var(--c) 18%, transparent), color-mix(in srgb, var(--c) 3%, transparent) 70%, transparent);
    border-left-color: var(--c); color: var(--text);
    box-shadow: inset 14px 0 18px -16px var(--c);
  }
  nav button[style*="--c"].active small { color: var(--muted); }
  nav button { transition: background .14s, color .14s, border-color .14s, transform .14s; }
  nav button:not(.label):hover { transform: translateX(2px); }
  .sysico { transition: transform .18s cubic-bezier(.3, 1.6, .5, 1), filter .18s; }
  nav button:hover .sysico { transform: scale(1.12) rotate(-4deg); filter: drop-shadow(0 0 6px color-mix(in srgb, currentColor 55%, transparent)); }
  @media (prefers-reduced-motion: reduce) {
    nav button, .sysico, .chev, nav .seek { transition: none; }
    nav button:not(.label):hover, nav button:hover .sysico { transform: none; }
  }
  /* a heading that opens and closes its group: the small capitals of the other headings, with a
     chevron, and a count on the right when the group has one */
  nav button.label.head {
    display: flex; align-items: center; gap: 6px; width: 100%; border: 0; border-radius: 6px; background: none; padding: 5px 8px; margin: 4px 0;
    font-size: 10px; letter-spacing: .08em; text-transform: uppercase; font-weight: 600; color: var(--muted);
  }
  nav button.label.head:hover { background: none; color: var(--text); }
  .chev { display: inline-block; font-size: 9px; width: 9px; transition: transform .12s; }
  .chev.shut { transform: rotate(-90deg); }
  .cnt { margin-left: auto; font-weight: 500; letter-spacing: 0; font-variant-numeric: tabular-nums; color: var(--faint); }
  /* the Systems heading: the title opens and closes the group, a magnifier at its end opens the search */
  .headrow { display: flex; align-items: center; gap: 2px; min-height: 30px; }
  .headrow nav button.label.head, .headrow .label.head { flex: none; width: auto; }
  .headrow .cnt { margin-left: auto; padding-right: 8px; }
  nav .seek {
    flex: none; display: grid; place-items: center; width: 26px; height: 26px; padding: 0; border: 0; border-radius: 7px;
    background: none; color: var(--faint); transition: color .14s, background .14s, transform .18s;
  }
  nav .seek:hover { background: var(--surface-2); color: var(--accent); transform: scale(1.1); }
  /* the field grows out of the magnifier to the width of the row: a magnifier, the text, the "with games" switch and a close button */
  .find {
    flex: 1; min-width: 0; display: flex; align-items: center; gap: 2px; height: 30px; padding: 0 3px 0 9px; border-radius: 9px; overflow: hidden;
    background: var(--surface-2); border: 1px solid color-mix(in srgb, var(--accent) 55%, var(--border)); color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .mag { flex: none; margin-right: 6px; }
  .find input { flex: 1; min-width: 0; height: 100%; padding: 0; border: 0; background: none; color: var(--text); font-size: 12.5px; }
  .find input::placeholder { color: var(--faint); }
  .find input:focus, .find input:focus-visible { outline: none; }
  .find button, nav .find button {
    flex: none; display: grid; place-items: center; width: 24px; height: 24px; padding: 0; border: 0; border-left: 0; border-radius: 6px; background: none; color: var(--faint);
  }
  .find button:hover { background: color-mix(in srgb, var(--text) 8%, transparent); color: var(--text); }
  .find button.only.on { color: var(--accent); background: color-mix(in srgb, var(--accent) 16%, transparent); }
  .none { margin: 4px 10px; color: var(--faint); font-size: 12px; }
  /* the fold button sits on the sidebar's edge, level with the brand */
  .fold {
    position: absolute; top: calc(var(--strip) / 2 - 11px); right: -11px; z-index: 6; width: 22px; height: 22px; display: grid; place-items: center;
    border-radius: 50%; border: 1px solid var(--border); background: var(--surface-2); color: var(--muted); opacity: 0; transition: opacity .12s;
  }
  aside:hover .fold, .fold:focus-visible { opacity: 1; }
  .fold:hover { color: var(--text); border-color: var(--faint); }
  /* ---- the rail: icons only ---- */
  aside.rail { padding: 0 6px 12px; }
  aside.rail .words, aside.rail .nm, aside.rail small, aside.rail .label.head, aside.rail .tile span, aside.rail .hint, aside.rail .upill { display: none; }
  aside.rail .brand { justify-content: center; padding: 0; }
  aside.rail nav button { justify-content: center; border-left-width: 0; border-radius: 8px; padding: 8px 0; }
  aside.rail .tiles { grid-template-columns: 1fr; }
  aside.rail .tile { padding: 9px 0; }
  aside.rail .addbtn { padding: 10px 0; font-size: 0; gap: 0; }
  aside.rail .more { display: none; }
  aside.rail .gap { margin-top: 8px; }
  aside.rail .fold { opacity: 1; }
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
  .upill { display: flex; align-items: center; gap: 8px; margin: 0 0 8px; padding: 7px 10px; border-radius: 9px; border: 1px solid color-mix(in srgb, var(--accent) 50%, var(--border)); background: color-mix(in srgb, var(--accent) 10%, transparent); color: var(--accent); font-size: 12px; text-align: left; }
  .upill:hover { background: color-mix(in srgb, var(--accent) 18%, transparent); }
  .upill .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--accent); flex: none; }
  .hint { color: var(--faint); font-size: 11px; margin: 0 2px; line-height: 1.4; }
  .linked { padding: 6px 10px 6px; }
  .lhead { display: flex; align-items: center; gap: 6px; color: var(--muted); font-size: 11px; }
  .path { display: block; margin: 6px 0 4px; font-size: 11px; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .acts { display: flex; gap: 14px; }
  .acts button { background: none; border: 0; padding: 2px 0; color: var(--muted); font-size: 12px; }
  .acts button:hover { color: var(--accent); }
</style>
