<script>
  import { onMount } from 'svelte';
  import { app, refresh, importBios } from '../lib/store.svelte.js';
  import { api } from '../lib/api.js';
  import { colorOf } from '../lib/systems.js';
  import Select from './Select.svelte';
  import { toast } from '../lib/ui.svelte.js';

  let over = $state(false);
  /** core name -> percent while it downloads */
  let installing = $state({});
  const ov = $derived(app.overview);

  const statusOf = (s) =>
    !s.enabled ? ['Off', 'muted'] : !s.core_installed ? ['Core missing', 'bad'] : !s.bios_ready ? ['BIOS missing', 'bad'] : ['Ready', 'ok'];

  async function toggle(s) {
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

  <div class="cards">
    {#each ov?.systems ?? [] as s (s.id)}
      {@const [label, kind] = statusOf(s)}
      <article style="--c:{colorOf(s.id)}" class:off={!s.enabled}>
        <header>
          <h3>{s.name}</h3>
          <span class="status {kind}">● {label}</span>
        </header>
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
</section>

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
  .note { color: var(--faint); margin-top: 10px; }
</style>
