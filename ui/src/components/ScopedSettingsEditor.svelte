<script>
  import { api } from '../lib/api.js';
  import { toast } from '../lib/ui.svelte.js';

  /**
   * Window size, CRT and rewind for one system or one game. `load()` and `save(override)` talk
   * to the backend for that scope (both give { effective, inherited, overridden });
   * `follows` names what everything not changed here is inherited from.
   */
  let { name, load, save, follows = 'the general settings' } = $props();

  const SCALES = [1, 2, 3, 4, 5, 6];
  let data = $state(null);
  let saved = $state(false);
  let timer;

  $effect(() => {
    data = null;
    let stale = false;
    load().then((d) => !stale && (data = d));
    return () => (stale = true);
  });

  /** Set one field of this system's override; `null` makes it follow the general setting again. */
  async function set(field, value) {
    try {
      data = await save({ ...data.overridden, [field]: value });
      saved = true;
      clearTimeout(timer);
      timer = setTimeout(() => (saved = false), 1800);
    } catch (e) {
      toast(String(e), 'error');
    }
  }
  const own = (f) => data.overridden[f] != null;
  const anyOwn = $derived(data ? ['window_scale', 'crt', 'rewind'].some(own) : false);

  async function resetAll() {
    data = await save({ window_scale: null, crt: null, rewind: null });
    toast(`${name} follows ${follows} again`);
  }
</script>

{#if data}
  <div class="intro">
    <p>
      Choices here apply to <strong>{name}</strong> only. Anything you do not change follows {follows}.
      Sound and the other habits stay general.
    </p>
    <span class="saved" class:show={saved} aria-live="polite">Saved</span>
    {#if anyOwn}<button class="btn" onclick={resetAll}>Follow {follows} again</button>{/if}
  </div>

  <div class="panel">
    <div class="row" class:own={own('window_scale')}>
      <div class="txt">
        <div class="t">Window size {#if own('window_scale')}<span class="badge">customised</span>{/if}</div>
        <div class="d">A multiple of the game's own resolution. Inherited: {data.inherited.window_scale}×.</div>
      </div>
      <div class="seg" role="radiogroup" aria-label="Window size for {name}">
        {#each SCALES as n (n)}
          <button role="radio" aria-checked={data.effective.window_scale === n} class:on={data.effective.window_scale === n} onclick={() => set('window_scale', n)}>{n}×</button>
        {/each}
      </div>
      {#if own('window_scale')}<button class="link" onclick={() => set('window_scale', null)}>Inherit</button>{/if}
    </div>

    <div class="row" class:own={own('crt')}>
      <div class="txt">
        <div class="t">CRT scanlines {#if own('crt')}<span class="badge">customised</span>{/if}</div>
        <div class="d">Looks best on low-resolution systems. Inherited: {data.inherited.crt ? 'on' : 'off'}.</div>
      </div>
      <button class="switch" role="switch" aria-checked={data.effective.crt} aria-label="CRT scanlines for {name}" class:on={data.effective.crt} onclick={() => set('crt', !data.effective.crt)}><span></span></button>
      {#if own('crt')}<button class="link" onclick={() => set('crt', null)}>Inherit</button>{/if}
    </div>

    <div class="row" class:own={own('rewind')}>
      <div class="txt">
        <div class="t">Rewind {#if own('rewind')}<span class="badge">customised</span>{/if}</div>
        <div class="d">Uses memory in proportion to the system's save states, so it suits small systems best. Inherited: {data.inherited.rewind ? 'on' : 'off'}.</div>
      </div>
      <button class="switch" role="switch" aria-checked={data.effective.rewind} aria-label="Rewind for {name}" class:on={data.effective.rewind} onclick={() => set('rewind', !data.effective.rewind)}><span></span></button>
      {#if own('rewind')}<button class="link" onclick={() => set('rewind', null)}>Inherit</button>{/if}
    </div>
  </div>
{:else}
  <p class="muted">Loading…</p>
{/if}

<style>
  .intro { display: flex; align-items: center; gap: 14px; flex-wrap: wrap; margin-bottom: 16px; color: var(--muted); }
  .intro p { margin: 0; flex: 1 1 360px; }
  .saved { color: var(--ok); font-size: 12px; opacity: 0; transition: opacity .2s; }
  .saved.show { opacity: 1; }
  .panel { background: var(--surface); border: 1px solid var(--border); border-radius: 14px; overflow: hidden; } container-type: inline-size; 
  .row { display: flex; align-items: center; gap: 16px; padding: 16px 18px; border-bottom: 1px solid var(--border); }
  .row:last-child { border-bottom: 0; }
  .row.own { background: color-mix(in srgb, var(--accent) 6%, transparent); }
  .txt { flex: 1; min-width: 0; }
  .t { font-weight: 600; }
  .d { color: var(--muted); font-size: 12px; margin-top: 2px; }
  .badge { margin-left: 8px; font-size: 10px; text-transform: uppercase; letter-spacing: .08em; color: var(--accent); border: 1px solid color-mix(in srgb, var(--accent) 50%, transparent); border-radius: 999px; padding: 1px 7px; }
  .seg { display: inline-flex; background: var(--bg); border: 1px solid var(--border); border-radius: 10px; padding: 3px; gap: 2px; }
  .seg button { background: none; border: 0; color: var(--muted); padding: 5px 10px; border-radius: 7px; font: inherit; }
  .seg button.on { background: var(--accent); color: #090b10; font-weight: 700; }
  .switch { flex: none; width: 42px; height: 24px; border-radius: 999px; border: 1px solid var(--border); background: var(--bg); position: relative; padding: 0; }
  .switch span { position: absolute; top: 2px; left: 2px; width: 18px; height: 18px; border-radius: 50%; background: var(--muted); transition: transform .15s, background .15s; }
  .switch.on { background: color-mix(in srgb, var(--accent) 30%, var(--bg)); border-color: var(--accent); }
  .switch.on span { transform: translateX(18px); background: var(--accent); }
  /* in the narrow details panel the rows stack: text on top, controls below */
  @container (max-width: 440px) {
    .row { flex-wrap: wrap; padding: 12px 14px; gap: 10px; }
    .txt { flex: 1 1 100%; }
    .seg button { padding: 5px 8px; }
  }
  .link { background: none; border: 0; color: var(--muted); font-size: 12px; text-decoration: underline; padding: 0; }
  .link:hover { color: var(--text); }
  .muted { color: var(--muted); }
</style>
