<script>
  import { app } from '../lib/store.svelte.js';
  import { toast } from '../lib/ui.svelte.js';
  import { api } from '../lib/api.js';

  // the keys that stop the game from anywhere (registered by the app while a game is open)
  const keys = /Mac/i.test(navigator.platform) ? '⌘⌥Q' : 'Ctrl+Alt+Q';

  async function stop() {
    try {
      await api.stopGame();
    } catch (e) {
      toast(String(e), 'error');
    }
  }
</script>

{#if app.playing}
  <div class="banner" role="status" aria-live="polite">
    <span class="name"
      ><strong>Playing</strong> {app.playing}<small
        >{app.playingNote ? `Runs in ${app.playingNote} (the Dock shows “${app.playingNote}”). ` : ''}{keys} stops it from anywhere</small
      ></span
    >
    <button class="stop" onclick={stop}>Stop</button>
  </div>
{/if}

<style>
  .banner { position: fixed; right: 20px; bottom: 20px; z-index: 40; max-width: 360px; background: var(--surface); border: 1px solid var(--border); border-radius: 12px; padding: 10px 12px 10px 14px; box-shadow: 0 8px 28px rgba(0,0,0,.35); display: flex; align-items: center; gap: 12px; font-size: 13px; }
  .name { overflow: hidden; display: flex; flex-direction: column; gap: 2px; }
  small { color: var(--muted); font-size: 11px; }
  .stop { flex: none; padding: 6px 14px; border-radius: 8px; border: 1px solid var(--border); background: color-mix(in srgb, var(--text) 8%, transparent); color: var(--text); cursor: pointer; }
  .stop:hover { background: color-mix(in srgb, var(--text) 16%, transparent); }
</style>
