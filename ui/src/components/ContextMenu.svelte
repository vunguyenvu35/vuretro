<script>
  import { ctx, closeMenu } from '../lib/ui.svelte.js';

  let el = $state();
  let pos = $state({ x: 0, y: 0 });

  // Keep the menu inside the window: measure it, then flip or shift as needed.
  $effect(() => {
    if (!ctx.open || !el) return;
    const { width, height } = el.getBoundingClientRect();
    const pad = 8;
    pos = {
      x: Math.max(pad, Math.min(ctx.x, window.innerWidth - width - pad)),
      y: Math.max(pad, Math.min(ctx.y, window.innerHeight - height - pad)),
    };
    el.querySelector('button:not(:disabled)')?.focus({ preventScroll: true });
  });

  function run(item) {
    if (item.disabled) return;
    closeMenu();
    item.action?.();
  }

  function onkeydown(e) {
    if (!ctx.open) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      return closeMenu();
    }
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
    e.preventDefault();
    const items = [...el.querySelectorAll('button:not(:disabled)')];
    const i = items.indexOf(document.activeElement);
    const next = e.key === 'ArrowDown' ? (i + 1) % items.length : (i - 1 + items.length) % items.length;
    items[next]?.focus();
  }
</script>

<svelte:window {onkeydown} onresize={closeMenu} onblur={closeMenu} />

{#if ctx.open}
  <!-- a transparent layer catches the click that dismisses the menu -->
  <div class="scrim" role="presentation" onmousedown={closeMenu} oncontextmenu={(e) => { e.preventDefault(); closeMenu(); }}></div>
  <div class="menu" role="menu" bind:this={el} style="left:{pos.x}px; top:{pos.y}px" oncontextmenu={(e) => e.preventDefault()}>
    {#if ctx.title}<div class="mtitle">{ctx.title}</div>{/if}
    {#each ctx.items as it, i (i)}
      {#if it.separator}
        <div class="sep" role="separator"></div>
      {:else}
        <button role="menuitem" class:danger={it.danger} disabled={it.disabled} onclick={() => run(it)}>
          <span class="l">{it.label}</span>
          {#if it.hint}<kbd>{it.hint}</kbd>{/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .scrim { position: fixed; inset: 0; z-index: 90; }
  .menu {
    position: fixed; z-index: 91; min-width: 220px; max-width: 320px; padding: 6px;
    background: color-mix(in srgb, var(--surface-2) 92%, transparent); backdrop-filter: blur(14px);
    border: 1px solid var(--border); border-radius: 12px;
    box-shadow: 0 18px 50px rgba(0, 0, 0, .6), 0 2px 0 rgba(255, 255, 255, .03) inset;
    animation: pop .12s cubic-bezier(.3, .8, .2, 1); transform-origin: top left;
  }
  @keyframes pop { from { opacity: 0; transform: scale(.97); } }
  .mtitle { padding: 6px 10px 8px; color: var(--faint); font-size: 11px; text-transform: uppercase; letter-spacing: .07em; max-width: 280px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  button { width: 100%; display: flex; align-items: center; justify-content: space-between; gap: 18px; background: none; border: 0; border-radius: 8px; padding: 7px 10px; color: var(--text); font-size: 13px; text-align: left; }
  button:hover:not(:disabled), button:focus-visible { background: color-mix(in srgb, var(--accent) 16%, transparent); outline: none; }
  button.danger { color: var(--bad); }
  button.danger:hover:not(:disabled), button.danger:focus-visible { background: color-mix(in srgb, var(--bad) 16%, transparent); }
  button:disabled { color: var(--faint); }
  kbd { font: 600 11px var(--font-mono); color: var(--faint); background: none; border: 0; padding: 0; }
  .sep { height: 1px; background: var(--border); margin: 5px 4px; }
</style>
