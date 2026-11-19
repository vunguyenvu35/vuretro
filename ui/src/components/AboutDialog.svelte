<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { REPO, KOFI } from '../lib/links.js';
  import { toast } from '../lib/ui.svelte.js';
  import logo from '../lib/logo-mark.svg';
  import Modal from './Modal.svelte';

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
  <div class="name"><b>Vu</b>retro</div>
  <div>Version <b data-testid="about-version">{version || '…'}</b></div>
  <p class="muted">A retro game frontend. Free and open source (MIT). Your games, saves and settings stay on this computer; Vuretro does not include games or BIOS files.</p>
  <p class="muted">Made by Vu Nguyen Vu.</p>
  <p class="muted links">
    <button class="link" onclick={() => open(REPO)}>GitHub</button> ·
    <button class="link" onclick={() => open(REPO + '/issues/new')}>Report a bug</button> ·
    <button class="link" onclick={() => open(REPO + '/blob/main/LICENSE')}>License</button> ·
    <button class="link" onclick={() => open(KOFI)}>Ko-fi</button>
  </p>
</Modal>

<style>
  .name { font: 700 22px var(--font-head); }
  .name b { color: var(--brand-magenta); }
  .muted { color: var(--muted); margin: 0; }
  .link { background: none; border: 0; padding: 0; color: var(--accent); text-decoration: underline; font: inherit; }
</style>
