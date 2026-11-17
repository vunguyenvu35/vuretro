<script>
  import { app, chooseRomFolder, runDemo, refresh } from '../lib/store.svelte.js';
  import logo from '../lib/logo-mark.svg';

  const ov = $derived(app.overview);
  const noFolder = $derived(!ov?.rom_dir);
</script>

<div class="wrap">
  <div class="panel">
    <img src={logo} alt="" width="72" height="72" />
    <h1>{noFolder ? 'Welcome to vuretro' : 'No games found yet'}</h1>
    <p class="lead">
      {#if noFolder}
        Play your retro games with one clean library. vuretro reads your ROM folder in place: nothing is copied or uploaded.
      {:else}
        vuretro looked in <span class="mono path">{ov.rom_dir}</span> but found no games it recognizes.
      {/if}
    </p>

    <ol class="steps">
      <li class:done={!noFolder}>
        <b>1</b>
        <div><strong>Pick your ROM folder</strong><span>One subfolder per system: <span class="mono">neogeo</span>, <span class="mono">psx</span>, <span class="mono">snes</span>, <span class="mono">gba</span>.</span></div>
      </li>
      <li>
        <b>2</b>
        <div><strong>Add BIOS files if a system needs one</strong><span>Neo Geo and PlayStation do. Drop them in Cores &amp; BIOS.</span></div>
      </li>
      <li>
        <b>3</b>
        <div><strong>Double-click a game to play</strong><span>Saves and save states are kept for you.</span></div>
      </li>
    </ol>

    <div class="row">
      <button class="btn primary" onclick={chooseRomFolder}>{noFolder ? 'Choose ROM folder…' : 'Choose another folder…'}</button>
      {#if !noFolder}<button class="btn" onclick={refresh}>Rescan</button>{/if}
      {#if ov?.demo_available}<button class="btn" onclick={runDemo}>Try the demo</button>{/if}
    </div>

    {#if ov && !ov.runner_found}
      <p class="warn">Game runner not found. Run <code>make build</code> once, then restart the app.</p>
    {/if}
  </div>
</div>

<style>
  .wrap { flex: 1; display: grid; place-items: center; padding: 32px; overflow: auto; }
  .panel {
    width: 100%; max-width: 560px; display: flex; flex-direction: column; align-items: center; text-align: center; gap: 14px;
  }
  img { filter: drop-shadow(0 8px 24px rgba(255, 79, 139, .25)); border-radius: 18px; }
  h1 { font-size: 30px; margin-top: 6px; }
  .lead { color: var(--muted); margin: 0; max-width: 46ch; font-size: 14px; }
  .path { color: var(--text); word-break: break-all; }
  .steps { list-style: none; padding: 0; margin: 14px 0 6px; width: 100%; display: grid; gap: 10px; text-align: left; }
  .steps li {
    display: grid; grid-template-columns: 32px 1fr; gap: 14px; align-items: center;
    background: var(--surface); border: 1px solid var(--border); border-radius: 12px; padding: 14px 16px;
  }
  .steps li.done { border-color: color-mix(in srgb, var(--ok) 50%, var(--border)); }
  .steps b {
    width: 32px; height: 32px; border-radius: 50%; display: grid; place-items: center;
    background: var(--surface-2); color: var(--accent); font: 700 14px var(--font-head);
  }
  .steps li.done b { background: var(--ok); color: #090b10; }
  .steps div { display: flex; flex-direction: column; gap: 2px; }
  .steps span { color: var(--muted); font-size: 12px; }
  .row { display: flex; gap: 10px; flex-wrap: wrap; justify-content: center; margin-top: 6px; }
  .warn { color: var(--bad); margin: 0; }
  code { font-family: var(--font-mono); background: var(--surface-2); padding: 1px 5px; border-radius: 4px; }
</style>
