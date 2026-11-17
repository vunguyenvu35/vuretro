<script>
  import { toasts, dismissToast } from '../lib/store.svelte.js';

  // "✓ 3 games added" style lines: the symbol becomes a proper icon instead.
  const lines = (text) => text.split('\n');
  const strip = (l) => l.replace(/^[✓✕]\s*/, '');
  const mark = (l) => (l.startsWith('✓') ? 'ok' : l.startsWith('✕') ? 'bad' : '');
</script>

<div class="stack" role="region" aria-label="Notifications" aria-live="polite">
  {#each toasts as t (t.id)}
    <div class="toast {t.kind}" role={t.kind === 'error' ? 'alert' : 'status'}>
      <span class="ico" aria-hidden="true">
        {#if t.kind === 'error'}
          <svg viewBox="0 0 20 20" width="18" height="18"><path d="M10 3.2l7.2 12.6H2.8L10 3.2zM10 8.4v3.4m0 2.2v.01" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></svg>
        {:else}
          <svg viewBox="0 0 20 20" width="18" height="18"><circle cx="10" cy="10" r="7.4" fill="none" stroke="currentColor" stroke-width="1.6" /><path d="M6.8 10.3l2.2 2.2 4.2-4.6" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" /></svg>
        {/if}
      </span>
      <div class="body">
        {#each lines(t.text) as l, i (i)}
          <div class="line {mark(l)}" class:first={i === 0}>{strip(l)}</div>
        {/each}
      </div>
      <button class="x" onclick={() => dismissToast(t.id)} aria-label="Dismiss">✕</button>
      <!-- the bar runs down the visible time; when it ends the toast goes. Hovering pauses it. -->
      <span class="bar" style="animation-duration:{t.ms}ms" onanimationend={() => dismissToast(t.id)}></span>
    </div>
  {/each}
</div>

<style>
  .stack {
    position: fixed; right: 20px; bottom: 20px; z-index: 120; display: flex; flex-direction: column; align-items: flex-end; gap: 10px;
    width: min(380px, calc(100vw - 40px)); pointer-events: none;
  }
  .toast {
    position: relative; width: 100%; display: flex; align-items: flex-start; gap: 12px; padding: 13px 12px 15px 14px; overflow: hidden; pointer-events: auto;
    background: color-mix(in srgb, var(--surface-2) 94%, transparent); backdrop-filter: blur(14px);
    border: 1px solid var(--border); border-radius: 14px; box-shadow: 0 16px 44px rgba(0, 0, 0, .55);
    animation: in .24s cubic-bezier(.3, .8, .2, 1);
  }
  @keyframes in { from { opacity: 0; transform: translateX(24px); } }
  .ico { flex: none; width: 30px; height: 30px; border-radius: 9px; display: grid; place-items: center; color: var(--ok); background: color-mix(in srgb, var(--ok) 14%, transparent); }
  .toast.error .ico { color: var(--bad); background: color-mix(in srgb, var(--bad) 15%, transparent); }
  .body { flex: 1; min-width: 0; padding-top: 5px; font-size: 13px; line-height: 1.45; user-select: text; }
  .line { color: var(--muted); overflow-wrap: anywhere; }
  .line.first { color: var(--text); font-weight: 500; }
  .line.ok { color: var(--ok); }
  .line.bad { color: var(--bad); }
  .toast .line + .line { margin-top: 2px; }
  .x { flex: none; background: none; border: 0; color: var(--faint); width: 24px; height: 24px; border-radius: 7px; font-size: 11px; padding: 0; margin-top: 3px; }
  .x:hover { color: var(--text); background: color-mix(in srgb, var(--text) 8%, transparent); }
  .bar { position: absolute; left: 0; bottom: 0; height: 2px; width: 100%; background: var(--ok); opacity: .7; transform-origin: left; animation: run linear forwards; }
  .toast.error .bar { background: var(--bad); }
  .toast:hover .bar { animation-play-state: paused; }
  @keyframes run { to { transform: scaleX(0); } }
  @media (prefers-reduced-motion: reduce) { .toast { animation: none; } }
</style>
