<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { app } from '../lib/store.svelte.js';
  import { toast } from '../lib/ui.svelte.js';
  import { ACTIONS, RESERVED, keyName, keyLabel, padLabel } from '../lib/controls.js';
  import Select from './Select.svelte';

  let cfg = $state(null);
  let controls = $state(null);
  let scope = $state(''); // '' = every game, otherwise a game key
  let saved = $state(false);
  let capture = $state(null); // { id, kind: 'key' | 'pad' }
  let rec = $state(null); // { id, held: string[], last: number, started: boolean }
  let saveTimer;
  let savedTimer;

  const KINDS = [
    { id: 'combo', name: 'Combo', hint: 'Hold the key to press several buttons together' },
    { id: 'turbo', name: 'Turbo', hint: 'Hold the key to tap the buttons again and again' },
    { id: 'sequence', name: 'Sequence', hint: 'Press the key to play buttons one after another' },
  ];

  const gameKey = (path) => {
    const name = path.split(/[\\/]/).pop();
    const i = name.lastIndexOf('.');
    return (i > 0 ? name.slice(0, i) : name).toLowerCase();
  };
  const games = $derived.by(() => {
    const seen = new Map();
    for (const g of app.overview?.games ?? []) seen.set(gameKey(g.path), g.title);
    return [...seen].map(([key, title]) => ({ key, title })).sort((a, b) => a.title.localeCompare(b.title));
  });
  // Only reads: Svelte forbids changing state inside a $derived, and choosing a game that has no
  // macros yet used to do exactly that, which broke the whole screen.
  const NONE = [];
  const list = $derived(cfg ? (scope ? (cfg.games[scope] ?? NONE) : cfg.global) : NONE);
  const scopeName = $derived(scope ? (games.find((g) => g.key === scope)?.title ?? scope) : 'every game');

  $effect(() => {
    app.capturing = !!capture || !!rec;
    return () => (app.capturing = false);
  });

  onMount(() => {
    (async () => {
      cfg = await api.macros();
      controls = (await api.controls()).config;
      const sel = app.overview?.games.find((g) => g.path === app.selected);
      if (sel) scope = gameKey(sel.path);
    })();
    let off = () => {};
    api.onPadButton((p) => {
      if (capture?.kind === 'pad') {
        const m = list.find((x) => x.id === capture.id);
        if (m) m.pad = p.button;
        capture = null;
        save();
      }
    }).then((f) => (off = f));
    return () => off();
  });

  function save() {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(async () => {
      try {
        const out = await api.setMacros($state.snapshot(cfg));
        cfg = out;
        saved = true;
        clearTimeout(savedTimer);
        savedTimer = setTimeout(() => (saved = false), 1600);
      } catch (e) {
        toast(String(e), 'error');
      }
    }, 250);
  }

  function add(kind) {
    const uid = crypto?.randomUUID?.() ?? String(Math.random()).slice(2);
    const k = KINDS.find((x) => x.id === kind);
    // A game gets its own list the first time something is added to it. Read it back after
    // assigning: `x ??= []` gives the plain array, not the copy the state keeps, and a macro
    // pushed onto the plain one is lost.
    if (scope && !cfg.games[scope]) cfg.games[scope] = [];
    const target = scope ? cfg.games[scope] : cfg.global;
    target.push({
      id: uid, name: k.name, enabled: true, player: 0, key: null, pad: null, kind,
      actions: [], rate_hz: 15, steps: kind === 'sequence' ? [{ actions: [], ms: 100 }] : [], repeat: false,
    });
    // the next thing to do is choose the trigger, so wait for that key right away
    capture = { id: uid, kind: 'key' };
    save();
  }
  function remove(m) {
    const i = list.indexOf(m);
    if (i >= 0) list.splice(i, 1);
    if (scope && !list.length) delete cfg.games[scope];
    save();
  }
  const has = (arr, a) => arr.includes(a);
  function toggle(arr, a) {
    const i = arr.indexOf(a);
    if (i >= 0) arr.splice(i, 1);
    else arr.push(a);
    save();
  }
  function addStep(m) {
    m.steps.push({ actions: [], ms: 100 });
    save();
  }
  function removeStep(m, i) {
    m.steps.splice(i, 1);
    save();
  }
  const totalMs = (m) => m.steps.reduce((t, s) => t + (+s.ms || 0), 0);

  // ---- recording a sequence from the keyboard (uses the player's own key bindings) ----
  function actionOf(code, player) {
    const name = keyName(code);
    if (!name || !controls) return null;
    const kb = controls.players[player]?.keyboard ?? {};
    return Object.keys(kb).find((a) => kb[a].includes(name)) ?? null;
  }
  function startRec(m) {
    capture = null; // recording a move and choosing a trigger key are different things
    m.steps = [];
    rec = { id: m.id, held: [], last: performance.now(), started: false };
  }
  function pushStep(m, now) {
    if (!rec.started) return;
    const ms = Math.max(10, Math.round(now - rec.last));
    m.steps.push({ actions: ACTIONS.map((a) => a.id).filter((a) => rec.held.includes(a)), ms });
    rec.last = now;
  }
  function stopRec() {
    const m = list.find((x) => x.id === rec?.id);
    if (m && rec.started) {
      pushStep(m, performance.now());
      // drop a trailing pause
      while (m.steps.length && !m.steps.at(-1).actions.length) m.steps.pop();
    }
    rec = null;
    save();
  }

  function onkeydown(e) {
    if (rec) {
      e.preventDefault();
      if (e.code === 'Escape') return stopRec();
      const m = list.find((x) => x.id === rec.id);
      const a = m && actionOf(e.code, m.player);
      if (!a || rec.held.includes(a)) return;
      const now = performance.now();
      pushStep(m, now);
      rec.held = [...rec.held, a];
      rec.started = true;
      rec.last = now;
      return;
    }
    if (!capture || capture.kind !== 'key') return;
    e.preventDefault();
    if (e.code === 'Escape') return (capture = null);
    if (RESERVED.includes(e.code)) return toast(`${e.code} is reserved for the game hotkeys`, 'error');
    const name = keyName(e.code);
    if (!name) return toast(`That key cannot be used (${e.code})`, 'error');
    const m = list.find((x) => x.id === capture.id);
    if (m) m.key = name;
    capture = null;
    save();
  }
  function onkeyup(e) {
    if (!rec) return;
    const m = list.find((x) => x.id === rec.id);
    const a = m && actionOf(e.code, m.player);
    if (!a || !rec.held.includes(a)) return;
    const now = performance.now();
    pushStep(m, now);
    rec.held = rec.held.filter((x) => x !== a);
    rec.last = now;
  }
</script>

<svelte:window {onkeydown} {onkeyup} />

<section class="page">
  <div class="col">
    <header class="head">
      <div>
        <h1>Macros</h1>
        <p class="sub">One key or button that does more: several buttons at once, rapid fire, or a whole move. Each game can have its own.</p>
      </div>
      <span class="saved" class:show={saved} aria-live="polite">✓ Saved</span>
    </header>

    {#if cfg}
      <div class="scope">
        <span class="slabel">Macros for</span>
        <div class="scopesel">
          <Select bind:value={scope} searchable ariaLabel="Macros for" options={[{ value: '', label: 'Every game', hint: 'global' }, ...games.map((g) => ({ value: g.key, label: g.title }))]} />
        </div>
        <span class="muted">{scope ? 'Played together with the macros for every game.' : 'Available in all games.'}</span>
      </div>

      {#each list as m (m.id)}
        <article class="card" class:off={!m.enabled}>
          <header class="mh">
            <span class="badge {m.kind}">{KINDS.find((k) => k.id === m.kind)?.name}</span>
            <input class="name" bind:value={m.name} oninput={save} maxlength="60" aria-label="Macro name" />
            <button class="switch" role="switch" aria-checked={m.enabled} aria-label="Enabled" class:on={m.enabled} onclick={() => { m.enabled = !m.enabled; save(); }}><span></span></button>
            <button class="del" onclick={() => remove(m)} aria-label="Delete macro" title="Delete">✕</button>
          </header>

          <div class="trig">
            <span class="lbl">Trigger</span>
            <span class="chips">
              {#if capture?.id === m.id && capture.kind === 'key'}
                <span class="chip waiting">Press a key… <small>Esc cancels</small></span>
              {:else if m.key}
                <span class="chip">{keyLabel(m.key)}<button aria-label="Remove key" onclick={() => { m.key = null; save(); }}>×</button></span>
              {:else}
                <button class="add" onclick={() => (capture = { id: m.id, kind: 'key' })}>+ Key</button>
              {/if}
              {#if capture?.id === m.id && capture.kind === 'pad'}
                <span class="chip waiting">Press a button…</span>
              {:else if m.pad}
                <span class="chip">{padLabel(m.pad)}<button aria-label="Remove button" onclick={() => { m.pad = null; save(); }}>×</button></span>
              {:else}
                <button class="add" onclick={() => (capture = { id: m.id, kind: 'pad' })}>+ Gamepad button</button>
              {/if}
            </span>
            <span class="pl">Player
              <Select compact bind:value={m.player} onchange={save} ariaLabel="Player" options={[0, 1, 2, 3].map((p) => ({ value: p, label: String(p + 1) }))} minWidth={72} />
            </span>
          </div>

          {#if m.kind !== 'sequence'}
            <div class="acts">
              <span class="lbl">{m.kind === 'turbo' ? 'Taps' : 'Presses'}</span>
              <span class="pad">
                {#each ACTIONS as a (a.id)}
                  <button class:on={has(m.actions, a.id)} onclick={() => toggle(m.actions, a.id)} title={a.hint}>{a.label}</button>
                {/each}
              </span>
            </div>
            {#if m.kind === 'turbo'}
              <div class="acts">
                <span class="lbl">Speed</span>
                <span class="speed">
                  <input type="range" min="2" max="30" step="1" bind:value={m.rate_hz} onchange={save} style="--pct:{((m.rate_hz - 2) / 28) * 100}%" aria-label="Taps per second" />
                  <b class="mono">{m.rate_hz} / sec</b>
                </span>
              </div>
            {/if}
          {:else}
            <div class="steps">
              <div class="shead">
                <span class="lbl">Steps <small>{m.steps.length} · {(totalMs(m) / 1000).toFixed(2)} s</small></span>
                <span class="srow">
                  {#if rec?.id === m.id}
                    <span class="recnote">● Recording… play the move with your keys <small>Esc stops</small></span>
                    <button class="btn small" onclick={stopRec}>Stop</button>
                  {:else}
                    <button class="btn small" onclick={() => startRec(m)} disabled={!!rec}>● Record</button>
                    <button class="btn small" onclick={() => addStep(m)}>+ Step</button>
                  {/if}
                </span>
              </div>
              {#each m.steps as st, i (i)}
                <div class="step">
                  <span class="n mono">{i + 1}</span>
                  <span class="pad small">
                    {#each ACTIONS as a (a.id)}
                      <button class:on={has(st.actions, a.id)} onclick={() => toggle(st.actions, a.id)} title={a.hint}>{a.label}</button>
                    {/each}
                  </span>
                  <label class="ms"><input type="number" min="10" max="10000" step="10" bind:value={st.ms} onchange={save} aria-label="Milliseconds" /> ms</label>
                  <button class="del" onclick={() => removeStep(m, i)} aria-label="Remove step">✕</button>
                </div>
              {/each}
              <label class="check"><input type="checkbox" bind:checked={m.repeat} onchange={save} /> Keep repeating while the trigger is held</label>
            </div>
          {/if}
        </article>
      {:else}
        <div class="empty">
          <strong>No macros for {scopeName} yet</strong>
          <span>Add one below. Pick what it should do.</span>
        </div>
      {/each}

      <div class="addrow">
        {#each KINDS as k (k.id)}
          <button class="newk" onclick={() => add(k.id)}>
            <b>+ {k.name}</b>
            <small>{k.hint}</small>
          </button>
        {/each}
      </div>
      <p class="foot">Macros load when a game starts. Keys that games use as hotkeys (Esc, Tab, P, F2 to F10) cannot be triggers.</p>
    {/if}
  </div>
</section>

<style>
  .page { flex: 1; overflow: auto; padding: 40px 32px 64px; min-width: 0; }
  .col { max-width: 860px; margin: 0 auto; display: flex; flex-direction: column; gap: 16px; }
  .head { display: flex; justify-content: space-between; align-items: flex-start; gap: 24px; }
  h1 { font-size: 30px; letter-spacing: -.015em; }
  .sub { color: var(--muted); margin: 8px 0 0; max-width: 62ch; }
  .saved { color: var(--ok); font-size: 13px; opacity: 0; transition: opacity .25s; padding: 6px 12px; border-radius: 999px; background: color-mix(in srgb, var(--ok) 12%, transparent); }
  .saved.show { opacity: 1; }
  .muted { color: var(--muted); font-size: 13px; }

  .scope { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; background: var(--surface); border: 1px solid var(--border); border-radius: 14px; padding: 12px 16px; }
  .slabel { font-weight: 600; font-size: 13px; }
  select, input[type='number'], .name {
    background: var(--bg); border: 1px solid var(--border); border-radius: 9px; padding: 7px 10px; color: var(--text); font: inherit; font-size: 13px;
  }
  .scopesel { width: 280px; }

  .card { background: var(--surface); border: 1px solid var(--border); border-radius: 16px; padding: 16px 18px; display: flex; flex-direction: column; gap: 14px; }
  .card.off { opacity: .55; }
  .mh { display: flex; align-items: center; gap: 12px; }
  .badge { font: 700 11px var(--font-body); text-transform: uppercase; letter-spacing: .07em; padding: 4px 9px; border-radius: 999px; flex: none; }
  .badge.combo { color: var(--accent); background: color-mix(in srgb, var(--accent) 16%, transparent); }
  .badge.turbo { color: var(--brand-magenta); background: color-mix(in srgb, var(--brand-magenta) 16%, transparent); }
  .badge.sequence { color: #6cc8ff; background: color-mix(in srgb, #6cc8ff 16%, transparent); }
  .name { flex: 1; min-width: 0; font-weight: 600; font-size: 15px; background: transparent; border-color: transparent; }
  .name:hover, .name:focus { border-color: var(--border); background: var(--bg); outline: none; }
  .del { background: none; border: 0; color: var(--faint); font-size: 14px; padding: 4px 8px; border-radius: 6px; }
  .del:hover { color: var(--bad); background: color-mix(in srgb, var(--bad) 12%, transparent); }

  .trig, .acts { display: flex; align-items: center; gap: 14px; flex-wrap: wrap; }
  .lbl { width: 64px; flex: none; color: var(--muted); font-size: 12px; text-transform: uppercase; letter-spacing: .07em; }
  .lbl small { display: block; text-transform: none; letter-spacing: 0; color: var(--faint); margin-top: 2px; }
  .chips { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; flex: 1; }
  .chip { display: inline-flex; align-items: center; gap: 6px; background: var(--surface-2); border: 1px solid var(--border); border-radius: 8px; padding: 4px 4px 4px 10px; font: 12px var(--font-mono); }
  .chip button { background: none; border: 0; color: var(--faint); padding: 0 6px; font-size: 14px; }
  .chip button:hover { color: var(--bad); }
  .chip.waiting { border-color: var(--accent); color: var(--accent); padding: 4px 10px; animation: pulse 1s infinite; }
  .chip.waiting small { color: var(--faint); margin-left: 6px; }
  .add { background: none; border: 1px dashed var(--border); color: var(--muted); border-radius: 8px; padding: 4px 11px; font-size: 12px; }
  .add:hover { color: var(--text); border-color: var(--faint); }
  .pl { display: flex; align-items: center; gap: 8px; color: var(--muted); font-size: 12px; }

  .pad { display: flex; flex-wrap: wrap; gap: 6px; flex: 1; }
  .pad button { min-width: 38px; padding: 6px 8px; background: var(--bg); border: 1px solid var(--border); border-radius: 8px; color: var(--muted); font: 600 12px var(--font-body); }
  .pad button:hover { color: var(--text); border-color: var(--faint); }
  .pad button.on { background: color-mix(in srgb, var(--accent) 18%, var(--bg)); border-color: var(--accent); color: var(--accent); }
  .pad.small button { min-width: 30px; padding: 4px 6px; font-size: 11px; }
  .speed { display: flex; align-items: center; gap: 14px; flex: 1; }
  .speed b { min-width: 74px; text-align: right; font-size: 13px; }
  input[type='range'] { -webkit-appearance: none; appearance: none; flex: 1; height: 6px; border-radius: 3px; outline: none; background: linear-gradient(to right, var(--accent) var(--pct), var(--bg) var(--pct)); box-shadow: 0 0 0 1px var(--border) inset; }
  input[type='range']::-webkit-slider-thumb { -webkit-appearance: none; width: 18px; height: 18px; border-radius: 50%; background: #fff; box-shadow: 0 2px 6px rgba(0,0,0,.5); cursor: pointer; }

  .steps { display: flex; flex-direction: column; gap: 8px; }
  .shead { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .shead .lbl { width: auto; }
  .srow { display: flex; align-items: center; gap: 8px; }
  .recnote { color: var(--bad); font-size: 12px; font-weight: 600; animation: pulse 1s infinite; }
  .recnote small { color: var(--faint); font-weight: 400; margin-left: 6px; }
  .step { display: flex; align-items: center; gap: 12px; background: var(--bg); border: 1px solid var(--border); border-radius: 12px; padding: 8px 10px; }
  .step .n { width: 20px; color: var(--faint); font-size: 12px; text-align: center; }
  .step .pad { gap: 4px; }
  .ms { display: flex; align-items: center; gap: 6px; color: var(--muted); font-size: 12px; flex: none; }
  .ms input { width: 76px; padding: 5px 8px; }
  .check { display: flex; align-items: center; gap: 8px; color: var(--muted); font-size: 13px; margin-top: 4px; }
  .check input { accent-color: var(--accent); }
  .btn.small { padding: 5px 12px; font-size: 12px; }

  .switch { width: 44px; height: 26px; border-radius: 13px; border: 1px solid var(--border); background: var(--bg); position: relative; flex: none; padding: 0; }
  .switch span { position: absolute; top: 3px; left: 3px; width: 18px; height: 18px; border-radius: 50%; background: var(--muted); transition: transform .2s; }
  .switch.on { background: var(--accent); border-color: var(--accent); }
  .switch.on span { transform: translateX(18px); background: #fff; }

  .empty { display: flex; flex-direction: column; gap: 4px; text-align: center; padding: 34px 20px; background: var(--surface); border: 1px dashed var(--border); border-radius: 16px; color: var(--muted); }
  .empty strong { color: var(--text); }
  .addrow { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }
  .newk { display: flex; flex-direction: column; align-items: flex-start; gap: 4px; text-align: left; background: var(--surface); border: 1px solid var(--border); border-radius: 14px; padding: 14px 16px; color: var(--text); transition: border-color .15s, transform .1s; }
  .newk b { font-size: 14px; }
  .newk small { color: var(--muted); font-size: 12px; line-height: 1.4; }
  .newk:hover { border-color: var(--accent); }
  .newk:active { transform: scale(.98); }
  .foot { color: var(--faint); font-size: 12px; margin: 0; }
  @media (max-width: 820px) { .addrow { grid-template-columns: 1fr; } }
</style>
