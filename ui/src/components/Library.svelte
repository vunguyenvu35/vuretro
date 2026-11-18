<script>
  import { app, play, refresh, importRoms, removeRom, removeMany, toggleFavorite, covers, setLayout, pickRomFiles, pickRomFolder } from '../lib/store.svelte.js';
  import { openMenu, confirmDialog } from '../lib/ui.svelte.js';
  import { visibleGames, lastPlayed, headingFor, pickRange, formatSize } from '../lib/library.js';
  import Welcome from "./Welcome.svelte";
  import Icon from './Icon.svelte';
  import SaveSlots from './SaveSlots.svelte';
  import Cheats from './Cheats.svelte';
  let newTag = $state('');
  async function setTags(game, tags) {
    await api.setTags(game.path, tags);
    await refresh();
  }
  async function addTag(game) {
    const t = newTag.trim();
    newTag = '';
    if (t) await setTags(game, [...(game.tags ?? []), t]);
  }
  import { api } from '../lib/api.js';
  import { colorOf, formatAgo, formatPlaytime, modKey } from '../lib/systems.js';

  import { onMount } from "svelte";
  let searchEl = $state();

  const ov = $derived(app.overview);
  const sysName = (id) => ov?.systems.find((s) => s.id === id)?.name ?? id;

  const visible = $derived(visibleGames(ov, app));
  const hero = $derived(lastPlayed(ov?.games ?? []));
  const selected = $derived(ov?.games.find((g) => g.path === app.selected) ?? null);
  const sel = $derived(selected ? ov.systems.find((s) => s.id === selected.system) : null);
  const heading = $derived(headingFor(ov, app));

  let over = $state(false);
  onMount(() => {
    document.activeElement?.blur?.();
    let off = () => {};
    api
      .onDragDrop((p) => {
        if (p.type === 'over' || p.type === 'enter') over = true;
        else if (p.type === 'leave') over = false;
        else if (p.type === 'drop') {
          over = false;
          importRoms(p.paths ?? []);
        }
      })
      .then((f) => (off = f));
    return () => off();
  });
  // ---- choosing several games to remove ----
  const removable = $derived(visible.filter((g) => g.imported));
  const pickedGames = $derived(removable.filter((g) => app.picked.includes(g.path)));
  const allPicked = $derived(removable.length > 0 && pickedGames.length === removable.length);
  function toggleAll() {
    app.picked = allPicked ? [] : removable.map((g) => g.path);
  }
  let lastPick = null;
  let autoSelect = false; // selection mode was started by a modifier-click, not the Select button

  function pick(g, e) {
    if (!g.imported) return;
    if (!app.selecting) autoSelect = true;
    app.selecting = true;
    if (e?.shiftKey && lastPick) {
      const range = pickRange(visible, lastPick, g.path);
      app.picked = [...new Set([...app.picked, ...range])];
    } else {
      app.picked = app.picked.includes(g.path) ? app.picked.filter((p) => p !== g.path) : [...app.picked, g.path];
    }
    lastPick = g.path;
    if (autoSelect && !app.picked.length) stopSelecting();
  }
  function cardClick(g, e) {
    if (app.selecting || e.metaKey || e.ctrlKey || e.shiftKey) pick(g, e);
    else app.selected = g.path;
  }
  function stopSelecting() {
    autoSelect = false;
    app.selecting = false;
    app.picked = [];
    lastPick = null;
  }
  async function removePicked() {
    const games = [...pickedGames];
    const n = games.length;
    if (!n) return;
    const yes = await confirmDialog({
      title: n === 1 ? 'Remove this game?' : `Remove ${n} games?`,
      message: `${n === 1 ? 'It' : 'They'} will be deleted from the vuretro library, along with ${n === 1 ? 'its' : 'their'} play history and preview.`,
      note: `Your original files are not touched. You can import ${n === 1 ? 'it' : 'them'} again any time.`,
      confirmLabel: n === 1 ? 'Remove game' : `Remove ${n} games`,
      danger: true,
    });
    if (yes) removeMany(games);
  }

  async function remove(g) {
    const yes = await confirmDialog({
      title: `Remove “${g.title}”?`,
      message: 'It will be deleted from the vuretro library, along with its play history and preview.',
      note: 'Your original file is not touched. You can import it again any time.',
      confirmLabel: 'Remove game',
      danger: true,
    });
    if (yes) removeRom(g);
  }

  /** Move the selection through the grid (dx: left/right, dy: up/down). */
  function move(dx, dy) {
    const cards = [...document.querySelectorAll('.grid .card, .list .card')];
    if (!cards.length) return;
    const cols = Math.max(1, cards.filter((c) => c.offsetTop === cards[0].offsetTop).length);
    let i = visible.findIndex((g) => g.path === app.selected);
    i = i < 0 ? 0 : Math.min(visible.length - 1, Math.max(0, i + dx + dy * cols));
    app.selected = visible[i].path;
    cards[i]?.scrollIntoView({ block: 'nearest' });
  }

  function onkey(e) {
    if (app.capturing) return;
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      searchEl?.focus();
      return;
    }
    if (e.key === 'Escape') {
      if (app.selecting) return stopSelecting();
      app.query = '';
      searchEl?.blur();
      return;
    }
    if (mod && e.key.toLowerCase() === 'a' && removable.length) {
      e.preventDefault();
      app.selecting = true;
      app.picked = removable.map((g) => g.path);
      return;
    }
    if (mod || e.altKey) return;
    const tag = e.target instanceof HTMLElement ? e.target.tagName : '';
    if (['INPUT', 'TEXTAREA', 'SELECT'].includes(tag)) {
      // from the search box, Down goes to the results
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        searchEl?.blur();
        move(0, 0);
      }
      return;
    }
    if (e.key === 'Delete' || e.key === 'Backspace') {
      e.preventDefault();
      if (app.picked.length) removePicked();
      else if (selected?.imported) remove(selected);
      return;
    }
    const arrows = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
    if (arrows[e.key]) {
      e.preventDefault();
      move(...arrows[e.key]);
    } else if (e.key === 'Enter' && tag !== 'BUTTON' && selected) {
      play(selected);
    } else if (e.key.toLowerCase() === 'f' && selected) {
      fav(selected);
    }
  }
  const SEP = { separator: true };

  /** Right-click on a game. */
  function gameMenu(e, g) {
    const many = app.selecting && app.picked.includes(g.path) && pickedGames.length > 1;
    if (many) {
      return openMenu(e, [
        { label: `Remove ${pickedGames.length} games from library…`, danger: true, action: removePicked },
        SEP,
        { label: 'Select all', action: toggleAllOn },
        { label: 'Done selecting', action: stopSelecting },
      ], `${pickedGames.length} selected`);
    }
    if (!app.selecting) app.selected = g.path;
    openMenu(e, [
      { label: g.blocked ? 'Play (unavailable)' : 'Play', hint: '⏎', disabled: !!g.blocked, action: () => play(g) },
      { label: g.favorite ? 'Remove from favorites' : 'Add to favorites', hint: 'F', action: () => fav(g) },
      SEP,
      { label: 'Show in folder', action: () => api.reveal(g.path) },
      { label: 'Select…', disabled: !g.imported, action: () => pick(g) },
      SEP,
      { label: 'Remove from library…', danger: true, disabled: !g.imported, hint: g.imported ? '⌫' : '', action: () => remove(g) },
    ], g.title);
  }

  /** Right-click on empty space. */
  function listMenu(e) {
    openMenu(e, [
      { label: 'Import ROMs…', hint: `${modKey}O`, action: pickRomFiles },
      { label: 'Import a folder…', hint: `${modKey}⇧O`, action: pickRomFolder },
      SEP,
      { label: 'Refresh library', hint: `${modKey}R`, action: refresh },
      SEP,
      { label: app.layout === 'list' ? 'View as grid' : 'View as list', action: () => setLayout(app.layout === 'list' ? 'grid' : 'list') },
      removable.length ? { label: app.selecting ? 'Done selecting' : 'Select games…', action: () => (app.selecting ? stopSelecting() : ((autoSelect = false), (app.selecting = true))) } : SEP,
    ]);
  }
  const toggleAllOn = () => (app.picked = removable.map((g) => g.path));

  const fav = toggleFavorite;
</script>

<svelte:window onkeydown={onkey} />

{#if !ov || ov.games.length === 0}
  <Welcome />
{:else}
{#if over}<div class="dropzone">Drop ROMs to add them to the library</div>{/if}
<section class="main">
  <div class="scroll" role="presentation" oncontextmenu={listMenu}>
      {#if hero && !app.query}
        <div class="hero">
          <div class="crt scan" style="--c:{colorOf(hero.system)}">{#if covers[hero.path]}<img src={covers[hero.path]} alt="" />{:else}<span>{hero.title}</span>{/if}</div>
          <div class="hero-body">
            <div class="label">Continue playing</div>
            <h2>{hero.title}</h2>
            <p class="muted">{sysName(hero.system)} · {formatPlaytime(hero.play_secs)} played · {formatAgo(hero.last_played)}</p>
            <div class="row">
              <button class="btn primary" disabled={!!hero.blocked} onclick={() => play(hero)}>▶ Resume</button>
              <button class="btn" onclick={() => (app.selected = hero.path)}>Details</button>
            </div>
          </div>
        </div>
      {/if}

      <div class="bar">
        <h3>{heading} <small>{visible.length}</small></h3>
        <label class="search">
          <svg class="mag" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="7" cy="7" r="4.6" fill="none" stroke="currentColor" stroke-width="1.6" /><path d="M10.6 10.6L14 14" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg>
          <input id="search" bind:this={searchEl} bind:value={app.query} placeholder="Search games" aria-label="Search games" autocomplete="off" spellcheck="false" />
          {#if app.query}
            <button type="button" class="clear" onclick={() => { app.query = ''; searchEl?.focus(); }} aria-label="Clear search">✕</button>
          {:else}
            <kbd>{modKey} K</kbd>
          {/if}
        </label>
        <div class="tools">
          {#if app.selecting}
            <span class="selbar">
              <span class="count"><b>{pickedGames.length}</b> selected</span>
              <button onclick={toggleAll}>{allPicked ? 'Clear' : 'Select all'}</button>
              <button class="danger-btn" disabled={!pickedGames.length} onclick={removePicked}>Remove{pickedGames.length ? ` ${pickedGames.length}` : ''}</button>
              <button class="done" onclick={stopSelecting}>Done</button>
            </span>
          {:else if removable.length}
            <button onclick={() => { autoSelect = false; app.selecting = true; }} title="Choose several games to remove ({modKey}-click or Shift-click a game)">Select</button>
          {/if}
        </div>
        <div class="sort">
          Sort:
          {#each [['recent', 'Recent'], ['az', 'A–Z'], ['size', 'Size']] as [k, name] (k)}
            <button class:on={app.sort === k} onclick={() => (app.sort = k)}>{name}</button>
          {/each}
        </div>
        <div class="layout" role="radiogroup" aria-label="View">
          <button role="radio" aria-checked={app.layout === 'grid'} class:on={app.layout === 'grid'} onclick={() => setLayout('grid')} title="Grid" aria-label="Grid view">
            <svg viewBox="0 0 16 16" width="15" height="15"><rect x="2" y="2" width="5" height="5" rx="1.2" fill="currentColor" /><rect x="9" y="2" width="5" height="5" rx="1.2" fill="currentColor" /><rect x="2" y="9" width="5" height="5" rx="1.2" fill="currentColor" /><rect x="9" y="9" width="5" height="5" rx="1.2" fill="currentColor" /></svg>
          </button>
          <button role="radio" aria-checked={app.layout === 'list'} class:on={app.layout === 'list'} onclick={() => setLayout('list')} title="List" aria-label="List view">
            <svg viewBox="0 0 16 16" width="15" height="15"><rect x="2" y="3" width="12" height="2.4" rx="1.2" fill="currentColor" /><rect x="2" y="6.8" width="12" height="2.4" rx="1.2" fill="currentColor" /><rect x="2" y="10.6" width="12" height="2.4" rx="1.2" fill="currentColor" /></svg>
          </button>
        </div>
      </div>

      {#if visible.length === 0}
        <p class="muted pad">Nothing matches.</p>
      {:else if app.layout === 'list'}
        <div class="list" class:selecting={app.selecting} role="list">
          <div class="lhead" aria-hidden={!app.selecting}>
            {#if app.selecting}
              <button class="hcheck" class:on={allPicked} class:part={pickedGames.length > 0 && !allPicked} onclick={toggleAll} aria-label={allPicked ? 'Clear selection' : 'Select all'}>
                {#if allPicked}<svg viewBox="0 0 16 16" width="11" height="11"><path d="M3 8.5l3.2 3.2L13 4.8" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" /></svg>{:else if pickedGames.length}<svg viewBox="0 0 16 16" width="11" height="11"><path d="M3.5 8h9" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" /></svg>{/if}
              </button>
            {/if}
            <span></span><span>Game</span><span>System</span><span class="r">Size</span><span>Last played</span><span class="r">Time played</span><span></span>
          </div>
          {#each visible as g (g.path)}
            <button class="card lrow" class:sel={app.selected === g.path && !app.selecting} class:picked={app.picked.includes(g.path)} class:locked={app.selecting && !g.imported} title={app.selecting && !g.imported ? 'Games read from a linked folder cannot be removed here' : ''} onclick={(e) => cardClick(g, e)} oncontextmenu={(e) => gameMenu(e, g)} ondblclick={() => !app.selecting && play(g)}>
              {#if app.selecting}
                <span class="rtick" class:on={app.picked.includes(g.path)} class:off={!g.imported} aria-hidden="true">{#if app.picked.includes(g.path)}<svg viewBox="0 0 16 16" width="11" height="11"><path d="M3 8.5l3.2 3.2L13 4.8" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" /></svg>{/if}</span>
              {/if}
              <span class="thumb cover scan" style="--c:{colorOf(g.system)}">{#if covers[g.path]}<img src={covers[g.path]} alt="" loading="lazy" />{/if}</span>
              <span class="rname"><b>{g.title}</b>{#if g.favorite}<i class="rstar" title="Favorite">★</i>{/if}{#if g.blocked}<i class="rlock" title={g.blocked}>🔒</i>{/if}</span>
              <span class="rsys"><span class="sico" style="color:{colorOf(g.system)}"><Icon name={g.system} size={16} /></span>{sysName(g.system)}</span>
              <span class="r muted2">{formatSize(g.size)}</span>
              <span class="muted2">{g.last_played ? formatAgo(g.last_played) : '—'}</span>
              <span class="r muted2">{g.play_secs ? formatPlaytime(g.play_secs) : '—'}</span>
              <span class="rplay" class:off={app.selecting || !!g.blocked} role="button" tabindex="-1" aria-label="Play {g.title}" title={g.blocked ? g.blocked : 'Play'} onclick={(e) => { e.stopPropagation(); if (!app.selecting) play(g); }} onkeydown={() => {}}>
                <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"><path d="M4.5 2.8v10.4L13 8z" fill="currentColor" /></svg>
              </span>
            </button>
          {/each}
        </div>
      {:else}
        <div class="grid">
          {#each visible as g (g.path)}
            <button class="card" class:sel={app.selected === g.path && !app.selecting} class:picked={app.picked.includes(g.path)} class:locked={app.selecting && !g.imported} title={app.selecting && !g.imported ? 'Games read from a linked folder cannot be removed here' : ''} onclick={(e) => cardClick(g, e)} oncontextmenu={(e) => gameMenu(e, g)} ondblclick={() => !app.selecting && play(g)}>
              <div class="cover scan" style="--c:{colorOf(g.system)}">
                {#if app.selecting && g.imported}<span class="tick" aria-hidden="true">{#if app.picked.includes(g.path)}<svg viewBox="0 0 16 16" width="12" height="12"><path d="M3 8.5l3.2 3.2L13 4.8" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" /></svg>{/if}</span>{/if}
                {#if covers[g.path]}<img src={covers[g.path]} alt="" loading="lazy" />{:else}<span>{g.title.slice(0, 22)}</span>{/if}
                {#if g.favorite}<b class="star">★</b>{/if}
                {#if g.blocked}<b class="lock" title={g.blocked}>🔒</b>{/if}
              </div>
              <div class="title">{g.title}</div>
              <div class="sub">{sysName(g.system)}</div>
            </button>
          {/each}
        </div>
      {/if}
  </div>
</section>

{#if selected && sel}
  <aside class="detail">
    <div class="crt scan big" style="--c:{colorOf(selected.system)}">{#if covers[selected.path]}<img src={covers[selected.path]} alt="" />{:else}<span>{selected.title}</span>{/if}</div>
    <h2>{selected.title}</h2>
    <p class="muted">{sel.name} · {formatSize(selected.size)}</p>

    <dl>
      <dt class="label">Played</dt><dd>{formatPlaytime(selected.play_secs)} · {formatAgo(selected.last_played)}</dd>
      <dt class="label">Core</dt><dd class="mono">{sel.core}</dd>
      <dt class="label">File</dt><dd class="mono wrap">{selected.path}</dd>
      <dt class="label">BIOS</dt>
      <dd>
        {#each sel.bios.filter((b) => b.required) as b (b.file)}
          <div class={b.state === 'ok' ? 'ok' : 'bad'}>{b.state === 'ok' ? '✓' : '✕'} {b.file}</div>
        {:else}
          <span class="muted">none required</span>
        {/each}
      </dd>
    </dl>

    {#if selected.duplicate}
      <p class="dupe" title="Another file in the library has the same official title">⚠ Duplicate: another file has the same official title</p>
    {/if}
    <h3 class="label">Collections</h3>
    <div class="tags">
      {#each selected.tags ?? [] as t (t)}
        <span class="tag">{t}<button aria-label="Remove {t}" onclick={() => setTags(selected, selected.tags.filter((x) => x !== t))}>×</button></span>
      {/each}
      <input class="taginput" placeholder="Add to a collection…" bind:value={newTag} onkeydown={(e) => e.key === 'Enter' && addTag(selected)} aria-label="Add to a collection" />
    </div>

    <Cheats game={selected} />

    <SaveSlots game={selected} version={ov?.games.map((g) => g.last_played).join()} />

    <button class="btn primary wide" disabled={!!selected.blocked} onclick={() => play(selected)}>
      {selected.blocked ? `🔒 ${selected.blocked}` : '▶ Play'}
    </button>
    <button class="btn wide" onclick={() => fav(selected)}>{selected.favorite ? '★ Favorite' : '☆ Add to favorites'}</button>
    {#if selected.blocked}
      <button class="btn wide" onclick={() => (app.view = 'cores')}>Open Cores &amp; BIOS</button>
    {/if}
    {#if selected.imported}
      <button class="btn wide danger" onclick={() => remove(selected)}>Remove from library</button>
    {/if}
  </aside>
{/if}
{/if}

<style>
  .dupe { color: var(--warn, #ffb347); font-size: 12px; margin: 8px 0 0; }
  .tags { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 8px; }
  .tag { display: inline-flex; align-items: center; gap: 4px; background: var(--surface-2); border-radius: 999px; padding: 2px 4px 2px 10px; font-size: 12px; }
  .tag button { background: none; border: 0; color: var(--muted); padding: 0 4px; font-size: 14px; line-height: 1; }
  .tag button:hover { color: var(--text); }
  .taginput { flex: 1 1 120px; min-width: 100px; background: var(--bg); border: 1px solid var(--border); border-radius: 999px; padding: 3px 10px; color: var(--text); font: inherit; font-size: 12px; }
  .dropzone { position: fixed; inset: 12px; z-index: 20; display: grid; place-items: center; border: 2px dashed var(--accent); border-radius: 16px; background: color-mix(in srgb, var(--bg, #000) 80%, transparent); color: var(--accent); font-size: 18px; pointer-events: none; }
  .danger { color: var(--bad); }
  .main { flex: 1; min-width: 0; display: flex; flex-direction: column; padding: 16px 20px 0; gap: 14px; }
  .search {
    flex: 1 1 260px; min-width: 180px; max-width: 380px; display: flex; align-items: center; gap: 10px; height: 38px; padding: 0 10px 0 12px;
    background: var(--surface); border: 1px solid var(--border); border-radius: 12px; color: var(--faint);
    transition: border-color .15s, box-shadow .15s, color .15s;
  }
  .search:focus-within { border-color: var(--accent); color: var(--accent); box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 16%, transparent); }
  .search .mag { flex: none; }
  .search input { flex: 1; min-width: 0; background: none; border: 0; outline: none; padding: 0; color: var(--text); font: inherit; user-select: text; }
  .search input::placeholder { color: var(--muted); }
  .search kbd { font: 600 11px var(--font-mono); color: var(--faint); background: var(--bg); border: 1px solid var(--border); border-radius: 6px; padding: 2px 7px; flex: none; }
  .search .clear { flex: none; background: var(--surface-2); border: 0; color: var(--muted); width: 22px; height: 22px; border-radius: 50%; font-size: 11px; line-height: 1; padding: 0; }
  .search .clear:hover { color: var(--text); }
  .scroll { overflow: auto; flex: 1; min-height: 0; padding: 6px 8px 24px 6px; margin-left: -6px; }
  .row { display: flex; gap: 10px; flex-wrap: wrap; }
  .muted { color: var(--muted); margin: 4px 0 0; }
  .pad { padding: 24px 0; }

  .hero { display: flex; gap: 18px; background: var(--surface); border: 1px solid var(--border); border-radius: 10px; padding: 14px; }
  .hero-body { display: flex; flex-direction: column; gap: 4px; justify-content: center; }
  .hero-body h2 { font-size: 22px; }
  .hero-body .row { margin-top: 10px; }
  .crt {
    position: relative; overflow: hidden;
    width: 200px; height: 124px; border-radius: 6px; flex: none; display: grid; place-items: center; text-align: center;
    background: linear-gradient(160deg, color-mix(in srgb, var(--c) 28%, #0b0e16), #0b0e16 75%);
    border: 1px solid color-mix(in srgb, var(--c) 30%, var(--border));
  }
  .crt span, .cover span { font: 10px/1.6 var(--font-pixel); color: var(--c); padding: 8px; text-shadow: 0 0 8px color-mix(in srgb, var(--c) 60%, transparent); word-break: break-word; }
  .crt.big { width: 100%; height: 150px; }

  /* one toolbar: title, search, selection tools, sort and view. It stays put while the
     list scrolls under it. */
  .bar { display: flex; align-items: center; gap: 14px; position: sticky; top: -6px; z-index: 5; background: var(--bg); margin: 14px 0 4px; padding: 8px 0; flex-wrap: wrap; row-gap: 8px; }
  .bar h3 { flex: none; white-space: nowrap; }
  .bar h3 { font-size: 15px; }
  .bar h3 small { color: var(--faint); font: 12px var(--font-body); margin-left: 6px; }
  .sort { color: var(--muted); display: flex; gap: 4px; align-items: center; }
  .sort button { background: none; border: 0; padding: 2px 6px; color: var(--muted); border-radius: 4px; }
  .sort button.on { color: var(--accent); }
  .sort button:hover { color: var(--text); }

  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(190px, 1fr)); gap: 18px 16px; }
  .card { background: none; border: 0; padding: 0; text-align: center; display: flex; flex-direction: column; gap: 6px; }
  .cover {
    aspect-ratio: 4 / 3; border-radius: 8px; position: relative; overflow: hidden; display: grid; place-items: center; text-align: center;
    background: linear-gradient(170deg, color-mix(in srgb, var(--c) 24%, #0b0e16), #0b0e16 80%);
    border: 1px solid color-mix(in srgb, var(--c) 22%, var(--border)); transition: transform .12s, border-color .12s;
  }
  .crt img, .cover img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; border-radius: inherit; image-rendering: pixelated; }
  .cover img { transition: transform .25s; }
  .card:hover .cover img { transform: scale(1.04); }
  .card:hover .cover { transform: translateY(-2px); border-color: var(--c); }
  .card.sel .cover { border-color: var(--accent); box-shadow: 0 0 0 2px var(--accent); }
  .tools { display: flex; align-items: center; gap: 10px; margin-left: auto; font-size: 12px; }
  .selbar { display: flex; align-items: center; gap: 6px; background: var(--surface); border: 1px solid var(--border); border-radius: 11px; padding: 3px 4px 3px 12px; }
  .selbar .count { color: var(--muted); margin-right: 6px; }
  .selbar .count b { color: var(--accent); font-weight: 700; }
  .selbar button { border-color: transparent; background: none; }
  .selbar button.done { color: var(--accent); font-weight: 600; }
  .selbar button.done:hover { background: color-mix(in srgb, var(--accent) 14%, transparent); border-color: transparent; }
  .tools button { background: none; border: 1px solid var(--border); color: var(--muted); border-radius: 8px; padding: 4px 11px; font-size: 12px; }
  .tools button:hover:not(:disabled) { color: var(--text); border-color: var(--faint); }
  .tools button:disabled { opacity: .4; }
  .tools .danger-btn:not(:disabled) { color: var(--bad); background: color-mix(in srgb, var(--bad) 12%, transparent); }
  .tick { position: absolute; top: 8px; left: 8px; z-index: 2; width: 22px; height: 22px; border-radius: 6px; background: rgba(0,0,0,.55); border: 2px solid rgba(255,255,255,.7); color: #17110a; font: 700 13px/18px var(--font-body); display: grid; place-items: center; }
  .card.picked .tick { background: var(--accent); border-color: var(--accent); }
  .card.picked:not(.lrow) .cover { border-color: var(--accent); box-shadow: 0 0 0 2px var(--accent); }
  .card.locked { opacity: .35; }
  .layout { display: flex; gap: 2px; margin-left: 14px; background: var(--surface); border: 1px solid var(--border); border-radius: 9px; padding: 2px; }
  .layout button { background: none; border: 0; color: var(--faint); padding: 5px 8px; border-radius: 7px; display: grid; place-items: center; }
  .layout button:hover { color: var(--text); }
  .layout button.on { background: var(--surface-2); color: var(--accent); }

  .list { display: flex; flex-direction: column; gap: 2px; }
  .lhead, .lrow { display: grid; grid-template-columns: 76px minmax(0, 2.2fr) minmax(0, 1.1fr) 84px minmax(0, 1fr) minmax(0, .8fr) 34px; align-items: center; gap: 16px; }
  /* in selection mode a checkbox gutter opens to the left of the picture */
  .list.selecting .lhead, .list.selecting .lrow { grid-template-columns: 18px 76px minmax(0, 2.2fr) minmax(0, 1.1fr) 84px minmax(0, 1fr) minmax(0, .8fr) 34px; }
  .lhead { position: sticky; top: 56px; z-index: 3; height: 34px; padding: 0 14px 0 18px; background: var(--bg); border-bottom: 1px solid var(--border); color: var(--faint); font-size: 11px; text-transform: uppercase; letter-spacing: .07em; }
  .lhead .r, .lrow .r { text-align: right; justify-self: end; }
  .lrow { width: 100%; padding: 7px 14px 7px 18px; border-radius: 10px; border: 1px solid transparent; background: none; color: var(--text); text-align: left; transition: background .12s; }
  .lrow:hover { background: var(--surface); }
  .lrow.sel { background: var(--surface); border-color: color-mix(in srgb, var(--accent) 45%, var(--border)); }
  /* chosen rows: a soft tint and an accent bar, not a ring around every row */
  .lrow.picked { background: color-mix(in srgb, var(--accent) 6%, transparent); border-color: transparent; box-shadow: inset 3px 0 0 var(--accent); border-radius: 4px 10px 10px 4px; }
  .lrow.picked:hover { background: color-mix(in srgb, var(--accent) 11%, transparent); }
  .lrow.picked .cover, .lrow.picked:hover .cover { border-color: color-mix(in srgb, var(--c) 22%, var(--border)); box-shadow: none; }
  .lrow .thumb { display: block; width: 76px; height: 57px; aspect-ratio: auto; border-radius: 6px; position: relative; overflow: hidden; }
  .lrow .thumb img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; image-rendering: pixelated; }
  .lrow:hover .thumb img { transform: none; }
  .rname { display: flex; align-items: center; gap: 8px; min-width: 0; }
  .rsys { display: flex; align-items: center; gap: 8px; color: var(--muted); font-size: 13px; min-width: 0; white-space: nowrap; }
  .sico { display: inline-flex; flex: none; }
  .rplay { width: 30px; height: 30px; border-radius: 50%; display: grid; place-items: center; background: var(--accent); color: #17110a; opacity: 0; transform: scale(.85); transition: opacity .12s, transform .12s; padding-left: 2px; }
  .lrow:hover .rplay, .lrow.sel .rplay { opacity: 1; transform: none; }
  .rplay:hover { filter: brightness(1.1); }
  .rplay.off { visibility: hidden; }
  .rmeta { display: flex; align-items: center; gap: 7px; color: var(--muted); font-size: 12px; }
  .rmeta i { width: 7px; height: 7px; border-radius: 50%; flex: none; }
  .rmeta em { font-style: normal; color: var(--faint); }
  .rtick svg, .hcheck svg { display: block; flex: none; width: 11px; height: 11px; margin: 0; }
  .rname b { font-size: 14px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .rstar { color: var(--accent); font-style: normal; font-size: 12px; }
  .rlock { font-style: normal; font-size: 12px; }
  .rsys { display: flex; align-items: center; gap: 8px; color: var(--muted); font-size: 13px; }
  .rsys i { width: 8px; height: 8px; border-radius: 50%; flex: none; }
  .muted2 { color: var(--muted); font-size: 13px; }
  .rtick, .hcheck { width: 18px; height: 18px; border-radius: 5px; border: 1.5px solid var(--faint); display: grid; place-items: center; color: #17110a; padding: 0; background: transparent; transition: background .12s, border-color .12s; }
  .rtick.off { visibility: hidden; }
  .rtick.on, .hcheck.on { background: var(--accent); border-color: var(--accent); }
  .hcheck { cursor: pointer; }
  .hcheck.part { background: color-mix(in srgb, var(--accent) 30%, transparent); border-color: var(--accent); color: var(--accent); }
  .hcheck:hover { border-color: var(--accent); }

  .star { position: absolute; top: 6px; right: 8px; color: var(--accent); font-size: 14px; }
  .lock { position: absolute; bottom: 6px; right: 8px; font-size: 12px; }
  .title { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sub { color: var(--muted); font-size: 11px; margin-top: -4px; }

  .detail {
    width: 290px; flex: none; background: var(--surface); border-left: 1px solid var(--border);
    padding: 16px; display: flex; flex-direction: column; gap: 8px; overflow: auto;
  }
  .detail h2 { font-size: 20px; margin-top: 8px; }
  dl { margin: 10px 0; display: grid; gap: 2px; }
  dt { margin-top: 10px; }
  dd { margin: 0; }
  .wrap { word-break: break-all; user-select: text; }
  .ok { color: var(--ok); }
  .bad { color: var(--bad); }
  .wide { width: 100%; }
</style>
