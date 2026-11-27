<script>
  import { app } from '../lib/store.svelte.js';

  /** Shown while Play fetches a core the game needs, so the wait is never a silent one. */
  const d = $derived(app.downloading);
  const name = $derived(d ? d.core.replace(/_libretro$/, '') : '');
</script>

{#if app.preparing && !d && app.preparing !== 'Starting'}
  <div class="banner" role="status" aria-live="polite">
    <div class="line"><strong>Setting the game up…</strong></div>
    <div class="track"><span class="busy"></span></div>
    <small>{app.preparing}. The first time takes a few minutes; after that it starts at once.</small>
  </div>
{/if}

{#if d}
  <div class="banner" role="status" aria-live="polite">
    <div class="line"><strong>Getting the {name} core…</strong> <span>{d.percent}%</span></div>
    <div class="track"><span style="width:{d.percent}%"></span></div>
    <small>The game starts when it is ready. A large core can take a few minutes.</small>
  </div>
{/if}

<style>
  .banner { position: fixed; left: 20px; bottom: 20px; z-index: 40; width: 320px; background: var(--surface); border: 1px solid var(--border); border-radius: 12px; padding: 12px 14px; box-shadow: 0 8px 28px rgba(0,0,0,.35); display: flex; flex-direction: column; gap: 8px; }
  .line { display: flex; justify-content: space-between; gap: 12px; font-size: 13px; }
  .line span { color: var(--muted); font-variant-numeric: tabular-nums; }
  .track { height: 6px; border-radius: 99px; background: color-mix(in srgb, var(--text) 12%, transparent); overflow: hidden; }
  .track span { display: block; height: 100%; background: var(--accent); transition: width .25s ease; }
  small { color: var(--muted); font-size: 12px; line-height: 1.4; }
.track .busy { width: 35%; animation: slide 1.2s ease-in-out infinite alternate; }
  @keyframes slide { from { margin-left: 0; } to { margin-left: 65%; } }
</style>
