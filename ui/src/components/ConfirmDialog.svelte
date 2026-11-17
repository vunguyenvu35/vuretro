<script>
  import { dlg, answerDialog } from '../lib/store.svelte.js';

  let cancelBtn = $state();
  $effect(() => {
    if (dlg.open) cancelBtn?.focus(); // the safe choice has the focus
  });

  function onkeydown(e) {
    if (!dlg.open) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      answerDialog(false);
    }
  }
</script>

<svelte:window {onkeydown} />

{#if dlg.open}
  <div class="scrim" role="presentation" onmousedown={() => answerDialog(false)}>
    <div class="dialog" class:danger={dlg.danger} role="alertdialog" aria-modal="true" aria-labelledby="dlg-t" aria-describedby="dlg-m" tabindex="-1" onmousedown={(e) => e.stopPropagation()}>
      <div class="icon" aria-hidden="true">
        {#if dlg.danger}
          <svg viewBox="0 0 24 24" width="26" height="26"><path d="M4.5 7h15M9.5 7V5.2c0-.4.3-.7.7-.7h3.6c.4 0 .7.3.7.7V7M6.8 7l.8 11.2c0 .5.4.8.9.8h7c.5 0 .9-.3.9-.8L17.2 7M10 11v5M14 11v5" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" /></svg>
        {:else}
          <svg viewBox="0 0 24 24" width="26" height="26"><circle cx="12" cy="12" r="8.5" fill="none" stroke="currentColor" stroke-width="1.7" /><path d="M12 8v5m0 3v.01" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" /></svg>
        {/if}
      </div>
      <h2 id="dlg-t">{dlg.title}</h2>
      <p id="dlg-m" class="msg">{dlg.message}</p>
      {#if dlg.note}
        <div class="note">
          <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="6.6" fill="none" stroke="currentColor" stroke-width="1.4" /><path d="M5.2 8.2l2 2 3.6-4" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" /></svg>
          <span>{dlg.note}</span>
        </div>
      {/if}
      <div class="buttons">
        <button class="b cancel" bind:this={cancelBtn} onclick={() => answerDialog(false)}>{dlg.cancelLabel}</button>
        <button class="b go" onclick={() => answerDialog(true)}>{dlg.confirmLabel}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .scrim { position: fixed; inset: 0; z-index: 100; display: grid; place-items: center; padding: 24px; background: rgba(3, 5, 9, .66); backdrop-filter: blur(4px); animation: fade .15s ease-out; }
  @keyframes fade { from { opacity: 0; } }
  .dialog {
    --tone: var(--accent);
    width: min(400px, 100%); text-align: center; padding: 30px 26px 24px; outline: none;
    background: linear-gradient(180deg, color-mix(in srgb, var(--surface-2) 70%, var(--surface)), var(--surface));
    border: 1px solid var(--border); border-radius: 22px;
    box-shadow: 0 40px 90px rgba(0, 0, 0, .7), 0 0 0 1px rgba(255, 255, 255, .02) inset;
    animation: rise .2s cubic-bezier(.3, .8, .2, 1);
  }
  .dialog.danger { --tone: #ff5a5f; }
  @keyframes rise { from { opacity: 0; transform: translateY(10px) scale(.97); } }

  .icon {
    width: 60px; height: 60px; margin: 0 auto 18px; border-radius: 50%; display: grid; place-items: center; color: var(--tone);
    background: radial-gradient(circle at 50% 35%, color-mix(in srgb, var(--tone) 26%, transparent), color-mix(in srgb, var(--tone) 10%, transparent));
    box-shadow: 0 0 0 6px color-mix(in srgb, var(--tone) 8%, transparent), 0 8px 28px color-mix(in srgb, var(--tone) 28%, transparent);
  }
  h2 { font-size: 20px; letter-spacing: -.01em; margin: 0 0 8px; }
  .msg { margin: 0 auto; max-width: 32ch; color: var(--muted); font-size: 14px; line-height: 1.5; white-space: pre-line; }

  .note {
    display: flex; align-items: flex-start; gap: 10px; text-align: left; margin: 18px 0 0; padding: 11px 14px;
    background: color-mix(in srgb, var(--ok) 7%, var(--bg)); border: 1px solid color-mix(in srgb, var(--ok) 22%, var(--border)); border-radius: 12px;
    color: color-mix(in srgb, var(--ok) 35%, var(--text)); font-size: 13px; line-height: 1.45;
  }
  .note svg { flex: none; margin-top: 1px; color: var(--ok); }

  .buttons { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; margin-top: 24px; }
  .b { height: 44px; border-radius: 12px; font: inherit; font-size: 14px; font-weight: 600; border: 1px solid transparent; transition: transform .08s, filter .15s, background .15s; }
  .b:active { transform: scale(.98); }
  .b:focus-visible { outline: 2px solid color-mix(in srgb, var(--accent) 80%, transparent); outline-offset: 2px; }
  .cancel { background: var(--surface-2); border-color: var(--border); color: var(--text); }
  .cancel:hover { background: color-mix(in srgb, var(--surface-2) 70%, var(--text) 6%); }
  .go { background: var(--tone); color: #17110a; box-shadow: 0 6px 20px color-mix(in srgb, var(--tone) 35%, transparent); }
  .danger .go { background: linear-gradient(180deg, #ff6b6f, #ef4146); color: #fff; }
  .go:hover { filter: brightness(1.08); }
</style>
