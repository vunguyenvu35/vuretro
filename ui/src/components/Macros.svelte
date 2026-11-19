<script>
  import { onMount, untrack } from 'svelte';
  import { gameKey, profilesOf } from '../lib/macros.js';
  import { api } from '../lib/api.js';
  import { app } from '../lib/store.svelte.js';
  import { toast, confirmDialog } from '../lib/ui.svelte.js';
  import { ACTIONS, RESERVED, keyName, keyLabel, padLabel, actionLabel, pressedBy } from '../lib/controls.js';
  import Select from './Select.svelte';

  let cfg = $state(null);
  let controls = $state(null);
  let scope = $state(''); // '' = every game, 'sys:<id>' = every game of a system, else a game key
  let saved = $state(false);
  let capture = $state(null); // { id, kind: 'key' | 'pad' }
  let rec = $state(null); // { id, down: { [key code]: string[] }, last: number, started: boolean }
  let saveTimer;
  let savedTimer;

  const KINDS = [
    { id: 'combo', name: 'Combo', hint: 'Hold the key to press several buttons together' },
    { id: 'turbo', name: 'Turbo', hint: 'Hold the key to tap the buttons again and again' },
    { id: 'sequence', name: 'Sequence', hint: 'Press the key to play buttons one after another' },
    { id: 'move', name: 'Move', hint: 'Type a special move or combo (236P, j.D, cl.C > 236P) and play it with one key' },
  ];

  // ---- moves and combos written in fighting-game notation ----
  let checks = $state({}); // macro id -> { ok, count, ms } | { error }
  const checkTimers = {};
  /** Moves to start from: the code that is written, and what it is. */
  const QUICK = [
    ['236P', 'Fireball (quarter circle forward + punch)'],
    ['214P', 'Reverse fireball (quarter circle back + punch)'],
    ['623P', 'Dragon punch'],
    ['41236P', 'Half circle forward + punch'],
    ['[4]6P', 'Charge back, then forward + punch'],
    ['236236P', 'Double quarter circle forward + punch'],
    ['623623AB', 'Double dragon punch + A+B'],
    ['j.D', 'Jumping kick'],
    ['cr.B', 'Crouching light kick'],
    ['A+B', 'A and B together'],
  ];
  /** Add a move to the one in the box: the first one replaces nothing, the next ones are linked with a comma. */
  function insertQuick(m, code) {
    const now = (m.notation ?? '').trim();
    m.notation = now ? `${now}, ${code}` : code;
    check(m);
  }
  /** What the notation means, shown in the card of a move. */
  const GUIDE = [
    ['Directions (facing right)', [['7 8 9', '↖ ↑ ↗'], ['4 5 6', '← neutral →'], ['1 2 3', '↙ ↓ ↘'], ['236', 'quarter circle forward']]],
    ['Buttons (Neo Geo)', [['A B C D', 'the four buttons'], ['P  K', 'a punch (A), a kick (B)'], ['A+B', 'two at once']]],
    ['Motions', [['QCF  QCB', '236  214'], ['DP  RDP', '623  421'], ['HCF  HCB', '41236  63214'], ['[4]6', 'charge back, then forward']]],
    ['Moves in a combo', [[',', 'link: wait for the first move'], ['>', 'cancel: press at once'], ['x3', 'repeat three times'], ['j.  cr.  cl.', 'jumping, crouching, standing']]],
  ];
  const isMove = (m) => m.notation != null;

  // What the buttons are called: the standard pad names, or a Neo Geo's A B C D. A Neo Geo
  // game gets its own names unless the player chose otherwise.
  const NAMES_KEY = 'vuretro.macro-names';
  const readNames = () => {
    try {
      return localStorage.getItem(NAMES_KEY) ?? '';
    } catch {
      return '';
    }
  };
  let namesPref = $state(readNames()); // '' = follow the game, 'neogeo' or 'pad'
  function setNames(v) {
    namesPref = v;
    try {
      localStorage.setItem(NAMES_KEY, v);
    } catch {
      /* a convenience only */
    }
  }
  const sysScope = $derived(scope.startsWith('sys:') ? scope.slice(4) : '');
  const gameScope = $derived(sysScope ? '' : scope); // the game key, when a game is chosen
  const scopeSystem = $derived(sysScope || (app.overview?.games.find((g) => gameKey(g.path) === gameScope)?.system ?? ''));
  // Moves are written with Neo Geo buttons, so only Neo Geo macros can be one.
  const kinds = $derived(KINDS.filter((k) => k.id !== 'move' || scopeSystem === 'neogeo'));
  const neo = $derived(namesPref === 'neogeo' || (namesPref === '' && scopeSystem === 'neogeo'));
  const lab = (a) => actionLabel(a, neo);
  const tip = (a) => `${a.label} on a standard pad · ${a.hint}`;
  /** "Z = A · X = B …": which of the player's keys is which button, in the names shown. */
  const keyLegend = (player) => {
    const kb = controls?.players[player]?.keyboard ?? {};
    return ['b', 'a', 'y', 'x']
      .map((id) => ({ a: ACTIONS.find((x) => x.id === id), keys: kb[id] ?? [] }))
      .filter((x) => x.keys.length)
      .map((x) => `${x.keys.map(keyLabel).join('/')} = ${lab(x.a)}`)
      .join(' · ');
  };

  const games = $derived.by(() => {
    const seen = new Map();
    for (const g of app.overview?.games ?? []) seen.set(gameKey(g.path), g.title);
    return [...seen].map(([key, title]) => ({ key, title })).sort((a, b) => a.title.localeCompare(b.title));
  });
  // Only reads: Svelte forbids changing state inside a $derived, and choosing a game that has no
  // macros yet used to do exactly that, which broke the whole screen.
  const NONE = [];
  // Which macros are shown: of every game, of one game (shared by its characters), or of one
  // profile of that game (a character).
  let profileId = $state(''); // '' = the macros shared by every character
  const profileInfo = $derived(profilesOf(cfg, gameScope));
  const profiles = $derived(profileInfo.profiles);
  const activeId = $derived(profileInfo.active);
  const profile = $derived(profiles.find((p) => p.id === profileId) ?? null);
  const list = $derived(
    cfg
      ? sysScope
        ? (cfg.systems?.[sysScope] ?? NONE)
        : gameScope
          ? (profile ? profile.macros : (cfg.games[gameScope] ?? NONE))
          : cfg.global
      : NONE,
  );
  // choosing another game shows the profile that is in use for it
  let profileScope = null;
  $effect(() => {
    const s = gameScope;
    if (cfg && s !== profileScope) {
      profileScope = s;
      profileId = untrack(() => profilesOf(cfg, s).active);
    }
  });
  const systemName = (id) => app.overview?.systems.find((x) => x.id === id)?.name ?? id;
  const systemsWithGames = $derived((app.overview?.systems ?? []).filter((x) => x.game_count > 0));
  const scopeName = $derived(
    sysScope ? `every ${systemName(sysScope)} game` : gameScope ? (games.find((g) => g.key === gameScope)?.title ?? gameScope) : 'every game',
  );

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

  /** Add a macro of a kind (`move` is a sequence made from notation); `extra` fills fields. */
  function add(kind, extra = {}) {
    const uid = crypto?.randomUUID?.() ?? String(Math.random()).slice(2);
    const k = KINDS.find((x) => x.id === kind);
    const asMove = kind === 'move';
    // A game gets its own list the first time something is added to it. Read it back after
    // assigning: `x ??= []` gives the plain array, not the copy the state keeps, and a macro
    // pushed onto the plain one is lost.
    if (sysScope) (cfg.systems ??= {}), (cfg.systems[sysScope] ??= []);
    else if (gameScope && !profile && !cfg.games[gameScope]) cfg.games[gameScope] = [];
    const target = sysScope ? cfg.systems[sysScope] : gameScope ? (profile ? profile.macros : cfg.games[gameScope]) : cfg.global;
    target.push({
      id: uid, name: k.name, enabled: true, player: 0, key: null, pad: null, kind: asMove ? 'sequence' : kind,
      actions: [], rate_hz: 15, steps: kind === 'sequence' ? [{ actions: [], ms: 100 }] : [], repeat: false,
      notation: asMove ? '' : null, mirror: false, auto_facing: asMove, ...extra,
    });
    // the next thing to do is choose the trigger, so wait for that key right away
    capture = { id: uid, kind: 'key' };
    save();
  }
  /** Work out what a move would play (and show why it cannot be played). */
  function check(m) {
    clearTimeout(checkTimers[m.id]);
    checkTimers[m.id] = setTimeout(async () => {
      const text = (m.notation ?? '').trim();
      if (!text) {
        checks[m.id] = { error: 'Type a move first, for example 236P' };
        return;
      }
      try {
        const r = await api.compileMove(text, m.mirror);
        checks[m.id] = { ok: true, count: r.steps.length, ms: r.total_ms };
        m.steps = r.steps;
      } catch (e) {
        checks[m.id] = { error: String(e) };
      }
      save();
    }, 250);
  }
  // moves that are already saved are checked when they appear
  $effect(() => {
    scope;
    cfg;
    untrack(() => {
      for (const m of list) if (isMove(m) && !checks[m.id]) check(m);
    });
  });

  // ---- profiles: a set of macros per character ----
  const newId = () => crypto?.randomUUID?.() ?? String(Math.random()).slice(2);
  function newProfile() {
    if (!gameScope || !cfg) return;
    cfg.profiles ??= {};
    cfg.active ??= {};
    if (!cfg.profiles[gameScope]) cfg.profiles[gameScope] = [];
    if (cfg.profiles[gameScope].length >= 50) return toast('A game can have up to 50 profiles', 'error');
    const id = newId();
    cfg.profiles[gameScope].push({ id, name: `Profile ${cfg.profiles[gameScope].length + 1}`, macros: [] });
    profileId = id;
    if (!cfg.active[gameScope]) cfg.active[gameScope] = id; // the first one is used at once
    save();
  }
  function useProfile(id) {
    cfg.active ??= {};
    if (id) cfg.active[gameScope] = id;
    else delete cfg.active[gameScope];
    save();
  }
  async function deleteProfile() {
    if (!profile) return;
    const yes = await confirmDialog({
      title: `Delete “${profile.name}”?`,
      message: `Its ${profile.macros.length} macro${profile.macros.length === 1 ? '' : 's'} are deleted with it.`,
      confirmLabel: 'Delete profile',
      danger: true,
    });
    if (!yes) return;
    const id = profile.id;
    cfg.profiles[gameScope] = cfg.profiles[gameScope].filter((p) => p.id !== id);
    if (!cfg.profiles[gameScope].length) delete cfg.profiles[gameScope];
    if (cfg.active?.[gameScope] === id) delete cfg.active[gameScope];
    profileId = '';
    save();
  }
  /** Turn a move into a plain sequence of steps that can be edited one by one. */
  function editByHand(m) {
    m.notation = null;
    save();
  }
  function remove(m) {
    const i = list.indexOf(m);
    if (i >= 0) list.splice(i, 1);
    if (sysScope && !list.length) delete cfg.systems[sysScope];
    else if (gameScope && !profile && !list.length) delete cfg.games[gameScope];
    save();
  }
  const has = (arr, a) => arr.includes(a);
  function toggle(arr, a) {
    const i = arr.indexOf(a);
    if (i >= 0) arr.splice(i, 1);
    else arr.push(a);
    save();
  }
  // A step with no button is a pause (the time between two presses). It is shown as a slim
  // "wait" row, not as a press with nothing lit; a step you just added is shown open so you
  // can choose its buttons.
  let openSteps = $state({}); // `${macro id}:${step index}` -> true
  const stepKey = (m, i) => `${m.id}:${i}`;
  const isWait = (m, st, i) => st.actions.length === 0 && !openSteps[stepKey(m, i)];
  function addStep(m) {
    m.steps.push({ actions: [], ms: 100 });
    openSteps[stepKey(m, m.steps.length - 1)] = true;
    save();
  }
  /** Make every pause between presses at most this long (a slow recording becomes a fast move). */
  const SHORT_PAUSE_MS = 100;
  function tightenPauses(m) {
    for (const st of m.steps) if (!st.actions.length) st.ms = Math.min(st.ms, SHORT_PAUSE_MS);
    save();
  }
  function removeStep(m, i) {
    openSteps = {};
    m.steps.splice(i, 1);
    save();
  }
  const totalMs = (m) => m.steps.reduce((t, s) => t + (+s.ms || 0), 0);

  // ---- recording a sequence from the keyboard (uses the player's own key bindings) ----
  /** The buttons a key presses for this player: its action, or both directions of a diagonal key. */
  function actionsOf(code, player) {
    const name = keyName(code);
    if (!name || !controls) return [];
    const kb = controls.players[player]?.keyboard ?? {};
    const hit = Object.keys(kb).find((a) => kb[a].includes(name));
    return hit ? pressedBy(hit) : [];
  }
  /** Every button held right now, whichever keys hold it. */
  const heldNow = () => ACTIONS.map((a) => a.id).filter((id) => Object.values(rec.down).some((l) => l.includes(id)));
  function startRec(m) {
    openSteps = {};
    capture = null; // recording a move and choosing a trigger key are different things
    m.steps = [];
    rec = { id: m.id, down: {}, last: performance.now(), started: false };
  }
  function pushStep(m, now) {
    if (!rec.started) return;
    const ms = Math.max(10, Math.round(now - rec.last));
    m.steps.push({ actions: heldNow(), ms });
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
      const as = m ? actionsOf(e.code, m.player) : [];
      if (!as.length || rec.down[e.code]) return; // not a game key, or a key that is already down
      const now = performance.now();
      pushStep(m, now);
      rec.down[e.code] = as;
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
    if (!m || !rec.down[e.code]) return;
    const now = performance.now();
    pushStep(m, now);
    delete rec.down[e.code];
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
          <Select bind:value={scope} searchable ariaLabel="Macros for" options={[{ value: '', label: 'Every game', hint: 'global' }, ...systemsWithGames.map((x) => ({ value: `sys:${x.id}`, label: `All ${x.name} games`, hint: 'system' })), ...games.map((g) => ({ value: g.key, label: g.title }))]} />
        </div>
        <span class="muted">{sysScope ? `Played in every ${systemName(sysScope)} game, together with the macros for every game.` : scope ? 'Played together with the macros of its system and for every game.' : 'Available in all games.'}</span>
        <span class="names" role="radiogroup" aria-label="How the buttons are named">
          <small>Buttons</small>
          <button role="radio" aria-checked={neo} class:on={neo} onclick={() => setNames('neogeo')} title="A B C D, as on a Neo Geo">Neo Geo</button>
          <button role="radio" aria-checked={!neo} class:on={!neo} onclick={() => setNames('pad')} title="B A Y X, as on a standard pad">Standard pad</button>
        </span>
      </div>

      {#if gameScope}
        <div class="profiles">
          <span class="slabel">Profile</span>
          <div class="ptabs" role="tablist" aria-label="Macro profile">
            <button role="tab" aria-selected={!profile} class="ptab" class:on={!profile} onclick={() => (profileId = '')} title="Macros every character of this game shares">All characters</button>
            {#each profiles as p (p.id)}
              <button role="tab" aria-selected={profile?.id === p.id} class="ptab" class:on={profile?.id === p.id} onclick={() => (profileId = p.id)}>{p.name || 'Profile'}{#if activeId === p.id}<i class="inuse">in use</i>{/if}</button>
            {/each}
            <button class="add" onclick={newProfile} title="A set of macros for one character">+ New profile</button>
          </div>
        </div>
        {#if profile}
          <div class="pbar">
            <input class="pname" bind:value={profile.name} oninput={save} maxlength="40" aria-label="Profile name" />
            {#if activeId === profile.id}
              <span class="inusebig">✓ in use when you play {scopeName}</span>
              <button class="btn small" onclick={() => useProfile(null)}>Stop using</button>
            {:else}
              <button class="btn small primary" onclick={() => useProfile(profile.id)}>Use this profile</button>
            {/if}
            <button class="btn small danger" onclick={deleteProfile}>Delete profile</button>
          </div>
          <p class="hint">Played together: “All characters”, this profile while it is in use, and the macros for every game.</p>
        {:else}
          <p class="hint">Shared by every character of this game. Make a profile for each character whose moves differ, and choose the one in use before you play.</p>
        {/if}
      {/if}

      {#each list as m (m.id)}
        <article class="card" class:off={!m.enabled}>
          <header class="mh">
            <span class="badge {isMove(m) ? 'move' : m.kind}">{isMove(m) ? 'Move' : KINDS.find((k) => k.id === m.kind)?.name}</span>
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

          {#if isMove(m)}
            <div class="acts">
              <span class="lbl">Move</span>
              <input class="notation" bind:value={m.notation} oninput={() => check(m)} placeholder="236P     or     j.D, cl.C > 236P" spellcheck="false" autocomplete="off" aria-label="Move or combo" />
            </div>
            <div class="acts">
              <span class="lbl">Start from</span>
              <span class="quick" role="group" aria-label="Moves to start from">
                {#each QUICK as [code, what] (code)}
                  <button onclick={() => insertQuick(m, code)} title="{what}. Click again to add another move after it.">{code}</button>
                {/each}
                {#if m.notation}<button class="clear" onclick={() => { m.notation = ''; check(m); }} title="Clear the box">Clear</button>{/if}
              </span>
            </div>
            <div class="acts">
              <span class="lbl">Facing</span>
              <span class="face" role="radiogroup" aria-label="Which way the character faces">
                <button role="radio" aria-checked={!m.auto_facing && !m.mirror} class:on={!m.auto_facing && !m.mirror} onclick={() => { m.auto_facing = false; m.mirror = false; check(m); }} title="Always as written: facing right">Right →</button>
                <button role="radio" aria-checked={!m.auto_facing && m.mirror} class:on={!m.auto_facing && m.mirror} onclick={() => { m.auto_facing = false; m.mirror = true; check(m); }} title="Always mirrored: facing left">← Left</button>
                <button role="radio" aria-checked={!!m.auto_facing} class:on={!!m.auto_facing} onclick={() => { m.auto_facing = true; m.mirror = false; check(m); }} title="The side is chosen each time you press the trigger">Auto</button>
              </span>
              <span class="status" class:ok={checks[m.id]?.ok} class:err={checks[m.id]?.error} aria-live="polite">
                {#if checks[m.id]?.ok}✓ {checks[m.id].count} steps · {(checks[m.id].ms / 1000).toFixed(2)} s{:else if checks[m.id]?.error}{checks[m.id].error}{/if}
              </span>
              <button class="btn small" onclick={() => editByHand(m)} title="Turn it into a sequence whose steps you edit yourself">Edit steps by hand</button>
            </div>
            {#if m.auto_facing}
              <p class="hint">Auto: when you press the trigger, <b>hold ← or → towards your opponent</b>. Holding ← plays the move mirrored, holding → (or nothing) plays it facing right. One move works on both sides.</p>
            {/if}
            <details class="guide">
              <summary>Notation guide</summary>
              <div class="gridg">
                {#each GUIDE as [title, rows] (title)}
                  <div>
                    <h5>{title}</h5>
                    {#each rows as [k, v] (k)}<div class="kv"><code>{k}</code><span>{v}</span></div>{/each}
                  </div>
                {/each}
              </div>
            </details>
          {:else if m.kind !== 'sequence'}
            <div class="acts">
              <span class="lbl">{m.kind === 'turbo' ? 'Taps' : 'Presses'}</span>
              <span class="pad">
                {#each ACTIONS as a (a.id)}
                  <button class:on={has(m.actions, a.id)} onclick={() => toggle(m.actions, a.id)} title={tip(a)}>{lab(a)}</button>
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
                    {#if keyLegend(m.player)}<span class="legend">{keyLegend(m.player)}</span>{/if}
                    <button class="btn small" onclick={stopRec}>Stop</button>
                  {:else}
                    <button class="btn small" onclick={() => startRec(m)} disabled={!!rec}>● Record</button>
                    <button class="btn small" onclick={() => addStep(m)}>+ Step</button>
                    <button class="btn small" onclick={() => tightenPauses(m)} title="Make every wait at most {SHORT_PAUSE_MS} ms">Shorten waits</button>
                  {/if}
                </span>
              </div>
              {#each m.steps as st, i (i)}
                {#if isWait(m, st, i)}
                  <div class="step wait">
                    <span class="n mono">{i + 1}</span>
                    <span class="waitl">⏱ wait <small>nothing pressed</small></span>
                    <button class="add" onclick={() => (openSteps[stepKey(m, i)] = true)} title="Press buttons in this step">+ buttons</button>
                    <label class="ms"><input type="number" min="10" max="10000" step="10" bind:value={st.ms} onchange={save} aria-label="Milliseconds" /> ms</label>
                    <button class="del" onclick={() => removeStep(m, i)} aria-label="Remove step">✕</button>
                  </div>
                {:else}
                <div class="step">
                  <span class="n mono">{i + 1}</span>
                  <span class="pad small">
                    {#each ACTIONS as a (a.id)}
                      <button class:on={has(st.actions, a.id)} onclick={() => toggle(st.actions, a.id)} title={tip(a)}>{lab(a)}</button>
                    {/each}
                  </span>
                  <label class="ms"><input type="number" min="10" max="10000" step="10" bind:value={st.ms} onchange={save} aria-label="Milliseconds" /> ms</label>
                  <button class="del" onclick={() => removeStep(m, i)} aria-label="Remove step">✕</button>
                </div>
                {/if}
              {/each}
              <label class="check"><input type="checkbox" bind:checked={m.repeat} onchange={save} /> Keep repeating while the trigger is held</label>
            </div>
          {/if}
        </article>
      {:else}
        <div class="empty">
          <strong>No macros for {profile ? profile.name : scopeName} yet</strong>
          <span>Add one below. Pick what it should do.</span>
        </div>
      {/each}

      <div class="addrow">
        {#each kinds as k (k.id)}
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
  .profiles { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .ptabs { display: flex; gap: 6px; flex-wrap: wrap; align-items: center; flex: 1; }
  .ptab { background: var(--surface); border: 1px solid var(--border); color: var(--muted); border-radius: 999px; padding: 5px 14px; font: 600 12px var(--font-body); display: inline-flex; gap: 8px; align-items: center; }
  .ptab:hover { color: var(--text); }
  .ptab.on { background: color-mix(in srgb, var(--accent) 16%, var(--surface)); border-color: var(--accent); color: var(--accent); }
  .inuse { font-style: normal; font-size: 10px; text-transform: uppercase; letter-spacing: .06em; color: var(--ok); }
  .inusebig { color: var(--ok); font-size: 12px; }
  .pbar { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .pname { flex: 1; min-width: 160px; font-weight: 600; background: var(--bg); border: 1px solid var(--border); border-radius: 9px; padding: 7px 12px; color: var(--text); }
  .pname:focus { outline: none; border-color: var(--accent); }
  .btn.danger { color: var(--bad); }
  .names { margin-left: auto; display: inline-flex; align-items: center; background: var(--bg); border: 1px solid var(--border); border-radius: 9px; padding: 2px; gap: 2px; }
  .names small { color: var(--faint); padding: 0 8px; font-size: 11px; text-transform: uppercase; letter-spacing: .06em; }
  .names button { background: none; border: 0; color: var(--muted); padding: 4px 10px; border-radius: 7px; font: 600 12px var(--font-body); }
  .names button.on { background: var(--accent); color: #090b10; }
  .legend { font: 12px var(--font-mono); color: var(--muted); }

  .card { background: var(--surface); border: 1px solid var(--border); border-radius: 16px; padding: 16px 18px; display: flex; flex-direction: column; gap: 14px; }
  .card.off { opacity: .55; }
  .mh { display: flex; align-items: center; gap: 12px; }
  .badge { font: 700 11px var(--font-body); text-transform: uppercase; letter-spacing: .07em; padding: 4px 9px; border-radius: 999px; flex: none; }
  .badge.combo { color: var(--accent); background: color-mix(in srgb, var(--accent) 16%, transparent); }
  .badge.turbo { color: var(--brand-magenta); background: color-mix(in srgb, var(--brand-magenta) 16%, transparent); }
  .badge.sequence { color: #6cc8ff; background: color-mix(in srgb, #6cc8ff 16%, transparent); }
  .badge.move { color: #7be29a; background: color-mix(in srgb, #7be29a 16%, transparent); }
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
  .step.wait { background: transparent; border-style: dashed; padding: 4px 10px; opacity: .8; }
  .waitl { flex: 1; color: var(--muted); font-size: 12px; }
  .waitl small { color: var(--faint); margin-left: 6px; }
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
  .addrow { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; }
  .newk { display: flex; flex-direction: column; align-items: flex-start; gap: 4px; text-align: left; background: var(--surface); border: 1px solid var(--border); border-radius: 14px; padding: 14px 16px; color: var(--text); transition: border-color .15s, transform .1s; }
  .newk b { font-size: 14px; }
  .newk small { color: var(--muted); font-size: 12px; line-height: 1.4; }
  .newk:hover { border-color: var(--accent); }
  .newk:active { transform: scale(.98); }
  .foot { color: var(--faint); font-size: 12px; margin: 0; }
  .notation { flex: 1; min-width: 220px; font: 14px var(--font-mono); background: var(--bg); border: 1px solid var(--border); border-radius: 9px; padding: 8px 12px; color: var(--text); }
  .notation:focus { outline: none; border-color: var(--accent); }
  .quick { display: flex; flex-wrap: wrap; gap: 6px; flex: 1; }
  .quick button { background: var(--bg); border: 1px solid var(--border); border-radius: 8px; padding: 4px 9px; color: var(--muted); font: 12px var(--font-mono); }
  .quick button:hover { color: var(--text); border-color: var(--accent); }
  .quick .clear { font: 600 12px var(--font-body); border-style: dashed; }
  .face { display: inline-flex; background: var(--bg); border: 1px solid var(--border); border-radius: 9px; padding: 2px; gap: 2px; }
  .face button { background: none; border: 0; color: var(--muted); padding: 4px 10px; border-radius: 7px; font: 600 12px var(--font-body); }
  .face button.on { background: var(--accent); color: #090b10; }
  .status { flex: 1; min-width: 140px; font-size: 12px; color: var(--muted); }
  .status.ok { color: var(--ok); }
  .status.err { color: var(--bad); }

  .hint { color: var(--muted); margin: 0; font-size: 12px; line-height: 1.5; }
  .guide { margin-top: 6px; }
  .guide > summary { cursor: pointer; color: var(--muted); font-size: 13px; }
  .gridg { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 14px; margin-top: 10px; }
  .gridg h5 { margin: 0 0 6px; font-size: 11px; text-transform: uppercase; letter-spacing: .07em; color: var(--muted); }
  .kv { display: flex; gap: 10px; font-size: 12px; color: var(--muted); padding: 1px 0; }
  .kv code { min-width: 44px; font: 12px var(--font-mono); color: var(--text); }
  @media (max-width: 980px) { .addrow { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
  @media (max-width: 820px) { .addrow { grid-template-columns: 1fr; } }
</style>
