<script>
  import { app } from '../lib/store.svelte.js';
  import Select from './Select.svelte';
  import { parseScope } from '../lib/scope.js';

  /** What the screen below is about: every game, or one system. */
  let { hint = '' } = $props();

  const game = $derived.by(() => {
    const p = parseScope(app.scope);
    return p.kind === 'game' ? app.overview?.games.find((g) => g.path === p.id) : null;
  });
  const options = $derived([
    { value: '', label: 'All games', hint: 'general' },
    ...(app.overview?.systems ?? []).map((s) => ({ value: s.id, label: s.name })),
    ...(game ? [{ value: app.scope, label: game.title, hint: 'this game' }] : []),
  ]);
</script>

<div class="scope">
  <span class="lab">Applies to</span>
  <Select bind:value={app.scope} {options} searchable ariaLabel="Settings apply to" minWidth={230} />
  {#if hint}<span class="hint">{hint}</span>{/if}
</div>

<style>
  .scope { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; margin: 4px 0 20px; padding: 10px 14px; background: var(--surface); border: 1px solid var(--border); border-radius: 12px; }
  .lab { color: var(--muted); font-size: 12px; text-transform: uppercase; letter-spacing: .08em; }
  .hint { color: var(--faint); font-size: 12px; }
</style>
