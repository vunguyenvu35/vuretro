<script>
  import { api } from '../lib/api.js';
  import { app } from '../lib/store.svelte.js';
  import { toast } from '../lib/ui.svelte.js';
  import { gameKey, profilesOf } from '../lib/macros.js';
  import Select from './Select.svelte';

  /** The macros of one game in its details: which profile plays, and the way into the Macros screen. */
  let { game } = $props();

  let cfg = $state(null);
  const key = $derived(gameKey(game.path));
  const info = $derived(profilesOf(cfg, key));
  const shared = $derived(cfg?.games?.[key]?.length ?? 0);
  const inUse = $derived(info.profiles.find((p) => p.id === info.active) ?? null);

  async function load() {
    try {
      cfg = await api.macros();
    } catch {
      cfg = null;
    }
  }
  $effect(() => {
    game.path;
    load();
  });

  async function choose(id) {
    try {
      cfg = await api.setMacroProfile(game.path, id || null);
      toast(id ? `Macro profile “${info.profiles.find((p) => p.id === id)?.name ?? ''}” is in use` : 'Only the shared macros play now');
    } catch (e) {
      toast(String(e), 'error');
    }
  }
</script>

<div class="mp">
  <div class="label">Macros</div>
  {#if info.profiles.length}
    <div class="row">
      <span class="what">Profile in use</span>
      <div class="sel">
        <Select
          value={info.active}
          onchange={choose}
          ariaLabel="Macro profile in use"
          options={[{ value: '', label: 'None (shared macros only)' }, ...info.profiles.map((p) => ({ value: p.id, label: p.name || 'Profile' }))]}
        />
      </div>
    </div>
    <p class="note">
      {inUse ? `“${inUse.name}” plays with ${shared} shared macro${shared === 1 ? '' : 's'}.` : `Only the ${shared} shared macro${shared === 1 ? '' : 's'} play.`}
    </p>
  {:else}
    <p class="note">{shared ? `${shared} macro${shared === 1 ? '' : 's'} for this game, shared by every character.` : 'No macros for this game yet.'}</p>
  {/if}
  <button class="btn wide" onclick={() => (app.view = 'macros')}>Macros for this game…</button>
</div>

<style>
  .mp { display: flex; flex-direction: column; gap: 8px; }
  .label { font-size: 11px; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); font-weight: 600; }
  .row { display: flex; align-items: center; gap: 10px; justify-content: space-between; }
  .what { color: var(--muted); font-size: 13px; }
  .sel { flex: 1; min-width: 0; max-width: 220px; }
  .note { margin: 0; color: var(--muted); font-size: 12px; line-height: 1.45; }
</style>
