<script>
  import { app, addNative, removeNative, playNative } from '../lib/store.svelte.js';
  import { toast } from '../lib/ui.svelte.js';
  import { api } from '../lib/api.js';

  const manifestExample = '{"name": "My game", "exe": "play", "args": ["--data", "{dir}/data"]}';
  const addFound = (g) => addNative(g.name, g.exe, g.args);
  const fileName = (path) => path.split('/').pop().split('\\').pop();

  // Programs the folder holds when it is not clear which one is the game
  let choices = $state([]);

  /** The user picks the game's folder; Vuretro finds the program in it. */
  async function addFromFolder() {
    const dir = await api.pickFolder("Choose the game's folder");
    if (!dir) return;
    try {
      const found = await api.programsInFolder(dir);
      if (found.length === 1) {
        choices = [];
        await addFound(found[0]);
      } else {
        choices = found;
      }
    } catch (e) {
      toast(String(e), 'error');
    }
  }
  async function choose(g) {
    if (await addFound(g)) choices = [];
  }
</script>

<section>
  <header>
    <h1>Native games</h1>
    <p>Games that are programs of their own, not run by a core: native ports, indie games, anything you can start from a file. Vuretro starts them as they are. They keep their own saves and controls.</p>
  </header>

  {#if app.nativeFound.length}
    <div class="found">
      {#each app.nativeFound as g (g.exe)}
        <div class="info">
          <div class="nm">{g.name} was found</div>
          <div class="path">Installed in Vuretro's data folder. Add it to play.</div>
        </div>
        <button class="btn primary" onclick={() => addFound(g)}>Add to library</button>
      {/each}
    </div>
  {/if}

  {#if app.native.length}
    <ul>
      {#each app.native as g (g.id)}
        <li>
          <div class="info">
            <div class="nm">{g.name}</div>
            <div class="path mono" title={g.exe}>{g.exe}</div>
          </div>
          <button class="btn primary" onclick={() => playNative(g)}>Play</button>
          <button class="btn" onclick={() => removeNative(g)}>Remove</button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="empty">No native game yet.</p>
  {/if}

  <div class="add">
    <button class="btn primary" onclick={addFromFolder}>Add a game from a folder…</button>
    <p class="note">Choose the folder of the game. Vuretro finds the program in it.</p>
  </div>

  {#if choices.length}
    <div class="pick">
      <h2>Which one starts the game?</h2>
      {#each choices as g (g.exe)}
        <div class="row">
          <span class="path mono" title={g.exe}>{fileName(g.exe)}</span>
          <button class="btn" onclick={() => choose(g)}>This one</button>
        </div>
      {/each}
    </div>
  {/if}

  <details class="howto">
    <summary>Install a game so it appears here by itself</summary>
    <p>Put the game in its own folder under Vuretro's <span class="mono">native</span> data folder, with a <span class="mono">game.json</span> that says how to start it:</p>
    <pre class="mono">{manifestExample}</pre>
    <p><span class="mono">exe</span> is the program inside the folder, and <span class="mono">{'{dir}'}</span> stands for the folder. The game then shows up above, ready to add. Vuretro does not include any game.</p>
    <p>For the native <b>Road Rash</b> port there is a script that does all this from its author's source: <span class="mono">scripts/build-road-rash.sh [your ROADRASH.iso]</span>.</p>
  </details>
</section>

<style>
  section { flex: 1; min-width: 0; overflow-y: auto; padding: 28px 32px; display: flex; flex-direction: column; gap: 20px; max-width: 760px; }
  h1 { font-size: 22px; margin: 0 0 6px; }
  h2 { font-size: 15px; margin: 0 0 8px; }
  header p, .note, .empty { color: var(--muted); font-size: 13px; line-height: 1.5; margin: 0; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 10px; }
  li { display: flex; align-items: center; gap: 10px; padding: 12px 14px; background: var(--surface); border: 1px solid var(--border); border-radius: 12px; }
  .info { flex: 1; min-width: 0; }
  .nm { font-weight: 600; }
  .path { color: var(--faint); font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .add { display: flex; align-items: center; gap: 14px; }
  .pick { display: grid; gap: 8px; padding: 14px; background: var(--surface); border: 1px solid var(--border); border-radius: 12px; }
  .row { display: flex; align-items: center; gap: 10px; }
  .row .path { flex: 1; min-width: 0; font-size: 12px; color: var(--text); }
  .found { display: flex; align-items: center; gap: 12px; padding: 14px; border-radius: 12px; border: 1px solid color-mix(in srgb, var(--accent) 50%, var(--border)); background: color-mix(in srgb, var(--accent) 8%, transparent); }
  details { background: var(--surface); border: 1px solid var(--border); border-radius: 12px; padding: 12px 16px; }
  summary { cursor: pointer; font-weight: 600; font-size: 13px; }
  .howto p { color: var(--muted); font-size: 13px; line-height: 1.5; }
  pre { margin: 8px 0; padding: 10px 12px; background: var(--surface-2); border-radius: 8px; font-size: 12px; overflow-x: auto; user-select: all; }
</style>
