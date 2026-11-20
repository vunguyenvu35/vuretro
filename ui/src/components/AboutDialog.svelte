<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { REPO, KOFI } from '../lib/links.js';
  import { toast } from '../lib/ui.svelte.js';
  import logo from '../lib/logo-mark.svg';
  import Modal from './Modal.svelte';
  import { update, checkForUpdate, installUpdate, setAutoCheck } from '../lib/update.svelte.js';

  let { onclose = () => {} } = $props();

  let version = $state('');
  onMount(() => {
    api.appVersion().then((v) => (version = v)).catch(() => {});
  });

  async function open(url) {
    try {
      await api.openUrl(url);
    } catch (e) {
      toast(String(e), 'error');
    }
  }
</script>

<Modal title="About Vuretro" {onclose}>
  <img src={logo} alt="" width="64" height="64" />
  <div class="name"><b>Vu</b><span class="re">retro</span></div>
  <div>Version <b data-testid="about-version">{version || '…'}</b></div>
  <div class="upd" aria-live="polite">
    {#if update.status === 'available'}
      <span>Version <b>{update.version}</b> is available.</span>
      <button class="go" onclick={installUpdate}>Update and restart</button>
    {:else if update.status === 'installing'}
      <span>Downloading {update.version}{update.percent == null ? '…' : ` · ${update.percent}%`}</span>
    {:else if update.status === 'checking'}
      <span class="muted">Checking for updates…</span>
    {:else}
      <span class="muted">{update.status === 'current' ? 'You are on the latest version.' : update.status === 'error' ? `Could not check: ${update.error}` : ''}</span>
      <button class="link" onclick={() => checkForUpdate()}>Check for updates</button>
    {/if}
  </div>
  {#if update.status === 'available' && update.notes}
    <pre class="notes">{update.notes}</pre>
  {/if}
  <label class="muted auto"><input type="checkbox" checked={update.auto} onchange={(e) => setAutoCheck(e.currentTarget.checked)} /> Check for updates when Vuretro starts</label>
  <p class="lead">Play classic console and arcade games from one library.</p>
  <p class="muted">Runs libretro cores on macOS, Windows and Linux. Free and open source under the MIT license.</p>
  <p class="muted">Your games, saves and settings stay on your computer. Vuretro does not include games or BIOS files.</p>
  <p class="muted">Made by Vu Nguyen Vu.</p>
  <p class="muted links">
    <button class="link" onclick={() => open(REPO)}>GitHub</button> ·
    <button class="link" onclick={() => open(REPO + '/issues/new')}>Report a bug</button> ·
    <button class="link" onclick={() => open(REPO + '/blob/main/LICENSE')}>License</button> ·
    <button class="link" onclick={() => open(KOFI)}>Ko-fi</button>
  </p>
</Modal>

<style>
  /* the same lockup as the sidebar: a bold magenta "Vu", a lighter "retro" */
  .name { font: 700 26px/1 var(--font-head); letter-spacing: -.025em; }
  .name b { color: var(--brand-magenta); }
  .name .re { color: #f4f1ff; font-weight: 500; }
  .lead { margin: 0; color: var(--text); font-size: 14px; }
  .muted { color: var(--muted); margin: 0; }
  .link { background: none; border: 0; padding: 0; color: var(--accent); text-decoration: underline; font: inherit; }
  .upd { display: flex; align-items: center; justify-content: center; gap: 10px; flex-wrap: wrap; min-height: 28px; }
  .go { background: var(--accent); color: #1a1205; border: 0; border-radius: 8px; padding: 6px 12px; font-weight: 600; }
  .notes { max-height: 120px; overflow: auto; margin: 0; padding: 8px 10px; text-align: left; white-space: pre-wrap; font: 12px/1.45 var(--font-body, inherit); color: var(--muted); background: var(--surface); border-radius: 8px; }
  .auto { display: flex; align-items: center; justify-content: center; gap: 6px; font-size: 12px; }
</style>
