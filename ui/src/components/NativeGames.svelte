<script>
  import { app, addNative, removeNative, playNative } from '../lib/store.svelte.js';
  import { api } from '../lib/api.js';

  let name = $state('');
  let exe = $state('');
  let assets = $state('');
  let adding = $state(false);

  async function chooseProgram() {
    const f = await api.pickProgram('Choose the game program (for example rr-game)');
    if (f) {
      exe = f;
      if (!name) name = 'Road Rash';
    }
  }
  async function chooseAssets() {
    const d = await api.pickFolder('Choose the folder with the game data (for example ROADRASH)');
    if (d) assets = d;
  }
  async function add() {
    adding = true;
    // the data folder goes to the program the way the Road Rash port reads it: --assets <folder>
    const args = assets ? ['--assets', assets] : [];
    const ok = await addNative(name, exe, args);
    adding = false;
    if (ok) name = exe = assets = '';
  }
  const ready = $derived(name.trim() !== '' && exe !== '' && !adding);
</script>

<section>
  <header>
    <h1>Native games</h1>
    <p>Games that are programs of their own, such as the native <b>Road Rash</b> port. Vuretro starts them as they are: no core and no ROM. They keep their own saves and controls.</p>
  </header>

  {#if app.native.length}
    <ul>
      {#each app.native as g (g.id)}
        <li>
          <div class="info">
            <div class="nm">{g.name}</div>
            <div class="path mono" title={g.exe}>{g.exe}</div>
            {#if g.args.length}<div class="path mono" title={g.args.join(' ')}>{g.args.join(' ')}</div>{/if}
          </div>
          <button class="btn primary" onclick={() => playNative(g)}>Play</button>
          <button class="btn" onclick={() => removeNative(g)}>Remove</button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="empty">No native game yet. Add one below.</p>
  {/if}

  <form onsubmit={(e) => { e.preventDefault(); if (ready) add(); }}>
    <h2>Add a game</h2>
    <label>Name <input bind:value={name} placeholder="Road Rash" /></label>
    <div class="row">
      <label class="grow">Program <input bind:value={exe} placeholder="/path/to/rr-game" /></label>
      <button type="button" class="btn" onclick={chooseProgram}>Choose…</button>
    </div>
    <div class="row">
      <label class="grow">Game data folder (optional) <input bind:value={assets} placeholder="/path/to/ROADRASH" /></label>
      <button type="button" class="btn" onclick={chooseAssets}>Choose…</button>
    </div>
    <p class="note">Vuretro does not include the game or its data: use the program you built and the disc you own.</p>
    <button class="btn primary" type="submit" disabled={!ready}>Add game</button>
  </form>
</section>

<style>
  section { flex: 1; min-width: 0; overflow-y: auto; padding: 28px 32px; display: flex; flex-direction: column; gap: 20px; max-width: 760px; }
  h1 { font-size: 22px; margin: 0 0 6px; }
  h2 { font-size: 15px; margin: 0 0 4px; }
  header p, .note, .empty { color: var(--muted); font-size: 13px; line-height: 1.5; margin: 0; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 10px; }
  li { display: flex; align-items: center; gap: 10px; padding: 12px 14px; background: var(--surface); border: 1px solid var(--border); border-radius: 12px; }
  .info { flex: 1; min-width: 0; }
  .nm { font-weight: 600; }
  .path { color: var(--faint); font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  form { display: grid; gap: 12px; padding: 16px; background: var(--surface); border: 1px solid var(--border); border-radius: 12px; }
  label { display: grid; gap: 4px; font-size: 12px; color: var(--muted); }
  input { padding: 8px 10px; border-radius: 8px; border: 1px solid var(--border); background: var(--surface-2); color: var(--text); font: inherit; }
  .row { display: flex; gap: 8px; align-items: end; }
  .grow { flex: 1; min-width: 0; }
  form > .btn { justify-self: start; }
</style>
