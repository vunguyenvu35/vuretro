<script>
  import { api } from '../lib/api.js';
  import { play } from '../lib/store.svelte.js';
  import { formatAgo } from '../lib/systems.js';

  /** The selected game; `version` changes when the library was refreshed (a game closed). */
  let { game, version = 0 } = $props();

  let slots = $state([]);
  let thumbs = $state({});
  // a slot saved or loaded in the running game: look again, without waiting for it to close
  let reported = $state(0);

  $effect(() => {
    let off;
    let gone = false;
    api
      .onGameEvent((e) => {
        if (e.event === 'state') reported++;
      })
      .then((f) => (gone ? f() : (off = f)));
    return () => {
      gone = true;
      off?.();
    };
  });

  $effect(() => {
    const path = game.path;
    void version;
    void reported;
    let stale = false;
    api.saveSlots(path).then(async (list) => {
      const pics = {};
      await Promise.all(
        list.filter((s) => s.has_thumb).map(async (s) => (pics[s.slot] = await api.slotThumbnail(path, s.slot))),
      );
      if (!stale) {
        slots = list;
        thumbs = pics;
      }
    });
    return () => (stale = true);
  });
</script>

{#if slots.length}
  <h3 class="label">Continue from a save</h3>
  <div class="slots">
    {#each slots as s (s.slot)}
      <button class="slot" disabled={!!game.blocked} onclick={() => play(game, s.slot)} title="Start from slot {s.slot}">
        {#if thumbs[s.slot]}<img src={thumbs[s.slot]} alt="" />{:else}<span class="blank">no picture</span>{/if}
        <span class="cap">Slot {s.slot} · {formatAgo(s.saved_at)}</span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .label { margin: 12px 0 6px; }
  .slots { display: grid; grid-template-columns: repeat(2, 1fr); gap: 8px; }
  .slot { position: relative; padding: 0; overflow: hidden; border: 1px solid var(--border); border-radius: 8px; background: var(--surface); aspect-ratio: 4 / 3; cursor: pointer; }
  .slot:hover:not(:disabled) { border-color: var(--accent); }
  .slot img { width: 100%; height: 100%; object-fit: cover; image-rendering: pixelated; display: block; }
  .blank { display: grid; place-items: center; height: 100%; color: var(--faint); font-size: 11px; }
  .cap { position: absolute; left: 0; right: 0; bottom: 0; padding: 3px 6px; font-size: 11px; text-align: left; color: #fff; background: linear-gradient(transparent, rgba(0, 0, 0, .75)); }
</style>
