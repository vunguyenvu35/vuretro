<script>
  import { api } from '../lib/api.js';
  import { REPO, KOFI } from '../lib/links.js';
  import { toast } from '../lib/ui.svelte.js';
  import Modal from './Modal.svelte';

  let { onclose = () => {} } = $props();

  const AMOUNTS = [
    ['☕', '$3', 'a cup of coffee'],
    ['🍜', '$10', 'a lunch'],
    ['🚀', '$25', 'fuel new features'],
  ];

  async function open(url) {
    try {
      await api.openUrl(url);
    } catch (e) {
      toast(String(e), 'error');
    }
  }
</script>

<Modal title="Support Vuretro" wide {onclose}>
  <p class="muted">Vuretro is free and open source, built and maintained in spare time. If it saves you time, you can help keep it going.</p>
  <div class="amounts">
    {#each AMOUNTS as [icon, amount, what] (amount)}
      <div class="amount">
        <div class="icon" aria-hidden="true">{icon}</div>
        <b>{amount}</b>
        <div class="muted">{what}</div>
      </div>
    {/each}
  </div>
  <button class="btn primary" onclick={() => open(KOFI)}>Support on Ko-fi</button>
  <p class="muted small">
    Other ways to help: <button class="link" onclick={() => open(REPO + '/issues/new')}>report a bug</button>, or star the project on
    <button class="link" onclick={() => open(REPO)}>GitHub</button>.
  </p>
</Modal>

<style>
  .muted { color: var(--muted); margin: 0; }
  .small { font-size: 12px; }
  .amounts { display: flex; gap: 12px; flex-wrap: wrap; justify-content: center; }
  .amount { min-width: 140px; padding: 12px 16px; border-radius: 12px; border: 1px solid var(--border); background: var(--bg); }
  .icon { font-size: 26px; }
  .link { background: none; border: 0; padding: 0; color: var(--accent); text-decoration: underline; font: inherit; }
</style>
