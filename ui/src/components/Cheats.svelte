<script>
  import { api } from '../lib/api.js';
  import { toast } from '../lib/ui.svelte.js';

  let { game } = $props();

  let text = $state('');
  let saved = $state('');
  let open = $state(false);

  $effect(() => {
    const path = game.path;
    let stale = false;
    api.cheats(path).then((t) => {
      if (!stale) text = saved = t;
    });
    return () => (stale = true);
  });

  const dirty = $derived(text !== saved);
  const count = $derived(text.split('\n').filter((l) => l.trim() && !l.trim().startsWith('#') && !l.trim().startsWith('-')).length);

  async function save() {
    try {
      await api.setCheats(game.path, text);
      saved = text;
      toast('Cheats saved. They apply the next time the game starts.');
    } catch (e) {
      toast(String(e), 'error');
    }
  }
</script>

<details bind:open>
  <summary class="label">Cheats{count ? ` · ${count} on` : ''}</summary>
  <textarea
    bind:value={text}
    rows="5"
    spellcheck="false"
    aria-label="Cheat codes"
    placeholder={'One code per line, as the core expects it.\n# a note\n-CODE  starts with "-" to switch it off'}
  ></textarea>
  <button class="btn" disabled={!dirty} onclick={save}>Save cheats</button>
  <p class="hint">Only works with cores that support cheats.</p>
</details>

<style>
  details { margin: 12px 0 8px; }
  summary { cursor: pointer; }
  textarea { width: 100%; margin: 8px 0; background: var(--bg); color: var(--text); border: 1px solid var(--border); border-radius: 8px; padding: 8px; font: 12px var(--font-mono); resize: vertical; box-sizing: border-box; user-select: text; }
  .hint { color: var(--faint); font-size: 11px; margin: 6px 0 0; }
</style>
