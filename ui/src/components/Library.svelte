<script>
  import { app, play, refresh, importRoms, removeRom, covers } from '../lib/store.svelte.js';
  import Welcome from "./Welcome.svelte";
  import { api } from '../lib/api.js';
  import { colorOf, formatAgo, formatPlaytime, modKey } from '../lib/systems.js';

  import { onMount } from "svelte";
  let searchEl = $state();

  const ov = $derived(app.overview);
  const sysName = (id) => ov?.systems.find((s) => s.id === id)?.name ?? id;

  const visible = $derived.by(() => {
    let list = (ov?.games ?? []).filter((g) => ov.systems.find((s) => s.id === g.system)?.enabled !== false);
    if (app.system) list = list.filter((g) => g.system === app.system);
    if (app.filter === 'recent') list = list.filter((g) => g.last_played);
    if (app.filter === 'favorites') list = list.filter((g) => g.favorite);
    const q = app.query.trim().toLowerCase();
    if (q) list = list.filter((g) => g.title.toLowerCase().includes(q));
    const by = {
      recent: (a, b) => b.last_played - a.last_played || a.title.localeCompare(b.title),
      az: (a, b) => a.title.localeCompare(b.title),
      size: (a, b) => b.size - a.size,
    }[app.sort];
    return [...list].sort(by);
  });

  const hero = $derived(
    [...(ov?.games ?? [])].filter((g) => g.last_played).sort((a, b) => b.last_played - a.last_played)[0],
  );
  const selected = $derived(ov?.games.find((g) => g.path === app.selected) ?? null);
  const sel = $derived(selected ? ov.systems.find((s) => s.id === selected.system) : null);
  const heading = $derived(
    app.system ? sysName(app.system) : { all: 'All games', recent: 'Recently played', favorites: 'Favorites' }[app.filter],
  );

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
  function remove(g) {
    if (confirm(`Remove "${g.title}" from the library?\nThe copy inside vuretro is deleted; your original file is not touched.`)) removeRom(g);
  }

  /** Move the selection through the grid (dx: left/right, dy: up/down). */
  function move(dx, dy) {
    const cards = [...document.querySelectorAll('.grid .card')];
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
      app.query = '';
      searchEl?.blur();
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
  async function fav(g) {
    await api.toggleFavorite(g.path);
    refresh();
  }
  const mb = (n) => (n >= 1e9 ? `${(n / 1e9).toFixed(1)} GB` : `${Math.max(1, Math.round(n / 1e6))} MB`);
</script>

<svelte:window onkeydown={onkey} />

{#if !ov || ov.games.length === 0}
  <Welcome />
{:else}
{#if over}<div class="dropzone">Drop ROMs to add them to the library</div>{/if}
<section class="main">
  <div class="top">
    <input id="search" bind:this={searchEl} bind:value={app.query} placeholder="Search games…  ({modKey} K)" aria-label="Search games" />
    <span class="summary">{ov.games.length} games · {ov.systems.filter((s) => s.game_count).length} systems</span>
  </div>

  <div class="scroll">
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
        <div class="sort">
          Sort:
          {#each [['recent', 'Recent'], ['az', 'A–Z'], ['size', 'Size']] as [k, name] (k)}
            <button class:on={app.sort === k} onclick={() => (app.sort = k)}>{name}</button>
          {/each}
        </div>
      </div>

      {#if visible.length === 0}
        <p class="muted pad">Nothing matches.</p>
      {:else}
        <div class="grid">
          {#each visible as g (g.path)}
            <button class="card" class:sel={app.selected === g.path} onclick={() => (app.selected = g.path)} ondblclick={() => play(g)}>
              <div class="cover scan" style="--c:{colorOf(g.system)}">
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
    <p class="muted">{sel.name} · {mb(selected.size)}</p>

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
  .dropzone { position: fixed; inset: 12px; z-index: 20; display: grid; place-items: center; border: 2px dashed var(--accent); border-radius: 16px; background: color-mix(in srgb, var(--bg, #000) 80%, transparent); color: var(--accent); font-size: 18px; pointer-events: none; }
  .danger { color: var(--bad); }
  .main { flex: 1; min-width: 0; display: flex; flex-direction: column; padding: 16px 20px 0; gap: 14px; }
  .top { display: flex; align-items: center; justify-content: space-between; gap: 16px; }
  .summary { color: var(--faint); white-space: nowrap; }
  .top input {
    flex: 1; max-width: 460px; background: var(--surface-2); border: 1px solid var(--border);
    border-radius: 20px; padding: 8px 16px; user-select: text;
  }
  .top input::placeholder { color: var(--muted); }
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

  .bar { display: flex; align-items: center; justify-content: space-between; margin: 20px 0 12px; }
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
