<script>
  /** A small dialog: a title, the content, and a Close button. Esc or a click outside closes it. */
  let { title, onclose = () => {}, wide = false, children } = $props();

  let closeBtn = $state();
  $effect(() => closeBtn?.focus());

  function onkeydown(e) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onclose();
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="scrim" role="presentation" onmousedown={onclose}>
  <div class="dialog" class:wide role="dialog" aria-modal="true" aria-label={title} tabindex="-1" onmousedown={(e) => e.stopPropagation()}>
    <h2>{title}</h2>
    <div class="body">{@render children()}</div>
    <footer><button class="btn" bind:this={closeBtn} onclick={onclose}>Close</button></footer>
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; background: rgba(5, 7, 12, .6); display: grid; place-items: center; z-index: 60; }
  .dialog { width: min(440px, 92vw); max-height: 86vh; overflow: auto; display: flex; flex-direction: column; gap: 14px; background: var(--surface); border: 1px solid var(--border); border-radius: 14px; padding: 22px; }
  .dialog.wide { width: min(620px, 92vw); }
  h2 { font-size: 18px; text-align: center; }
  .body { display: grid; gap: 14px; justify-items: center; text-align: center; }
  footer { display: flex; justify-content: center; }
</style>
