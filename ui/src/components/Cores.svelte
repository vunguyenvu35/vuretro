<script>
  import { onMount } from 'svelte';
  import { app, refresh, importBios, importDats } from '../lib/store.svelte.js';
  import { api } from '../lib/api.js';
  import { colorOf } from '../lib/systems.js';
  import Select from './Select.svelte';
  import CoreOptions from './CoreOptions.svelte';
  import { toast, confirmDialog } from '../lib/ui.svelte.js';

  let over = $state(false);
  /** the system whose core options are open */
  let optionsFor = $state(null);
  /** core name -> percent while it downloads */
  let installing = $state({});
  const ov = $derived(app.overview);

  const statusOf = (s) =>
    !s.enabled ? ['Off', 'muted'] : !s.renderer_ready ? ['3D not supported yet', 'bad'] : !s.core_installed ? ['Core missing', 'bad'] : !s.bios_ready ? ['BIOS missing', 'bad'] : ['Ready', 'ok'];

  const LEVELS = {
    good: ['Your computer meets the requirements', 'ok'],
    marginal: ['May lag in demanding games', 'warn'],
    poor: ['Probably too slow for this system', 'bad'],
  };

  /** 3D systems are heavy: say what this computer can do before turning one on. */
  async function toggle(s) {
    if (s.hardware && !s.enabled) {
      const a = s.assessment;
      const lines = [a ? `${LEVELS[a.level][0]}.` : 'This computer could not be measured.', ...(a?.reasons ?? [])];
      if (!s.renderer_ready) lines.push('This version of vuretro cannot draw 3D systems yet, so games will stay locked.');
      lines.push('vuretro tells you in the window title if a game runs slowly.');
      const ok = await confirmDialog({
        title: `Turn on ${s.name}?`,
        message: lines.map((l) => `• ${l}`).join('\n'),
        confirmLabel: 'Turn on',
        danger: a?.level === 'poor',
      });
      if (!ok) return;
    }
    await api.setSystemEnabled(s.id, !s.enabled);
    refresh();
  }

  async function pickCore(s, core) {
    try {
      await api.setSystemCore(s.id, core);
    } finally {
      refresh();
    }
  }

  async function install(core) {
    installing[core] = 0;
    try {
      await api.installCore(core);
      toast(`Installed ${core.replace(/_libretro$/, '')}`, 'info');
    } catch (e) {
      toast(String(e), 'error');
    } finally {
      delete installing[core];
      refresh();
    }
  }

  let ssUser = $state('');
  let ssPass = $state('');
  $effect(() => {
    ssUser = ov?.screenscraper_user ?? '';
  });
  async function saveSs() {
    await api.setScreenscraper(ssUser, ssPass);
    ssPass = '';
    await refresh();
    toast(ssUser ? 'ScreenScraper login saved' : 'ScreenScraper login removed');
  }

  async function toggleArt() {
    await api.setFetchArt(!ov.fetch_art);
    await refresh();
    if (ov.fetch_art) api.fetchArt().catch(() => {});
  }

  async function browseDats() {
    importDats(await api.pickFiles('Choose DAT files'));
  }

  const importFiles = importBios;

  async function browse() {
    importFiles(await api.pickFiles());
  }

  onMount(() => {
    let off = () => {};
    let offProgress = () => {};
    api.onCoreProgress((p) => {
      if (p.core in installing) installing[p.core] = p.percent;
    }).then((f) => (offProgress = f));
    api
      .onDragDrop((p) => {
        if (p.type === 'over' || p.type === 'enter') over = true;
        else if (p.type === 'leave') over = false;
        else if (p.type === 'drop') {
          over = false;
          importFiles(p.paths ?? []);
        }
      })
      .then((f) => (off = f));
    return () => {
      off();
      offProgress();
    };
  });
</script>

<section class="page">
  <h1>Cores &amp; BIOS</h1>
  <p class="sub">Turn systems on or off and add the BIOS files they need.</p>
  {#if ov?.machine}
    <p class="machine">This computer: <strong>{ov.machine.cpu_name}</strong> · {ov.machine.cpu_cores} processor cores · {ov.machine.ram_gb.toFixed(0)} GB memory</p>
  {/if}

  <div class="cards">
    {#each ov?.systems ?? [] as s (s.id)}
      {@const [label, kind] = statusOf(s)}
      <article style="--c:{colorOf(s.id)}" class:off={!s.enabled}>
        <header>
          <h3>{s.name}</h3>
          <span class="status {kind}">● {label}</span>
        </header>
        {#if s.hardware}
          <div class="three-d">
            <span class="tag">3D system</span>
            {#if s.assessment}
              <span class="lvl {LEVELS[s.assessment.level][1]}">● {LEVELS[s.assessment.level][0]}</span>
              <ul>{#each s.assessment.reasons as r}<li>{r}</li>{/each}</ul>
            {/if}
          </div>
        {/if}
        <dl>
          <dt>Core</dt>
          <dd>
            {#if (s.core_choices?.length ?? 0) > 1}
              <Select compact value={s.core} ariaLabel="Core for {s.name}" onchange={(v) => pickCore(s, v)}
                options={s.core_choices.map((c) => ({ value: c.name, label: c.name.replace(/_libretro$/, ''), hint: c.installed ? '' : 'not installed' }))} />
            {:else}
              <span class="mono">{s.core}</span>
            {/if}
          </dd>
          <dt>Files</dt><dd class="mono">{s.extensions.map((e) => '.' + e).join(' ')}</dd>
          <dt>Games</dt><dd>{s.game_count}</dd>
          <dt>BIOS</dt>
          <dd>
            {#each s.bios as b (b.file)}
              <div class={b.state === 'ok' ? 'ok' : b.required ? 'bad' : 'muted'}>
                {b.state === 'ok' ? '✓' : b.required ? '✕' : '○'} <span class="mono">{b.file}</span>{b.required ? '' : ' (optional)'}
              </div>
            {:else}
              <span class="muted">none needed</span>
            {/each}
          </dd>
        </dl>
        {#if !s.core_installed}
          { core = s.core}
          <button class="btn primary" disabled={core in installing} onclick={() => install(core)}>
            {core in installing ? `Installing… ${installing[core]}%` : `Install ${core.replace(/_libretro$/, '')}`}
          </button>
        {/if}
        {#if s.core_installed}
          <button class="btn" onclick={() => (optionsFor = { core: s.core, name: s.name })}>Core options…</button>
        {/if}
        <button class="btn" onclick={() => toggle(s)}>{s.enabled ? 'Switch off' : 'Switch on'}</button>
      </article>
    {/each}
  </div>

  <h3 class="sec">Add BIOS files</h3>
  <div class="drop" class:over role="button" tabindex="0" onclick={browse} onkeydown={(e) => e.key === 'Enter' && browse()}>
    <strong>Drop BIOS files here</strong>
    <span>vuretro checks the MD5, then copies them to</span>
    <span class="mono">{ov?.system_dir}</span>
    <span class="muted">or click to browse</span>
  </div>
  <p class="note">vuretro does not provide BIOS files. Use dumps from hardware you own.</p>

  <h3 class="sec">Official game names</h3>
  <p class="sub dat">
    Add No-Intro or Redump DAT files (XML) and vuretro names your games by their hash, and marks duplicates.
    {#if ov?.dat_entries}<strong>{ov.dat_entries.toLocaleString()} known dumps</strong>{:else}<span class="muted">No DAT files added yet.</span>{/if}
  </p>
  <button class="btn" onclick={browseDats}>Add DAT files…</button>
  <label class="art">
    <input type="checkbox" checked={ov?.fetch_art} onchange={toggleArt} />
    Download cover art for named games
    <span class="muted">(sends game names to thumbnails.libretro.com; needs DAT files)</span>
  </label>
  {#if ov?.art_sources?.includes('screenscraper')}
    <div class="ss">
      <strong>ScreenScraper</strong>
      <span class="muted">Used when libretro-thumbnails has no picture. Needs your free screenscraper.fr account; the password is stored as plain text in the app's settings file.</span>
      <div class="ssrow">
        <input placeholder="User name" bind:value={ssUser} aria-label="ScreenScraper user name" autocomplete="off" />
        <input type="password" placeholder={ov.screenscraper_user ? '(password saved)' : 'Password'} bind:value={ssPass} aria-label="ScreenScraper password" autocomplete="off" />
        <button class="btn" onclick={saveSs}>{ssUser ? 'Save' : 'Remove'}</button>
      </div>
    </div>
  {:else}
    <p class="note">ScreenScraper is not available in this build (it needs developer credentials from the project).</p>
  {/if}
</section>

{#if optionsFor}
  <CoreOptions core={optionsFor.core} name={optionsFor.name} onclose={() => (optionsFor = null)} />
{/if}

<style>
  .page { flex: 1; overflow: auto; padding: 32px 40px 56px; min-width: 0; }
  .page > :global(*) { max-width: 1040px; margin-left: auto; margin-right: auto; }
  h1 { font-size: 26px; }
  .sub { color: var(--muted); margin: 6px 0 24px; }
  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(250px, 1fr)); gap: 16px; }
  article {
    background: var(--surface); border: 1px solid color-mix(in srgb, var(--c) 55%, var(--border));
    border-radius: 10px; padding: 16px; display: flex; flex-direction: column; gap: 12px;
  }
  article .btn { margin-top: auto; }
  article.off { opacity: .55; border-color: var(--border); }
  header { display: flex; justify-content: space-between; align-items: baseline; }
  header h3 { color: var(--c); font-size: 16px; }
  .status { font-size: 11px; }
  .status.ok, .ok { color: var(--ok); }
  .status.bad, .bad { color: var(--bad); }
  .muted { color: var(--muted); }
  .machine { color: var(--muted); margin: -14px 0 20px; }
  .three-d { font-size: 12px; display: flex; flex-direction: column; gap: 4px; }
  .three-d ul { margin: 0; padding-left: 16px; color: var(--muted); }
  .tag { align-self: flex-start; background: var(--surface-2); border-radius: 999px; padding: 1px 8px; font-size: 11px; color: var(--muted); }
  .lvl.ok { color: var(--ok); }
  .lvl.warn { color: var(--warn, #ffb347); }
  .lvl.bad { color: var(--bad); }
  dl { display: grid; grid-template-columns: 56px 1fr; gap: 6px 8px; margin: 0; }
  dt { color: var(--muted); }
  dd { margin: 0; min-width: 0; word-break: break-all; }
  .sec { margin: 34px auto 12px; font-size: 13px; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); font-weight: 600; }
  .drop {
    border: 1.5px dashed var(--accent); border-radius: 10px; padding: 30px; text-align: center;
    display: flex; flex-direction: column; gap: 4px; align-items: center; color: var(--muted); cursor: pointer;
    transition: background .15s;
  }
  .drop strong { color: var(--accent); font-size: 15px; }
  .drop:hover, .drop.over { background: color-mix(in srgb, var(--accent) 8%, transparent); }
  .dat { margin: 0 auto 12px; }
  .ss { margin-top: 14px; display: flex; flex-direction: column; gap: 6px; }
  .ssrow { display: flex; gap: 8px; flex-wrap: wrap; }
  .ssrow input { background: var(--bg); color: var(--text); border: 1px solid var(--border); border-radius: 8px; padding: 6px 10px; font: inherit; }
  .art { display: flex; gap: 8px; align-items: center; margin-top: 12px; }
  .note { color: var(--faint); margin-top: 10px; }
</style>
