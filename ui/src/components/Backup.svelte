<script>
  import { api } from '../lib/api.js';
  import { app } from '../lib/store.svelte.js';
  import { toast, confirmDialog } from '../lib/ui.svelte.js';
  import { backupFileName, formatBytes } from '../lib/layout.js';

  let saves = $state(true);
  let covers = $state(false);
  let busy = $state(false);

  async function run(job) {
    busy = true;
    try {
      await job();
    } catch (e) {
      toast(String(e), 'error');
    }
    busy = false;
  }

  const backUp = () =>
    run(async () => {
      const dest = await api.pickSavePath('Save a backup', backupFileName(), 'zip', 'Vuretro backup');
      if (!dest) return;
      const s = await api.createBackup(dest, saves, covers);
      toast(`Backup saved: ${s.files} files, ${formatBytes(s.bytes)}\n${s.path}`);
    });

  const restore = () =>
    run(async () => {
      const src = await api.pickBackupFile();
      if (!src) return;
      const m = await api.inspectBackup(src);
      const when = new Date(m.created * 1000).toLocaleString();
      const ok = await confirmDialog({
        title: 'Restore this backup?',
        message: [
          `• Made on ${when} by Vuretro ${m.app_version}.`,
          `• Replaces your current settings, favorites, collections, controls, cheats and${m.includes.includes('saves') ? ' saves' : ' (no saves in this backup)'} with the ones in the backup.`,
          '• Your games, BIOS files and cores are not touched.',
          '• A copy of what you have now is saved first, in the "backups" folder of the Vuretro data folder.',
        ].join('\n'),
        confirmLabel: 'Restore',
        danger: true,
      });
      if (!ok) return;
      const s = await api.restoreBackup(src);
      toast(`Restored ${s.files} files. Reloading…`);
      setTimeout(() => location.reload(), 900);
    });

  const exportCsv = () =>
    run(async () => {
      const dest = await api.pickSavePath('Export the library', 'vuretro-library.csv', 'csv', 'Spreadsheet (CSV)');
      if (!dest) return;
      const n = await api.exportLibrary(dest);
      toast(`Exported ${n} games to ${dest}`);
    });
</script>

<h2 class="group">Backup and export</h2>
<div class="panel">
  <div class="row">
    <div class="txt">
      <div class="t">Back up</div>
      <div class="d">
        Saves your settings, favorites, collections, controls, cheats and game names into one file. Your games and BIOS files are not
        included: they are large and already yours.
      </div>
      <div class="opts">
        <label><input type="checkbox" bind:checked={saves} /> Game saves and save states</label>
        <label><input type="checkbox" bind:checked={covers} /> Cover pictures <span class="muted">(large; previews can be made again)</span></label>
      </div>
    </div>
    <button class="btn primary" disabled={busy} onclick={backUp}>Back up…</button>
  </div>
  <div class="row">
    <div class="txt">
      <div class="t">Restore</div>
      <div class="d">Bring back a backup, for example on a new computer. A copy of your current state is kept first.</div>
    </div>
    <button class="btn" disabled={busy} onclick={restore}>Restore…</button>
  </div>
  <div class="row">
    <div class="txt">
      <div class="t">Export the game list</div>
      <div class="d">A spreadsheet (CSV) with every game, its system, play time, favorite and collections.{app.overview ? ` ${app.overview.games.length} games.` : ''}</div>
    </div>
    <button class="btn" disabled={busy} onclick={exportCsv}>Export…</button>
  </div>
</div>

<style>
  .group { font: 600 12px var(--font-body); text-transform: uppercase; letter-spacing: .1em; color: var(--muted); margin: 36px 4px 10px; }
  .panel { background: var(--surface); border: 1px solid var(--border); border-radius: 14px; overflow: hidden; }
  .row { display: flex; align-items: center; gap: 18px; padding: 16px 18px; border-bottom: 1px solid var(--border); }
  .row:last-child { border-bottom: 0; }
  .txt { flex: 1; min-width: 0; }
  .t { font-weight: 600; }
  .d { color: var(--muted); font-size: 12px; margin-top: 2px; max-width: 64ch; }
  .opts { display: flex; flex-direction: column; gap: 6px; margin-top: 10px; font-size: 13px; }
  .opts label { display: flex; align-items: center; gap: 8px; cursor: pointer; }
  .muted { color: var(--faint); }
</style>
