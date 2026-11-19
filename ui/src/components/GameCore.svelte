<script>
  import { api } from '../lib/api.js';
  import { app, refresh } from '../lib/store.svelte.js';
  import { toast } from '../lib/ui.svelte.js';
  import Select from './Select.svelte';

  /** Which core runs one game: its system's, or one of its own (a ROM set fits one version). */
  let { game } = $props();

  const system = $derived(app.overview?.systems.find((s) => s.id === game.system));
  const choices = $derived(system?.core_choices ?? []);
  const short = (c) => c.replace(/_libretro$/, '');

  async function choose(core) {
    try {
      await api.setGameCore(game.path, core || null);
      await refresh();
      toast(core ? `${game.title} runs on ${short(core)}` : `${game.title} runs on the core of its system`);
    } catch (e) {
      toast(String(e), 'error');
    }
  }
</script>

{#if choices.length > 1}
  <div class="gc">
    <div class="label">Core</div>
    <div class="row">
      <span class="what">Runs on</span>
      <div class="sel">
        <Select
          value={game.core ?? ''}
          onchange={choose}
          ariaLabel="Core for this game"
          options={[
            { value: '', label: `${short(system.core)} (system${system.core_installed ? '' : ', not installed'})` },
            ...choices.filter((c) => c.name !== system.core).map((c) => ({ value: c.name, label: short(c.name) + (c.installed ? '' : ' (not installed)') })),
          ]}
        />
      </div>
    </div>
    <p class="note">The ROM set has to match the core's version. Pick another core here for this game only; the rest keep the system's.</p>
  </div>
{/if}

<style>
  .gc { display: flex; flex-direction: column; gap: 8px; }
  .label { font-size: 11px; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); font-weight: 600; }
  .row { display: flex; align-items: center; gap: 10px; justify-content: space-between; }
  .what { color: var(--muted); font-size: 13px; }
  .sel { flex: 1; min-width: 0; max-width: 220px; }
  .note { margin: 0; color: var(--muted); font-size: 12px; line-height: 1.45; }
</style>
