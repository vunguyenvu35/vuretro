<script>
  import { untrack } from 'svelte';
  import { app, renameGame } from '../lib/store.svelte.js';
  import Icon from './Icon.svelte';

  /**
   * The name of the selected game, which turns into a field right where it is: the pencil (or
   * Rename… in the game's menu) starts it, Enter or leaving the field keeps the name, Esc drops
   * it. An empty name gives the original one back. The file on disk keeps its own name.
   */
  let { game } = $props();

  const MAX = 120;
  const editing = $derived(app.renaming === game.path);
  let draft = $state('');
  let error = $state('');
  let input = $state();
  let finished = false;

  // when editing starts: the current name in the field, selected so typing replaces it
  $effect(() => {
    if (!editing) return;
    untrack(() => {
      draft = game.title;
      error = '';
      finished = false;
    });
    const t = setTimeout(() => {
      input?.focus();
      input?.select();
    }, 0);
    return () => clearTimeout(t);
  });

  function stop() {
    finished = true;
    app.renaming = null;
  }

  async function commit(next = draft) {
    if (finished) return;
    const name = next === null ? null : next.trim();
    if (name !== null && name === game.title) return stop();
    finished = true;
    try {
      await renameGame(game, name || null);
      app.renaming = null;
    } catch (e) {
      // keep the field open with what was typed, and say why
      error = String(e);
      finished = false;
      input?.focus();
    }
  }

  function key(e) {
    // typing here is not for the library's shortcuts (Delete removes a game)
    e.stopPropagation();
    if (e.key === 'Enter') {
      e.preventDefault();
      commit();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      stop();
    }
  }
</script>

<div class="titlerow">
  {#if editing}
    <input
      class="field"
      bind:this={input}
      bind:value={draft}
      maxlength={MAX}
      onkeydown={key}
      onblur={() => commit()}
      autocomplete="off"
      spellcheck="false"
      aria-label="Name in the library"
      aria-invalid={error ? 'true' : undefined}
    />
  {:else}
    <h2>{game.title}</h2>
    <button type="button" class="iconbtn" aria-label="Rename {game.title}" title="Rename" onclick={() => (app.renaming = game.path)}>
      <Icon name="edit" size={16} />
    </button>
  {/if}
</div>
{#if editing && error}
  <p class="err" role="alert">{error}</p>
{:else if editing}
  <p class="hint">Enter to keep · Esc to cancel · the file keeps its own name</p>
{:else if game.renamed}
  <p class="hint">Originally <b>{game.original_title}</b> · <button type="button" class="link" onclick={() => commit(null)}>Use original name</button></p>
{/if}

<style>
  .titlerow { display: flex; align-items: flex-start; gap: 6px; margin-top: 8px; min-width: 0; }
  h2 { font-size: 20px; margin: 0; min-width: 0; overflow-wrap: anywhere; }
  .iconbtn { flex: none; display: grid; place-items: center; width: 26px; height: 26px; margin-top: -1px; padding: 0; background: none; border: 1px solid transparent; border-radius: 7px; color: var(--muted); }
  .iconbtn:hover { color: var(--text); background: var(--surface-2); border-color: var(--border); }
  /* the field takes the place of the name and looks like it, with a frame to say it can be typed in */
  .field {
    flex: 1; min-width: 0; margin: -4px 0 0 -9px; padding: 3px 8px; font: 700 20px/1.3 var(--font-head);
    color: var(--text); background: var(--surface-2); border: 1px solid color-mix(in srgb, var(--accent) 70%, var(--border)); border-radius: 8px;
  }
  .field:focus-visible { outline: 2px solid color-mix(in srgb, var(--accent) 45%, transparent); outline-offset: 1px; }
  .field[aria-invalid='true'] { border-color: var(--bad); }
  .hint, .err { margin: 6px 0 0; font-size: 11.5px; line-height: 1.45; overflow-wrap: anywhere; }
  .hint { color: var(--faint); }
  .hint b { color: var(--muted); font-weight: 600; }
  .err { color: var(--bad); }
  .link { background: none; border: 0; padding: 0; font: inherit; color: var(--accent); text-decoration: underline; text-underline-offset: 2px; }
</style>
