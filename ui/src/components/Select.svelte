<script>
  /**
   * A drop-down that looks like the rest of the app (the system one does not, and is
   * unusable with a long list). Options: [{ value, label, hint? }]. With `searchable`
   * the list can be filtered by typing, which is what you want for hundreds of games.
   */
  let {
    value = $bindable(),
    options = [],
    onchange = () => {},
    searchable = false,
    placeholder = 'Choose…',
    ariaLabel = '',
    minWidth = 0,
    compact = false,
  } = $props();

  let open = $state(false);
  let query = $state('');
  let active = $state(0);
  let btn = $state();
  let list = $state();
  let search = $state();
  let pos = $state({ left: 0, top: 0, bottom: undefined, width: 240, maxH: 320 });

  const current = $derived(options.find((o) => o.value === value));
  const shown = $derived(query.trim() ? options.filter((o) => o.label.toLowerCase().includes(query.trim().toLowerCase())) : options);

  function openList() {
    const r = btn.getBoundingClientRect();
    const below = window.innerHeight - r.bottom - 14;
    const above = r.top - 14;
    const up = below < 200 && above > below;
    const room = Math.max(160, up ? above : below);
    const width = Math.max(r.width, 240);
    pos = {
      left: Math.max(8, Math.min(r.left, window.innerWidth - width - 8)),
      top: up ? undefined : r.bottom + 6,
      bottom: up ? window.innerHeight - r.top + 6 : undefined,
      width,
      maxH: Math.min(360, room - (searchable ? 54 : 0)),
    };
    query = '';
    active = Math.max(0, options.findIndex((o) => o.value === value));
    open = true;
  }
  function close(refocus = true) {
    open = false;
    if (refocus) btn?.focus();
  }
  function choose(o) {
    value = o.value;
    close();
    onchange(o.value);
  }

  // Focus the search box (or the list) when it opens, and keep the active row in view.
  $effect(() => {
    if (!open) return;
    queueMicrotask(() => (search ?? list)?.focus({ preventScroll: true }));
  });
  $effect(() => {
    if (!open || !list) return;
    list.querySelector(`[data-i="${active}"]`)?.scrollIntoView({ block: 'nearest' });
  });
  $effect(() => {
    query; // a new filter starts from the top
    active = 0;
  });

  function onTriggerKey(e) {
    if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(e.key)) {
      e.preventDefault();
      openList();
    }
  }
  function onListKey(e) {
    const n = shown.length;
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      return close();
    }
    if (e.key === 'ArrowDown') (e.preventDefault(), (active = n ? (active + 1) % n : 0));
    else if (e.key === 'ArrowUp') (e.preventDefault(), (active = n ? (active - 1 + n) % n : 0));
    else if (e.key === 'Home') (e.preventDefault(), (active = 0));
    else if (e.key === 'End') (e.preventDefault(), (active = Math.max(0, n - 1)));
    else if (e.key === 'Enter') {
      e.preventDefault();
      if (shown[active]) choose(shown[active]);
    } else if (e.key === 'Tab') close(false);
  }
</script>

<svelte:window onresize={() => open && close(false)} />

<button
  type="button"
  class="trigger"
  class:open
  class:compact
  bind:this={btn}
  style={minWidth ? `min-width:${minWidth}px` : ''}
  aria-haspopup="listbox"
  aria-expanded={open}
  aria-label={ariaLabel}
  onclick={() => (open ? close() : openList())}
  onkeydown={onTriggerKey}
>
  <span class="val" class:ph={!current}>{current ? current.label : placeholder}</span>
  <svg class="chev" viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M4 6.2l4 4 4-4" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>
</button>

{#if open}
  <div class="scrim" role="presentation" onmousedown={() => close(false)}></div>
  <div class="panel" style="left:{pos.left}px; {pos.top !== undefined ? `top:${pos.top}px` : `bottom:${pos.bottom}px`}; width:{pos.width}px" onkeydown={onListKey} role="presentation">
    {#if searchable}
      <div class="sbox">
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><circle cx="7" cy="7" r="4.4" fill="none" stroke="currentColor" stroke-width="1.6" /><path d="M10.4 10.4L13.5 13.5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg>
        <input bind:this={search} bind:value={query} placeholder="Search…" spellcheck="false" autocomplete="off" aria-label="Search" />
        {#if query}<button type="button" class="x" onclick={() => { query = ''; search?.focus(); }} aria-label="Clear">✕</button>{/if}
      </div>
    {/if}
    <ul bind:this={list} tabindex="-1" role="listbox" style="max-height:{pos.maxH}px">
      {#each shown as o, i (o.value)}
        <li role="option" aria-selected={o.value === value} data-i={i} class:active={i === active} class:sel={o.value === value} onmousemove={() => (active = i)} onmousedown={(e) => e.preventDefault()} onclick={() => choose(o)}>
          <span class="chk" aria-hidden="true">{#if o.value === value}<svg viewBox="0 0 16 16" width="13" height="13"><path d="M3 8.5l3.2 3.2L13 4.8" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" /></svg>{/if}</span>
          <span class="lab">{o.label}</span>
          {#if o.hint}<span class="hint">{o.hint}</span>{/if}
        </li>
      {:else}
        <li class="none">No match</li>
      {/each}
    </ul>
    {#if searchable}<div class="count">{shown.length} of {options.length}</div>{/if}
  </div>
{/if}

<style>
  .trigger {
    display: inline-flex; align-items: center; justify-content: space-between; gap: 10px; width: 100%;
    background: var(--bg); border: 1px solid var(--border); border-radius: 10px; padding: 8px 10px 8px 12px;
    color: var(--text); font: inherit; font-size: 13px; text-align: left; transition: border-color .15s, box-shadow .15s;
  }
  .trigger.compact { padding: 6px 8px 6px 10px; width: auto; }
  .trigger:hover { border-color: var(--faint); }
  .trigger:focus-visible, .trigger.open { outline: none; border-color: var(--accent); box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 16%, transparent); }
  .val { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .val.ph { color: var(--muted); }
  .chev { flex: none; color: var(--muted); transition: transform .15s; }
  .trigger.open .chev { transform: rotate(180deg); color: var(--accent); }

  .scrim { position: fixed; inset: 0; z-index: 90; }
  .panel {
    position: fixed; z-index: 91; padding: 6px; border-radius: 14px;
    background: color-mix(in srgb, var(--surface-2) 94%, transparent); backdrop-filter: blur(14px);
    border: 1px solid var(--border); box-shadow: 0 18px 50px rgba(0, 0, 0, .6);
    animation: pop .12s cubic-bezier(.3, .8, .2, 1);
  }
  @keyframes pop { from { opacity: 0; transform: translateY(-4px) scale(.985); } }
  .sbox { display: flex; align-items: center; gap: 8px; padding: 0 10px; height: 36px; margin-bottom: 6px; background: var(--bg); border: 1px solid var(--border); border-radius: 9px; color: var(--faint); }
  .sbox:focus-within { border-color: var(--accent); color: var(--accent); }
  .sbox input { flex: 1; min-width: 0; background: none; border: 0; outline: none; padding: 0; color: var(--text); font: inherit; font-size: 13px; }
  .sbox input::placeholder { color: var(--muted); }
  .sbox .x { background: var(--surface-2); border: 0; color: var(--muted); width: 18px; height: 18px; border-radius: 50%; font-size: 9px; padding: 0; line-height: 1; }
  ul { list-style: none; margin: 0; padding: 0; overflow-y: auto; outline: none; overscroll-behavior: contain; }
  ul::-webkit-scrollbar { width: 8px; }
  ul::-webkit-scrollbar-thumb { background: var(--border); border-radius: 4px; }
  li { display: flex; align-items: center; gap: 8px; padding: 7px 10px 7px 6px; border-radius: 8px; font-size: 13px; color: var(--text); cursor: default; }
  li.active { background: color-mix(in srgb, var(--accent) 15%, transparent); }
  li.sel .lab { font-weight: 600; color: var(--accent); }
  .chk { width: 14px; flex: none; display: grid; place-items: center; color: var(--accent); }
  .lab { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hint { color: var(--faint); font-size: 11px; flex: none; }
  .none { color: var(--muted); justify-content: center; padding: 16px 0; }
  .count { padding: 7px 8px 2px; color: var(--faint); font-size: 11px; text-align: right; }
</style>
