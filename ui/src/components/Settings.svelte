<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { toast } from '../lib/ui.svelte.js';
  import { HOTKEYS, PAD_HOTKEYS, APP_SHORTCUTS } from '../lib/controls.js';
  import { app } from '../lib/store.svelte.js';
  import Backup from './Backup.svelte';
  import ScopeBar from './ScopeBar.svelte';
  import ScopedSettingsEditor from './ScopedSettingsEditor.svelte';
  import { parseScope } from '../lib/scope.js';
  const scope = $derived(parseScope(app.scope));

  let s = $state(null);
  let saved = $state(false);
  let saveTimer;

  const SCALES = [1, 2, 3, 4, 5, 6];
  const NATIVE = { w: 304, h: 224 }; // Neo Geo
  const SYSTEMS = [
    { name: 'Neo Geo', w: 304, h: 224, color: '#ffb347' },
    { name: 'Super Nintendo', w: 256, h: 224, color: '#b497ff' },
    { name: 'PlayStation', w: 320, h: 240, color: '#6cc8ff' },
    { name: 'Game Boy Advance', w: 240, h: 160, color: '#9be36b' },
  ];
  // Width of the preview monitor's screen at 6x. Its height (x 224/304) plus the bezel, the
  // gap and the size label must fit the stage below, or the biggest sizes get clipped.
  const STAGE_MAX = 290;

  // Whatever the backend sends, fill in anything missing so a switch never gets stuck.
  const DEFAULTS = {
    window_scale: 2, sound: true, volume: 100, sync_display: true, pause_on_blur: false, crt: false, rewind: false,
    fast_forward: 4, ff_sound: false, resume: false, lcd: false, integer_scale: false, fullscreen: false, rumble: true, auto_coin: false,
  };
  const SPEEDS = [2, 3, 4, 8];
  const withDefaults = (v) => ({ ...DEFAULTS, ...v });

  onMount(async () => {
    s = withDefaults(await api.gameSettings());
  });

  async function save() {
    try {
      s = withDefaults(await api.setGameSettings($state.snapshot(s)));
      saved = true;
      clearTimeout(saveTimer);
      saveTimer = setTimeout(() => (saved = false), 1800);
    } catch (e) {
      toast(String(e), 'error');
    }
  }
  function setScale(n) {
    s.window_scale = n;
    save();
  }
  function toggleSound() {
    s.sound = !s.sound;
    save();
  }
  function toggleSync() {
    s.sync_display = !s.sync_display;
    save();
  }
  function toggleRewind() {
    s.rewind = !s.rewind;
    save();
  }
  function toggleCrt() {
    s.crt = !s.crt;
    save();
  }
  function togglePause() {
    s.pause_on_blur = !s.pause_on_blur;
    save();
  }
  /** Flip a yes/no setting by name and save. */
  function flip(field) {
    s[field] = !s[field];
    save();
  }
  function setSpeed(n) {
    s.fast_forward = n;
    save();
  }
  async function reset() {
    s = withDefaults(await api.setGameSettings(await api.defaultGameSettings()));
    toast('Settings reset to defaults');
  }

  const idx = $derived(s ? SCALES.indexOf(s.window_scale) : 0);
  const boxW = $derived(s ? (STAGE_MAX * s.window_scale) / 6 : 0);
</script>

<section class="page">
  <div class="col">
    <header class="head">
      <div>
        <h1>Settings</h1>
        <p class="sub">Choose how games look and sound. Applies the next time you start a game.</p>
      </div>
      <span class="saved" class:show={saved} aria-live="polite">
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M3 8.5l3.2 3.2L13 4.8" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" /></svg>
        Saved
      </span>
    </header>

    <ScopeBar hint="Window size, picture look, rewind, fast-forward speed and carrying on can differ per system." />

    {#if scope.kind === 'system'}
      {#key app.scope}
        <ScopedSettingsEditor
          name={app.overview?.systems.find((x) => x.id === scope.id)?.name ?? scope.id}
          load={() => api.scopedSettings(scope.id)}
          save={(ov) => api.setSystemOverride(scope.id, ov)}
          follows="the settings for All games"
          coinOp={!!app.overview?.systems.find((x) => x.id === scope.id)?.coin_op}
        />
      {/key}
    {:else if scope.kind === 'game'}
      {#key app.scope}
        <ScopedSettingsEditor
          name={app.overview?.games.find((g) => g.path === scope.id)?.title ?? scope.id}
          load={() => api.gameOverride(scope.id)}
          save={(ov) => api.setGameOverride(scope.id, ov)}
          follows="its system's settings"
          coinOp={!!app.overview?.systems.find((x) => x.id === app.overview?.games.find((g) => g.path === scope.id)?.system)?.coin_op}
        />
      {/key}
    {:else if s}
      <!-- ================= DISPLAY ================= -->
      <h2 class="group">Display</h2>
      <div class="panel">
        <div class="preview">
          <div class="glow" aria-hidden="true"></div>
          <div class="monitor" style="width:{boxW + 20}px" aria-hidden="true">
            <svg class="game" viewBox="0 0 {NATIVE.w} {NATIVE.h}" shape-rendering="crispEdges" preserveAspectRatio="xMidYMid slice">
              <defs>
                <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0" stop-color="#2a1b5c" /><stop offset=".55" stop-color="#ff4f8b" /><stop offset="1" stop-color="#ffb347" />
                </linearGradient>
              </defs>
              <rect width="304" height="224" fill="url(#sky)" />
              <circle cx="212" cy="104" r="30" fill="#ffe29a" />
              <path d="M0 150 L48 108 L84 138 L132 92 L186 144 L238 112 L304 152 V224 H0Z" fill="#26164a" />
              <path d="M0 176 L60 150 L120 172 L190 146 L304 178 V224 H0Z" fill="#150b2e" />
              <rect y="196" width="304" height="28" fill="#0c0620" />
              <rect x="42" y="168" width="14" height="28" fill="#ffb347" /><rect x="46" y="160" width="8" height="8" fill="#ffd9a0" />
              <text x="152" y="52" text-anchor="middle" font-family="'Press Start 2P', monospace" font-size="13" fill="#fff" letter-spacing="1">INSERT COIN</text>
            </svg>
            <div class="scan"></div>
          </div>
          <div class="dim mono"><b>{NATIVE.w * s.window_scale}</b> × <b>{NATIVE.h * s.window_scale}</b> px</div>
        </div>

        <div class="row">
          <span class="tile amber" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="18" height="18"><rect x="3" y="4" width="18" height="12" rx="2" fill="none" stroke="currentColor" stroke-width="1.8" /><path d="M9 20h6M12 16v4" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" /></svg>
          </span>
          <div class="txt">
            <div class="t">Window size</div>
            <div class="d">A multiple of the game's own resolution. In a game, press <kbd>F9</kbd> / <kbd>F10</kbd> to make it smaller or bigger.</div>
          </div>
          <div class="seg" role="radiogroup" aria-label="Window size" style="--i:{idx}">
            <span class="thumb" aria-hidden="true"></span>
            {#each SCALES as n (n)}
              <button role="radio" aria-checked={s.window_scale === n} class:on={s.window_scale === n} onclick={() => setScale(n)}>{n}×</button>
            {/each}
          </div>
        </div>

        <div class="row">
          <span class="tile blue" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="18" height="18"><path d="M3 12h4l2.5-6 4 12 2.5-6H21" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>
          </span>
          <div class="txt">
            <div class="t">Smooth motion</div>
            <div class="d">Match your screen's refresh rate. Removes the small periodic stutter on 60 Hz displays; the game may run up to 5% faster than the original.</div>
          </div>
          <button class="switch" role="switch" aria-checked={s.sync_display} aria-label="Smooth motion" class:on={s.sync_display} onclick={toggleSync}><span></span></button>
        </div>

        <div class="row">
          <span class="tile blue" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="18" height="18"><path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>
          </span>
          <div class="txt">
            <div class="t">Start in full screen</div>
            <div class="d">Games open in a window that fills the display. Press <kbd>Alt</kbd> + <kbd>Enter</kbd> in a game to switch. Not available on every system yet.</div>
          </div>
          <button class="switch" role="switch" aria-checked={s.fullscreen} aria-label="Start in full screen" class:on={s.fullscreen} onclick={() => flip('fullscreen')}><span></span></button>
        </div>

        <div class="row">
          <span class="tile blue" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="18" height="18"><path d="M5 5h6v6H5zM13 13h6v6h-6z" fill="none" stroke="currentColor" stroke-width="1.8" /></svg>
          </span>
          <div class="txt">
            <div class="t">Whole-number scaling</div>
            <div class="d">Enlarges the picture by 2×, 3×, 4× … only, with black borders instead of a blurry stretch. Pixels stay sharp.</div>
          </div>
          <button class="switch" role="switch" aria-checked={s.integer_scale} aria-label="Whole-number scaling" class:on={s.integer_scale} onclick={() => flip('integer_scale')}><span></span></button>
        </div>

        <div class="sizes">
          <div class="shead">Resulting window size</div>
          <ul>
            {#each SYSTEMS as sys (sys.name)}
              <li>
                <span class="sname"><i style="background:{sys.color}"></i>{sys.name}</span>
                <span class="sval mono">{sys.w * s.window_scale} × {sys.h * s.window_scale}</span>
              </li>
            {/each}
          </ul>
        </div>
      </div>

      <!-- ================= GAMEPLAY ================= -->
      <h2 class="group">Gameplay</h2>
      <div class="panel">
        <div class="row">
          <span class="tile green" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="18" height="18"><rect x="6" y="5" width="4" height="14" rx="1.2" fill="currentColor" /><rect x="14" y="5" width="4" height="14" rx="1.2" fill="currentColor" /></svg>
          </span>
          <div class="txt">
            <div class="t">Pause when the window loses focus</div>
            <div class="d">The game stops and goes quiet while you are in another app, and carries on when you come back. Press <kbd>P</kbd> in a game to pause it yourself, and again to resume.</div>
          </div>
          <button class="switch" role="switch" aria-checked={s.pause_on_blur} aria-label="Pause when the window loses focus" class:on={s.pause_on_blur} onclick={togglePause}><span></span></button>
        </div>
        <div class="row">
          <span class="tile green" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="18" height="18"><rect x="4" y="5" width="16" height="11" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.7" /><path d="M7 9h10M7 12h10M9 19h6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" /></svg>
          </span>
          <div class="txt">
            <div class="t">CRT scanlines</div>
            <div class="d">Draws dark lines between the rows of the picture, like an old TV. Press <kbd>F4</kbd> in a game to switch it on or off.</div>
          </div>
          <button class="switch" role="switch" aria-checked={s.crt} aria-label="CRT scanlines" class:on={s.crt} onclick={toggleCrt}><span></span></button>
        </div>
        <div class="row">
          <span class="tile green" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="18" height="18"><path d="M11 6L4 12l7 6V6zM20 6l-7 6 7 6V6z" fill="currentColor" /></svg>
          </span>
          <div class="txt">
            <div class="t">Rewind</div>
            <div class="d">Keeps recent moments in memory (up to 64 MB) so you can hold <kbd>Backspace</kbd> to go back in time. Uses some extra CPU and works only with cores that support save states.</div>
          </div>
          <button class="switch" role="switch" aria-checked={s.rewind} aria-label="Rewind" class:on={s.rewind} onclick={toggleRewind}><span></span></button>
        </div>
        <div class="row">
          <span class="tile green" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="18" height="18"><path d="M4 5h16v14H4zM9.3 5v14M14.7 5v14M4 9.7h16M4 14.3h16" fill="none" stroke="currentColor" stroke-width="1.4" /></svg>
          </span>
          <div class="txt">
            <div class="t">LCD pixel grid</div>
            <div class="d">A handheld look: a fine grid between the pixels, for Game Boy and similar systems. Press <kbd>Shift</kbd> + <kbd>F4</kbd> in a game. If CRT scanlines are on too, only one is used.</div>
          </div>
          <button class="switch" role="switch" aria-checked={s.lcd} aria-label="LCD pixel grid" class:on={s.lcd} onclick={() => flip('lcd')}><span></span></button>
        </div>
        <div class="row">
          <span class="tile green" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="18" height="18"><path d="M4 6l7 6-7 6V6zM13 6l7 6-7 6V6z" fill="currentColor" /></svg>
          </span>
          <div class="txt">
            <div class="t">Fast-forward speed</div>
            <div class="d">How fast the game runs while you hold <kbd>Tab</kbd> (or press <kbd>F1</kbd> to leave it on). <kbd>Shift</kbd> + <kbd>F1</kbd> plays in slow motion.</div>
          </div>
          <div class="seg" role="radiogroup" aria-label="Fast-forward speed">
            {#each SPEEDS as n (n)}
              <button role="radio" aria-checked={s.fast_forward === n} class:on={s.fast_forward === n} onclick={() => setSpeed(n)}>{n}×</button>
            {/each}
          </div>
        </div>
        <div class="row">
          <span class="tile green" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="18" height="18"><path d="M4 9.5v5h3.5L12 18.5v-13L7.5 9.5H4z" fill="currentColor" /><path d="M15.5 9a4 4 0 010 6" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" /></svg>
          </span>
          <div class="txt">
            <div class="t">Sound while fast-forwarding</div>
            <div class="d">Off: the game is silent while it runs faster. On: the sound plays faster and higher, like a tape. Slow motion is always silent.</div>
          </div>
          <button class="switch" role="switch" aria-checked={s.ff_sound} aria-label="Sound while fast-forwarding" class:on={s.ff_sound} onclick={() => flip('ff_sound')}><span></span></button>
        </div>
        <div class="row">
          <span class="tile green" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="18" height="18"><path d="M6 4v16M6 5l11 3.5L6 12" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>
          </span>
          <div class="txt">
            <div class="t">Carry on where I left off</div>
            <div class="d">Saves the game when you close it and loads it again the next time you start that game. Press <kbd>F2</kbd> to start over. Works only with cores that support save states.</div>
          </div>
          <button class="switch" role="switch" aria-checked={s.resume} aria-label="Carry on where I left off" class:on={s.resume} onclick={() => flip('resume')}><span></span></button>
        </div>
        <div class="row">
          <span class="tile green" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="18" height="18"><circle cx="12" cy="12" r="8" fill="none" stroke="currentColor" stroke-width="1.8" /><path d="M12 8v8M9.5 10.5h5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg>
          </span>
          <div class="txt">
            <div class="t">Insert a coin when Start is pressed</div>
            <div class="d">For arcade games (Neo Geo, MAME), which ignore Start until a coin is in. A coin goes in first, then Start follows a moment later, so Start always begins a game. Does nothing on consoles.</div>
          </div>
          <button class="switch" role="switch" aria-checked={s.auto_coin} aria-label="Insert a coin when Start is pressed" class:on={s.auto_coin} onclick={() => flip('auto_coin')}><span></span></button>
        </div>
        <div class="row">
          <span class="tile green" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="18" height="18"><path d="M8 3h8v18H8zM4 8v8M20 8v8" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" /></svg>
          </span>
          <div class="txt">
            <div class="t">Gamepad vibration</div>
            <div class="d">Rumble when the game asks for it. Needs a gamepad that supports it, and works on Windows and Linux; macOS does not offer it to apps yet.</div>
          </div>
          <button class="switch" role="switch" aria-checked={s.rumble} aria-label="Gamepad vibration" class:on={s.rumble} onclick={() => flip('rumble')}><span></span></button>
        </div>
      </div>

      <!-- ================= SOUND ================= -->
      <h2 class="group">Sound</h2>
      <div class="panel">
        <div class="row">
          <span class="tile pink" class:muted={!s.sound} aria-hidden="true">
            <svg viewBox="0 0 24 24" width="18" height="18"><path d="M4 9.5v5h3.5L12 18.5v-13L7.5 9.5H4z" fill="currentColor" />{#if s.sound}<path d="M15.5 9a4 4 0 010 6M18 6.5a7.5 7.5 0 010 11" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />{:else}<path d="M16 9.5l5 5M21 9.5l-5 5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />{/if}</svg>
          </span>
          <div class="txt">
            <div class="t">Game sound</div>
            <div class="d">{s.sound ? 'Audio is on.' : 'Audio is muted.'} Press <kbd>F8</kbd> in a game to switch it without leaving.</div>
          </div>
          <button class="switch" role="switch" aria-checked={s.sound} aria-label="Game sound" class:on={s.sound} onclick={toggleSound}><span></span></button>
        </div>

        <div class="row" class:dim={!s.sound}>
          <span class="tile" aria-hidden="true"></span>
          <div class="txt">
            <label class="t" for="vol">Volume</label>
          </div>
          <div class="slider">
            <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true"><path d="M4 9.5v5h3.5L12 18.5v-13L7.5 9.5H4z" fill="currentColor" /></svg>
            <input id="vol" type="range" min="0" max="100" step="5" bind:value={s.volume} onchange={save} disabled={!s.sound} style="--pct:{s.volume}%" />
            <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true"><path d="M4 9.5v5h3.5L12 18.5v-13L7.5 9.5H4z" fill="currentColor" /><path d="M15.5 9a4 4 0 010 6M18 6.5a7.5 7.5 0 010 11" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" /></svg>
            <output class="pill mono">{s.volume}%</output>
          </div>
        </div>
      </div>

      <!-- ================= BACKUP ================= -->
      <Backup />

      <!-- ================= SHORTCUTS ================= -->
      {#each [['Shortcuts in the app', APP_SHORTCUTS, 'Press these anywhere in the app window.'], ['Shortcuts in a game', HOTKEYS, 'Button keys for each player are set on the Controls screen. These keys are fixed and cannot be used as buttons.'], ['Gamepad shortcuts in a game', PAD_HOTKEYS, 'Hold Select and press the other button. The game does not see those presses.']] as [title, list, foot] (title)}
        <h2 class="group">{title}</h2>
        <div class="panel">
          <ul class="keys">
            {#each list as h (h.what)}
              <li>
                <span class="kk">
                  {#each h.keys as k (k)}<kbd>{k}</kbd>{/each}
                  {#if h.also}<span class="or">or</span>{#each h.also as k (k)}<kbd>{k}</kbd>{/each}{/if}
                </span>
                <span class="w">{h.what}{#if h.note}<small>{h.note}</small>{/if}</span>
              </li>
            {/each}
          </ul>
          <p class="kfoot">{foot}</p>
        </div>
      {/each}

      <footer class="foot">
        <button class="link" onclick={reset}>Reset to defaults</button>
      </footer>
    {/if}
  </div>
</section>

<style>
  .page { flex: 1; overflow: auto; padding: 40px 32px 64px; min-width: 0; }
  .col { max-width: 860px; margin: 0 auto; }

  .head { display: flex; justify-content: space-between; align-items: flex-start; gap: 24px; }
  h1 { font-size: 30px; letter-spacing: -.015em; }
  .sub { color: var(--muted); margin: 8px 0 0; }
  .saved { display: inline-flex; align-items: center; gap: 6px; color: var(--ok); font-size: 13px; font-weight: 500; opacity: 0; transform: translateY(3px); transition: opacity .25s, transform .25s; padding: 6px 12px; border-radius: 999px; background: color-mix(in srgb, var(--ok) 12%, transparent); margin-top: 4px; }
  .saved.show { opacity: 1; transform: none; }

  .group { font: 600 12px var(--font-body); text-transform: uppercase; letter-spacing: .1em; color: var(--muted); margin: 36px 4px 10px; }
  .panel { background: var(--surface); border: 1px solid var(--border); border-radius: 16px; overflow: hidden; box-shadow: 0 1px 0 rgba(255,255,255,.03) inset, 0 12px 32px rgba(0,0,0,.25); }

  /* ---- preview ---- */
  .preview { position: relative; height: 320px; padding: 16px 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 16px; overflow: hidden;
    background: radial-gradient(70% 90% at 50% 0%, #1c2236 0%, #0d101a 70%); border-bottom: 1px solid var(--border); }
  .glow { position: absolute; inset: auto 0 -40% 0; height: 70%; background: radial-gradient(50% 60% at 50% 0%, color-mix(in srgb, var(--brand-magenta) 28%, transparent), transparent 70%); pointer-events: none; }
  .monitor { position: relative; padding: 10px; border-radius: 12px; background: linear-gradient(180deg, #1b2030, #0b0e16); box-shadow: 0 0 0 1px rgba(255,255,255,.07), 0 24px 50px rgba(0,0,0,.55); transition: width .3s cubic-bezier(.3,.8,.2,1); }
  .game { display: block; width: 100%; height: auto; border-radius: 5px; aspect-ratio: 304 / 224; }
  .scan { position: absolute; inset: 10px; border-radius: 5px; background: repeating-linear-gradient(0deg, rgba(0,0,0,.2) 0 1px, transparent 1px 3px); pointer-events: none; }
  .dim { position: relative; font-size: 12px; color: var(--muted); background: rgba(255,255,255,.05); border: 1px solid var(--border); padding: 4px 12px; border-radius: 999px; }
  .dim b { color: var(--text); font-weight: 600; }

  /* ---- rows ---- */
  .row { display: flex; align-items: center; gap: 16px; padding: 18px 22px; }
  .row + .row { border-top: 1px solid var(--border); }
  .row.dim { opacity: .45; }
  .tile { width: 36px; height: 36px; border-radius: 10px; display: grid; place-items: center; flex: none; background: transparent; color: var(--muted); }
  .tile.amber { color: var(--accent); background: color-mix(in srgb, var(--accent) 14%, transparent); }
  .tile.blue { color: #6cc8ff; background: color-mix(in srgb, #6cc8ff 14%, transparent); }
  .tile.green { color: var(--ok); background: color-mix(in srgb, var(--ok) 14%, transparent); }
  .tile.pink { color: var(--brand-magenta); background: color-mix(in srgb, var(--brand-magenta) 14%, transparent); transition: color .2s, background .2s; }
  .tile.pink.muted { color: var(--faint); background: var(--surface-2); }
  .txt { flex: 1; min-width: 0; }
  .t { font-weight: 600; font-size: 14px; }
  .d { color: var(--muted); font-size: 13px; margin-top: 3px; line-height: 1.45; }
  .sizes { padding: 16px 22px 20px; border-top: 1px solid var(--border); }
  .shead { color: var(--muted); font-size: 12px; text-transform: uppercase; letter-spacing: .08em; margin-bottom: 12px; }
  .sizes ul { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fit, minmax(170px, 1fr)); gap: 10px; }
  .sizes li { display: flex; flex-direction: column; gap: 6px; background: var(--surface-2); border: 1px solid var(--border); border-radius: 12px; padding: 12px 14px; }
  .sname { display: flex; align-items: center; gap: 8px; color: var(--muted); font-size: 12px; }
  .sname i { width: 7px; height: 7px; border-radius: 50%; flex: none; }
  .sval { font-size: 16px; font-weight: 600; color: var(--text); letter-spacing: .01em; }

  /* ---- segmented control with a sliding thumb ---- */
  .seg { --n: 6; position: relative; display: grid; grid-template-columns: repeat(var(--n), 1fr); background: var(--bg); border: 1px solid var(--border); border-radius: 12px; padding: 3px; flex: none; width: 300px; }
  .thumb { position: absolute; top: 3px; bottom: 3px; left: 3px; width: calc((100% - 6px) / var(--n)); border-radius: 9px; background: var(--accent); box-shadow: 0 2px 10px color-mix(in srgb, var(--accent) 40%, transparent); transform: translateX(calc(var(--i) * 100%)); transition: transform .25s cubic-bezier(.3,.8,.2,1); }
  .seg button { position: relative; z-index: 1; background: none; border: 0; padding: 8px 0; font: 700 13px var(--font-head); color: var(--muted); transition: color .2s; border-radius: 9px; }
  .seg button:hover { color: var(--text); }
  .seg button.on { color: #17110a; }

  /* ---- switch ---- */
  .switch { width: 52px; height: 30px; border-radius: 15px; border: 1px solid var(--border); background: var(--bg); position: relative; flex: none; padding: 0; transition: background .2s, border-color .2s; }
  .switch span { position: absolute; top: 3px; left: 3px; width: 22px; height: 22px; border-radius: 50%; background: var(--muted); box-shadow: 0 1px 4px rgba(0,0,0,.5); transition: transform .22s cubic-bezier(.3,.8,.2,1), background .2s; }
  .switch.on { background: var(--accent); border-color: var(--accent); }
  .switch.on span { transform: translateX(22px); background: #fff; }

  /* ---- slider ---- */
  .slider { display: flex; align-items: center; gap: 12px; color: var(--faint); flex: none; width: 360px; }
  .slider svg:last-of-type { color: var(--muted); }
  input[type='range'] { -webkit-appearance: none; appearance: none; flex: 1; min-width: 0; height: 6px; border-radius: 3px; outline: none; margin: 0;
    background: linear-gradient(to right, var(--accent) var(--pct), var(--bg) var(--pct)); box-shadow: 0 0 0 1px var(--border) inset; }
  input[type='range']::-webkit-slider-thumb { -webkit-appearance: none; width: 20px; height: 20px; border-radius: 50%; background: #fff; box-shadow: 0 2px 8px rgba(0,0,0,.55); cursor: pointer; transition: transform .1s; }
  input[type='range']::-webkit-slider-thumb:active { transform: scale(1.12); }
  input[type='range']::-moz-range-thumb { width: 20px; height: 20px; border: 0; border-radius: 50%; background: #fff; }
  input[type='range']:disabled::-webkit-slider-thumb { cursor: default; }
  .pill { min-width: 52px; text-align: right; font-size: 13px; color: var(--text); }

  kbd { font: 600 11px var(--font-mono); color: var(--text); background: var(--bg); border: 1px solid var(--border); border-bottom-width: 2px; border-radius: 5px; padding: 1px 6px; }

  .keys { list-style: none; margin: 0; padding: 8px 10px; display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 2px 12px; }
  .keys li { display: flex; align-items: center; gap: 14px; padding: 10px 12px; border-radius: 10px; }
  .keys li:hover { background: var(--surface-2); }
  .kk { display: inline-flex; align-items: center; gap: 5px; min-width: 150px; flex-wrap: wrap; }
  .kk .or { color: var(--faint); font-size: 11px; margin: 0 2px; }
  .w { font-size: 13px; color: var(--text); display: flex; flex-direction: column; }
  .w small { color: var(--faint); font-size: 11px; margin-top: 1px; }
  .kfoot { margin: 0; padding: 12px 22px 16px; border-top: 1px solid var(--border); color: var(--faint); font-size: 12px; }
  @media (max-width: 820px) { .keys { grid-template-columns: 1fr; } }
  .foot { display: flex; justify-content: flex-end; margin-top: 20px; }
  .link { background: none; border: 0; color: var(--muted); font-size: 13px; padding: 6px 4px; border-bottom: 1px dashed var(--border); }
  .link:hover { color: var(--accent); border-bottom-color: var(--accent); }

  @media (max-width: 820px) {
    .row { flex-wrap: wrap; }
    .seg, .slider { width: 100%; }
  }
</style>
