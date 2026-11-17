import { api } from './api.js';

export const app = $state({
  view: 'library', // library | cores | controls | settings
  overview: null,
  loaded: false,
  filter: 'all', // all | recent | favorites
  system: null, // system id or null
  query: '',
  sort: 'recent', // recent | az | size
  selected: null, // game path
  toast: null, // { text, kind }
  busy: false,
});

const hash = typeof location !== "undefined" ? location.hash.slice(1).split(",") : [];
if (["cores", "controls", "settings"].includes(hash[0])) app.view = hash[0];
export const startSelect = hash.includes("select");

let toastTimer;
export function toast(text, kind = 'info') {
  app.toast = { text, kind };
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (app.toast = null), kind === 'error' ? 7000 : 3500);
}

export async function refresh() {
  try {
    app.overview = await api.overview();
    if (startSelect && !app.selected) app.selected = app.overview.games.find((g) => g.blocked)?.path ?? app.overview.games[0]?.path ?? null;
  } catch (e) {
    toast(String(e), 'error');
  }
  app.loaded = true;
}

function report(results) {
  const ok = results.filter((r) => r.ok);
  const bad = results.filter((r) => !r.ok);
  const name = (r) => r.source.split(/[\\/]/).pop();
  const added = ok.filter((r) => r.message.startsWith('added')).length;
  const lines = [];
  if (added) lines.push(`✓ ${added} game${added === 1 ? '' : 's'} added`);
  const same = ok.length - added;
  if (same) lines.push(`${same} already in the library`);
  lines.push(...bad.slice(0, 4).map((r) => `✕ ${name(r)}: ${r.message}`));
  if (bad.length > 4) lines.push(`… and ${bad.length - 4} more`);
  toast(lines.join('\n') || 'Nothing to import', bad.length ? 'error' : 'info');
}

/** Copy ROM files (or every ROM in the given folders) into the app's library. */
export async function importRoms(paths) {
  if (!paths?.length) return;
  app.busy = true;
  try {
    report(await api.importRoms(paths, app.system));
  } catch (e) {
    toast(String(e), 'error');
  }
  await refresh();
  app.busy = false;
}
export async function pickRomFiles() {
  await importRoms(await api.pickFiles('Choose ROM files to import'));
}
export async function pickRomFolder() {
  const dir = await api.pickFolder('Choose a folder to import ROMs from');
  if (dir) await importRoms([dir]);
}

/** Optional: also read ROMs in place from a folder (nothing is copied). */
export async function linkFolder() {
  const dir = await api.pickFolder('Choose a folder to read ROMs from');
  if (!dir) return;
  await api.setRomDir(dir);
  app.busy = true;
  await refresh();
  app.busy = false;
  toast(`Reading ROMs from ${dir}`);
}
export async function unlinkFolder() {
  await api.setRomDir(null);
  await refresh();
  toast('Folder unlinked. Imported games stay in the library.');
}
export async function removeRom(game) {
  try {
    await api.removeRom(game.path);
    toast(`Removed ${game.title} from the library`);
  } catch (e) {
    toast(String(e), 'error');
  }
  await refresh();
}

export async function play(game) {
  if (game.blocked) return toast(game.blocked, 'error');
  try {
    await api.launchGame(game.path);
    toast(`Started ${game.title}`);
  } catch (e) {
    toast(String(e), 'error');
  }
  refresh();
}

export async function runDemo() {
  try {
    await api.launchDemo();
    toast('Demo started: arrow keys move the square, Esc quits');
  } catch (e) {
    toast(String(e), 'error');
  }
}
