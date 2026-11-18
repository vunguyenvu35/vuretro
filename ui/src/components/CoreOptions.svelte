<script>
  import { api } from '../lib/api.js';
  import { toast } from '../lib/ui.svelte.js';
  import Select from './Select.svelte';

  /** `core` is the core file stem; `name` is shown in the title. */
  let { core, name, onclose = () => {} } = $props();

  let options = $state(null); // null while loading
  let error = $state('');
  let query = $state('');
  let saving = $state(false);

  $effect(() => {
    options = null;
    error = '';
    api
      .coreOptions(core)
      .then((o) => (options = o))
      .catch((e) => (error = String(e)));
  });

  const shown = $derived(
    (options ?? []).filter((o) => !query || `${o.key} ${o.description}`.toLowerCase().includes(query.toLowerCase())),
  );
  const changed = $derived((options ?? []).filter((o) => o.value !== o.default));

  async function save() {
    saving = true;
    try {
      await api.setCoreOptions(core, Object.fromEntries(changed.map((o) => [o.key, o.value])));
      toast('Core options saved. They apply the next time a game starts.', 'info');
      onclose();
    } catch (e) {
      toast(String(e), 'error');
    }
    saving = false;
  }

  function resetAll() {
    for (const o of options ?? []) o.value = o.default;
  }

  function onkeydown(e) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window {onkeydown} />

<div class="scrim" role="presentation" onmousedown={onclose}>
  <div class="dialog" role="dialog" aria-modal="true" aria-label="{name} options" tabindex="-1" onmousedown={(e) => e.stopPropagation()}>
    <header>
      <h2>{name} options</h2>
      <span class="mono muted">{core}</span>
    </header>

    {#if error}
      <p class="bad">{error}</p>
    {:else if options === null}
      <p class="muted">Loading the core…</p>
    {:else if options.length === 0}
      <p class="muted">This core has no options.</p>
    {:else}
      <input class="search" type="search" placeholder="Filter {options.length} options" bind:value={query} aria-label="Filter options" />
      <ul>
        {#each shown as o (o.key)}
          <li class:changed={o.value !== o.default}>
            <div class="what">
              <strong>{o.description || o.key}</strong>
              <span class="mono muted">{o.key}</span>
            </div>
            <Select compact bind:value={o.value} ariaLabel={o.description || o.key} minWidth={150}
              options={o.values.map((v) => ({ value: v, label: v, hint: v === o.default ? 'default' : '' }))} />
          </li>
        {:else}
          <li class="muted">No option matches.</li>
        {/each}
      </ul>
    {/if}

    <footer>
      <button class="btn" onclick={resetAll} disabled={!changed.length}>Reset all</button>
      <span class="spacer"></span>
      <button class="btn" onclick={onclose}>Cancel</button>
      <button class="btn primary" onclick={save} disabled={saving || options === null}>Save{changed.length ? ` (${changed.length} changed)` : ''}</button>
    </footer>
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; background: rgba(5, 7, 12, .6); display: grid; place-items: center; z-index: 50; }
  .dialog { width: min(640px, 92vw); max-height: 80vh; display: flex; flex-direction: column; gap: 12px; background: var(--surface); border: 1px solid var(--border); border-radius: 12px; padding: 20px; }
  header { display: flex; align-items: baseline; gap: 12px; }
  h2 { font-size: 18px; }
  .muted { color: var(--muted); }
  .bad { color: var(--bad); }
  .search { width: 100%; }
  ul { list-style: none; margin: 0; padding: 0; overflow: auto; display: flex; flex-direction: column; gap: 2px; }
  li { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 8px 10px; border-radius: 8px; }
  li.changed { background: color-mix(in srgb, var(--accent) 10%, transparent); }
  .what { display: flex; flex-direction: column; min-width: 0; }
  .what span { font-size: 11px; }
  footer { display: flex; gap: 8px; align-items: center; }
  .spacer { flex: 1; }
</style>
