<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { app } from '../lib/store.svelte.js';
  import { toast } from '../lib/ui.svelte.js';
  import { modKey } from '../lib/systems.js';
  import Select from './Select.svelte';
  import ScopeBar from './ScopeBar.svelte';
  import { parseScope } from '../lib/scope.js';
  import { ACTIONS, DIAGONALS, HOTKEYS, PAD_HOTKEYS, RESERVED, keyName, keyLabel, padLabel, deviceValue, parseDevice } from '../lib/controls.js';

  let cfg = $state(null);
  /** this system has key bindings of its own (otherwise it uses the general ones) */
  let custom = $state(false);
  let pads = $state([]);
  let tab = $state(0);
  let capture = $state(null); // { player, kind: 'keyboard'|'gamepad', action }
  let saved = $state(false);
  let lastPress = $state({}); // pad id -> button, for "identify my pad"
  let timers = {};

  $effect(() => {
    app.capturing = !!capture;
    return () => (app.capturing = false);
  });

  const shortId = (id) => id.slice(0, 8);

  const scope = $derived(parseScope(app.scope));

  async function load() {
    const v = scope.kind === 'game' ? await api.controlsForGame(scope.id) : await api.controlsFor(app.scope);
    cfg = v.config;
    pads = v.pads;
    custom = !!v.custom;
  }

  // the screen shows the bindings of whatever the scope bar says
  $effect(() => {
    void app.scope;
    load();
  });

  async function useGeneral() {
    const v = scope.kind === 'game' ? await api.resetControlsForGame(scope.id) : await api.resetControlsFor(scope.id);
    cfg = v.config;
    custom = false;
    toast(`${scopeName} follows ${scope.kind === 'game' ? "its system's" : "the general"} key bindings again`);
  }
  const scopeName = $derived(
    scope.kind === 'game'
      ? (app.overview?.games.find((g) => g.path === scope.id)?.title ?? scope.id)
      : (app.overview?.systems.find((x) => x.id === scope.id)?.name ?? scope.id),
  );

  let saveTimer;
  async function save() {
    try {
      if (scope.kind !== 'all') {
        const snap = $state.snapshot(cfg);
        const v = scope.kind === 'game' ? await api.setControlsForGame(scope.id, snap) : await api.setControlsFor(scope.id, snap);
        cfg = v.config;
        custom = true;
      } else {
        cfg = await api.setControls($state.snapshot(cfg));
      }
      saved = true;
      clearTimeout(saveTimer);
      saveTimer = setTimeout(() => (saved = false), 1800);
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  function swap() {
    const [a, b] = [cfg.players[0].device, cfg.players[1].device];
    cfg.players[0].device = b;
    cfg.players[1].device = a;
    save();
  }
  const nth = (n) => ['first', 'second', 'third', 'fourth'][n] ?? `#${n + 1}`;
  function setDevice(i, value) {
    cfg.players[i].device = parseDevice(value);
    save();
  }
  function removeBinding(i, kind, action, value) {
    cfg.players[i][kind][action] = (cfg.players[i][kind][action] ?? []).filter((v) => v !== value);
    save();
  }
  function addBinding(i, kind, action, value) {
    // A key or button belongs to one action per player: take it away from the others.
    for (const a of Object.keys(cfg.players[i][kind])) {
      cfg.players[i][kind][a] = cfg.players[i][kind][a].filter((v) => v !== value);
    }
    cfg.players[i][kind][action] = [...(cfg.players[i][kind][action] ?? []), value];
    save();
  }
  async function resetPlayer(i) {
    const defaults = await api.defaultControls();
    cfg.players[i] = defaults.players[i];
    await save();
    toast(`Player ${i + 1} reset to defaults`);
  }

  function startCapture(player, kind, action) {
    capture = { player, kind, action };
  }

  function onkeydown(e) {
    if (!capture) return;
    e.preventDefault();
    if (e.code === 'Escape') return (capture = null);
    if (capture.kind !== 'keyboard') return;
    if (RESERVED.includes(e.code)) {
      return toast(`${e.code} is reserved for the game hotkeys`, 'error');
    }
    const name = keyName(e.code);
    if (!name) return toast(`That key cannot be bound (${e.code})`, 'error');
    addBinding(capture.player, 'keyboard', capture.action, name);
    capture = null;
  }

  function describe(i) {
    const d = cfg.players[i].device;
    const pad = (id) => pads.find((p) => p.id === id);
    if (d.kind === 'none') return 'Disabled';
    if (d.kind === 'keyboard') return 'Keyboard only';
    if (d.kind === 'gamepad') return pad(d.id) ? pad(d.id).name : 'Gamepad not connected';
    // auto
    if (i === 0) return `Keyboard${pads[0] ? ' + ' + pads[0].name : ''}`;
    if (pads[i]) return pads[i].name;
    if (i === 1 && pads[0] && cfg.players[0].device.kind !== 'auto') return pads[0].name;
    return 'Waiting for a gamepad';
  }

  onMount(() => {
    const offs = [];
    api.onPadsChanged((list) => (pads = list)).then((f) => offs.push(f));
    api
      .onPadButton((p) => {
        lastPress[p.id] = p.button;
        clearTimeout(timers[p.id]);
        timers[p.id] = setTimeout(() => delete lastPress[p.id], 1500);
        if (capture?.kind === 'gamepad') {
          addBinding(capture.player, 'gamepad', capture.action, p.button);
          capture = null;
        }
      })
      .then((f) => offs.push(f));
    return () => offs.forEach((f) => f());
  });
</script>

<svelte:window {onkeydown} />

<section class="page">
  <header class="head">
    <div>
      <h1>Controls</h1>
      <p class="sub">Choose who plays with what, and set up every button. Changes save instantly and apply the next time a game starts.</p>
    </div>
    <span class="saved" class:show={saved}>✓ Saved</span>
  </header>

  <ScopeBar hint="Each system can have its own buttons: a NES pad and a PlayStation pad are not alike." />
  {#if scope.kind !== 'all'}
    {@const parent = scope.kind === 'game' ? "its system's (or the general)" : 'the general'}
    <p class="scopenote">
      {#if custom}
        <strong>{scopeName}</strong> has its own key bindings. <button class="link" onclick={useGeneral}>Use {parent} ones</button>
      {:else}
        <strong>{scopeName}</strong> uses {parent} key bindings. Change any button below to give it bindings of its own.
      {/if}
    </p>
  {/if}

  {#if cfg}
    <h2 class="sec">Players</h2>
    <div class="players">
      {#each cfg.players as p, i (i)}
        <article class="card">
          <header>
            <h3>Player {i + 1}</h3>
            <span class="muted">{describe(i)}</span>
          </header>
          <label class="field">
            <span class="label">Device</span>
            <Select
              value={deviceValue(p.device)}
              onchange={(v) => setDevice(i, v)}
              ariaLabel="Device for player {i + 1}"
              options={[
                { value: 'auto', label: `Auto ${i === 0 ? '(keyboard + first gamepad)' : '(' + nth(i) + ' free gamepad)'}` },
                { value: 'keyboard', label: 'Keyboard only' },
                ...pads.map((pad) => ({ value: 'gamepad:' + pad.id, label: pad.name })),
                ...(p.device.kind === 'gamepad' && !pads.some((x) => x.id === p.device.id) ? [{ value: 'gamepad:' + p.device.id, label: `Gamepad ${shortId(p.device.id)} (not connected)` }] : []),
                { value: 'none', label: 'Disabled' },
              ]}
            />
          </label>
          <label class="field">
            <span class="label">Stick dead zone <b>{Math.round(p.deadzone * 100)}%</b></span>
            <input type="range" min="0.05" max="0.9" step="0.05" bind:value={p.deadzone} onchange={save} />
          </label>
          <label class="check">
            <input type="checkbox" bind:checked={p.stick_as_dpad} onchange={save} />
            Left stick works as the D-pad
          </label>
        </article>
      {/each}
    </div>
    <!-- a block, so it lines up with the cards (a bare inline button ignores the column's auto margins) -->
    <div class="swaprow">
      <button class="btn small" onclick={swap} title="Swap the devices of player 1 and player 2">⇄ Swap player 1 and 2</button>
    </div>

    <h2 class="sec">Connected gamepads</h2>
    {#if pads.length === 0}
      <div class="card empty">
        <strong>No gamepad detected</strong>
        <span class="muted">Plug one in or pair it over Bluetooth, then press a button. Xbox, DualSense and Switch Pro pads work.</span>
      </div>
    {:else}
      <div class="pads">
        {#each pads as pad, n (pad.id + n)}
          <div class="pad" class:active={lastPress[pad.id]}>
            <span class="dot"></span>
            <div>
              <div class="pname">{pad.name}</div>
              <div class="muted mono">{shortId(pad.id)}</div>
            </div>
            <span class="press mono">{lastPress[pad.id] ? padLabel(lastPress[pad.id]) : 'press a button to identify'}</span>
          </div>
        {/each}
      </div>
    {/if}

    <h2 class="sec">Button mapping</h2>
    <div class="card map">
      <div class="tabs" role="tablist">
        {#each cfg.players as _, i (i)}
          <button role="tab" aria-selected={tab === i} class:on={tab === i} onclick={() => ((tab = i), (capture = null))}>Player {i + 1}</button>
        {/each}
        <span class="grow"></span>
        <button class="btn small" onclick={() => resetPlayer(tab)}>Reset player {tab + 1}</button>
      </div>

      <table>
        <thead>
          <tr><th>Button</th><th>Keyboard</th><th>Gamepad</th></tr>
        </thead>
        <tbody>
          {#each [...ACTIONS, ...DIAGONALS] as a (a.id)}
            {#if a.id === 'up_left'}
              <tr class="sep">
                <td colspan="3">
                  <div class="act">Diagonal keys <small>keyboard only</small></div>
                  <div class="hint">One key for two directions at once, for a keyboard without a numpad. Pick keys you can reach with your arrow hand. A gamepad uses its D-pad.</div>
                </td>
              </tr>
            {/if}
            <tr>
              <td>
                <div class="act">{a.label}</div>
                <div class="hint">{a.hint}</div>
              </td>
              {#each (DIAGONALS.some((d) => d.id === a.id) ? [['keyboard', keyLabel]] : [['keyboard', keyLabel], ['gamepad', padLabel]]) as [kind, fmt] (kind)}
                <td>
                  <div class="chips">
                    {#each cfg.players[tab][kind][a.id] ?? [] as v (v)}
                      <span class="chip">{fmt(v)}<button aria-label="Remove {fmt(v)}" onclick={() => removeBinding(tab, kind, a.id, v)}>×</button></span>
                    {/each}
                    {#if capture && capture.player === tab && capture.kind === kind && capture.action === a.id}
                      <span class="chip waiting">{kind === 'keyboard' ? 'Press a key…' : 'Press a button…'} <small>Esc cancels</small></span>
                    {:else}
                      <button class="add" onclick={() => startCapture(tab, kind, a.id)}>+ {(cfg.players[tab][kind][a.id] ?? []).length ? 'Add' : 'Set'}</button>
                    {/if}
                  </div>
                </td>
              {/each}
              {#if DIAGONALS.some((d) => d.id === a.id)}<td class="muted">uses the D-pad</td>{/if}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <h2 class="sec">Hotkeys in a game</h2>
    <div class="keys">
      {#each HOTKEYS as h (h.what)}
        <div class="key">{#each h.keys as k (k)}<kbd>{k}</kbd>{/each}{#if h.also}<span class="or">or</span>{#each h.also as k (k)}<kbd>{k}</kbd>{/each}{/if}<span>{h.what}</span></div>
      {/each}
    </div>
    <p class="foot">These keys are fixed and cannot be used as buttons. Library search: <kbd>{modKey}</kbd> <kbd>K</kbd>.</p>

    <h2 class="sec">Gamepad shortcuts in a game</h2>
    <div class="keys">
      {#each PAD_HOTKEYS as h (h.what)}
        <div class="key">{#each h.keys as k (k)}<kbd>{k}</kbd>{/each}{#if h.also}<span class="or">or</span>{#each h.also as k (k)}<kbd>{k}</kbd>{/each}{/if}<span>{h.what}</span></div>
      {/each}
    </div>
    <p class="foot">Hold <kbd>Select</kbd> and press the other button. While you do, the game does not see those presses.</p>
  {/if}
</section>

<style>
  .scopenote { color: var(--muted); margin: -8px 0 18px; }
  .scopenote .link { background: none; border: 0; color: var(--accent); text-decoration: underline; padding: 0; font: inherit; }
  .page { flex: 1; overflow: auto; padding: 32px 40px 56px; min-width: 0; }
  .page > :global(*) { max-width: 1040px; margin-left: auto; margin-right: auto; }
  .head { display: flex; justify-content: space-between; align-items: flex-start; gap: 24px; }
  h1 { font-size: 26px; }
  .sub { color: var(--muted); margin: 6px 0 0; max-width: 60ch; }
  .saved { color: var(--ok); opacity: 0; transition: opacity .25s; padding-top: 8px; }
  .saved.show { opacity: 1; }
  .sec { font-size: 13px; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); margin: 34px auto 12px; font-weight: 600; }
  .card { background: var(--surface); border: 1px solid var(--border); border-radius: 12px; padding: 18px; }
  .muted { color: var(--muted); }

  .players { display: grid; grid-template-columns: repeat(2, 1fr); gap: 12px; align-items: stretch; }
  .swaprow { margin-top: 12px; }
  .players .card { display: flex; flex-direction: column; gap: 14px; }
  .players header { display: flex; justify-content: space-between; align-items: baseline; gap: 10px; }
  .players header .muted { font-size: 12px; text-align: right; }
  h3 { font-size: 16px; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field .label b { color: var(--text); margin-left: 4px; }
  select {
    background: var(--surface-2); border: 1px solid var(--border); border-radius: 8px;
    padding: 9px 10px; color: var(--text); font: inherit; width: 100%;
  }
  input[type='range'] { width: 100%; accent-color: var(--accent); }
  .check { display: flex; align-items: center; gap: 8px; color: var(--text); }
  .check input { accent-color: var(--accent); width: 16px; height: 16px; }
  .swap {
    align-self: center; display: flex; flex-direction: column; align-items: center; gap: 2px;
    background: var(--surface-2); border: 1px solid var(--border); border-radius: 50%;
    width: 58px; height: 58px; justify-content: center; font-size: 20px; line-height: 1; color: var(--accent);
  }
  .swap span { font-size: 10px; color: var(--muted); letter-spacing: .06em; text-transform: uppercase; }
  .swap:hover { border-color: var(--accent); }

  .empty { display: flex; flex-direction: column; gap: 4px; }
  .pads { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
  .pad { display: grid; grid-template-columns: auto 1fr auto; gap: 12px; align-items: center; background: var(--surface); border: 1px solid var(--border); border-radius: 12px; padding: 14px 16px; transition: border-color .15s; }
  .pad.active { border-color: var(--accent); }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: var(--ok); }
  .pname { font-weight: 600; }
  .press { color: var(--faint); font-size: 11px; text-align: right; }
  .pad.active .press { color: var(--accent); }

  .map { padding: 0; overflow: hidden; }
  .tabs { display: flex; align-items: center; gap: 4px; padding: 10px 12px; border-bottom: 1px solid var(--border); }
  .tabs [role='tab'] { background: none; border: 0; padding: 7px 14px; border-radius: 8px; color: var(--muted); }
  .tabs [role='tab'].on { background: var(--surface-2); color: var(--accent); }
  .grow { flex: 1; }
  .btn.small { padding: 6px 12px; font-size: 12px; }
  table { width: 100%; border-collapse: collapse; }
  th { text-align: left; color: var(--muted); font-weight: 500; font-size: 11px; text-transform: uppercase; letter-spacing: .06em; padding: 10px 16px; }
  td { padding: 10px 16px; border-top: 1px solid var(--border); vertical-align: middle; }
  td:first-child { width: 28%; }
  .act { font-weight: 600; }
  .hint { color: var(--faint); font-size: 11px; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .chip {
    display: inline-flex; align-items: center; gap: 6px; background: var(--surface-2); border: 1px solid var(--border);
    border-radius: 6px; padding: 3px 4px 3px 9px; font: 12px var(--font-mono);
  }
  .chip button { background: none; border: 0; color: var(--faint); padding: 0 5px; font-size: 14px; line-height: 1; border-radius: 4px; }
  .chip button:hover { color: var(--bad); }
  .chip.waiting { border-color: var(--accent); color: var(--accent); padding: 3px 9px; animation: pulse 1s infinite; }
  .chip.waiting small { color: var(--faint); margin-left: 4px; }
  .add { background: none; border: 1px dashed var(--border); color: var(--muted); border-radius: 6px; padding: 3px 10px; font-size: 12px; }
  .add:hover { border-color: var(--accent); color: var(--accent); }
  @keyframes pulse { 50% { background: color-mix(in srgb, var(--accent) 14%, var(--surface-2)); } }

  .keys { display: grid; grid-template-columns: repeat(auto-fill, minmax(270px, 1fr)); gap: 10px; }
  .key { display: flex; align-items: center; gap: 12px; background: var(--surface); border: 1px solid var(--border); border-radius: 10px; padding: 10px 14px; }
  .key .or { color: var(--muted); font-size: 12px; }
  kbd { font: 12px var(--font-mono); background: var(--surface-2); border: 1px solid var(--border); border-bottom-width: 2px; border-radius: 5px; padding: 2px 8px; }
  .foot { color: var(--faint); margin-top: 12px; }
  tr.sep td { padding-top: 22px; border-bottom: 0; }
  tr.sep .act small { color: var(--faint); font-weight: 400; margin-left: 8px; }
  @media (max-width: 980px) { .players { grid-template-columns: 1fr; } }
</style>
