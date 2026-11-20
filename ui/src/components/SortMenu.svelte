<script>
  import { app } from '../lib/store.svelte.js';
  import { DEFAULT_SORTS, SORT_OPTIONS, addSort, changeSortKey, moveSort, removeSort, sortDirection, sortWay, turnSort } from '../lib/library.js';
  import Select from './Select.svelte';

  /**
   * How the games are ordered, by one thing or by several: a button that says how, and under it a
   * list of the sorts. The first one is the main one; each of the others settles what the ones
   * before it cannot ("Then by"). The list headings of the table do the same with a click and a
   * Shift+click, but a menu can be seen, and works for the grid, which has no headings.
   */
  let open = $state(false);
  let button = $state();

  const label = (key) => SORT_OPTIONS.find((o) => o.value === key)?.label ?? key;
  const main = $derived(app.sorts[0] ?? DEFAULT_SORTS[0]);
  const many = $derived(app.sorts.length > 1);
  const isDefault = $derived(app.sorts.length === 1 && main.key === DEFAULT_SORTS[0].key && !main.reverse);
  /** The keys sort number `i` can be changed to: the ones the other sorts do not use. */
  const optionsFor = (i) => SORT_OPTIONS.filter((o) => o.value === app.sorts[i].key || !app.sorts.some((s) => s.key === o.value));

  function close() {
    open = false;
    button?.focus();
  }
  // Esc puts the menu away and nothing else (it also puts the game's details away): taken before the library sees it
  function onKeyCapture(e) {
    if (open && e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      close();
    }
  }
</script>

<svelte:window onkeydowncapture={onKeyCapture} />

<div class="wrap">
  <button
    type="button"
    bind:this={button}
    class="trigger"
    class:on={many}
    aria-haspopup="dialog"
    aria-expanded={open}
    title={many ? 'Sorted by several things: click to change' : 'Sort the games, by one thing or by several'}
    onclick={() => (open = !open)}
  >
    <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M5 2.5v10M2.6 10.1L5 12.5l2.4-2.4M11 13.5v-10M8.6 5.9L11 3.5l2.4 2.4" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" /></svg>
    <span class="what">{many ? `${app.sorts.length} sorts` : label(main.key)}</span>
    {#if !many}
      <svg class="dir" class:up={sortDirection(main.key, main.reverse) === 'asc'} viewBox="0 0 10 10" width="9" height="9" aria-hidden="true"><path d="M5 1.6v6.8M2.2 5.8L5 8.6l2.8-2.8" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" /></svg>
    {/if}
  </button>

  {#if open}
    <div class="scrim" role="presentation" onmousedown={close}></div>
    <div class="panel" role="dialog" aria-label="Sort the games">
      <ol>
        {#each app.sorts as s, i (s.key)}
          <li>
            <span class="by">{i === 0 ? 'Sort by' : 'Then by'}</span>
            <div class="row">
              <div class="key">
                <Select compact bind:value={() => s.key, (k) => (app.sorts = changeSortKey(app.sorts, i, k))} options={optionsFor(i)} ariaLabel={i === 0 ? 'Sort by' : `Then by, sort ${i + 1}`} />
              </div>
              <button type="button" class="way" onclick={() => (app.sorts = turnSort(app.sorts, i))} title="Turn it round">
                {sortWay(s.key, s.reverse)}
                <svg class="arrow" class:up={sortDirection(s.key, s.reverse) === 'asc'} viewBox="0 0 10 10" width="10" height="10" aria-hidden="true"><path d="M5 1.6v6.8M2.2 5.8L5 8.6l2.8-2.8" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" /></svg>
              </button>
              <span class="tools">
                <button type="button" class="mini" disabled={i === 0} aria-label="Move this sort up" title="Move up" onclick={() => (app.sorts = moveSort(app.sorts, i, -1))}>
                  <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true"><path d="M2 6.2L5 3.2l3 3" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></svg>
                </button>
                <button type="button" class="mini" disabled={i === app.sorts.length - 1} aria-label="Move this sort down" title="Move down" onclick={() => (app.sorts = moveSort(app.sorts, i, 1))}>
                  <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true"><path d="M2 3.8l3 3 3-3" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></svg>
                </button>
                <button type="button" class="mini" disabled={app.sorts.length === 1} aria-label="Remove this sort" title="Remove" onclick={() => (app.sorts = removeSort(app.sorts, i))}>
                  <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true"><path d="M2.2 2.2l5.6 5.6M7.8 2.2L2.2 7.8" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg>
                </button>
              </span>
            </div>
          </li>
        {/each}
      </ol>
      <div class="foot">
        <button type="button" class="add" disabled={app.sorts.length >= SORT_OPTIONS.length} onclick={() => (app.sorts = addSort(app.sorts))}>+ Add another sort</button>
        <button type="button" class="reset" disabled={isDefault} onclick={() => (app.sorts = [...DEFAULT_SORTS])}>Reset</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .wrap { position: relative; flex: none; }
  .trigger {
    display: inline-flex; align-items: center; gap: 7px; height: 30px; padding: 0 10px; border-radius: 8px;
    background: none; border: 1px solid var(--border); color: var(--muted); font-size: 12px; white-space: nowrap;
  }
  .trigger:hover, .trigger[aria-expanded='true'] { color: var(--text); border-color: var(--faint); }
  .trigger.on { color: var(--accent); border-color: color-mix(in srgb, var(--accent) 55%, var(--border)); }
  .trigger:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  .dir { flex: none; }
  .up { transform: rotate(180deg); }
  .arrow { flex: none; transition: transform .15s; }
  .arrow.up { transform: rotate(180deg); }

  .scrim { position: fixed; inset: 0; z-index: 60; }
  .panel {
    position: absolute; right: 0; top: calc(100% + 8px); z-index: 61; width: 372px; max-width: calc(100vw - 24px);
    padding: 12px; border-radius: 14px; background: var(--surface-2);
    border: 1px solid var(--border); box-shadow: 0 18px 50px rgba(0, 0, 0, .6); animation: pop .12s cubic-bezier(.3, .8, .2, 1);
  }
  @keyframes pop { from { opacity: 0; transform: translateY(-4px); } }
  @media (prefers-reduced-motion: reduce) { .panel { animation: none; } .arrow { transition: none; } }
  ol { list-style: none; margin: 0; padding: 0; display: grid; gap: 10px; }
  .by { display: block; margin-bottom: 4px; color: var(--faint); font-size: 10px; letter-spacing: .08em; text-transform: uppercase; }
  .row { display: flex; align-items: center; gap: 8px; }
  .key { flex: 1 1 0; min-width: 0; }
  .way {
    flex: none; display: inline-flex; align-items: center; gap: 6px; height: 30px; padding: 0 10px; border-radius: 8px;
    background: var(--surface); border: 1px solid var(--border); color: var(--text); font-size: 12px; white-space: nowrap;
  }
  .way:hover { border-color: var(--faint); }
  .tools { flex: none; display: inline-flex; gap: 2px; }
  .mini { display: grid; place-items: center; width: 24px; height: 28px; padding: 0; background: none; border: 0; border-radius: 6px; color: var(--muted); }
  .mini:hover:not(:disabled) { color: var(--text); background: var(--surface); }
  .mini:disabled { opacity: .3; cursor: default; }
  .foot { display: flex; align-items: center; justify-content: space-between; margin-top: 12px; padding-top: 10px; border-top: 1px solid var(--border); }
  .add, .reset { background: none; border: 0; padding: 4px 2px; font-size: 12px; }
  .add { color: var(--accent); }
  .add:hover:not(:disabled) { text-decoration: underline; text-underline-offset: 3px; }
  .reset { color: var(--muted); }
  .reset:hover:not(:disabled) { color: var(--text); }
  .add:disabled, .reset:disabled { opacity: .4; cursor: default; }
  .wrap :global(button:focus-visible) { outline: 2px solid var(--accent); outline-offset: 1px; }
</style>
