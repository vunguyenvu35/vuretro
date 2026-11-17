import { api } from './api.js';

export const app = $state({
  view: 'library', // library | cores | controls
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
if (["cores", "controls"].includes(hash[0])) app.view = hash[0];
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

export async function chooseRomFolder() {
  const dir = await api.pickFolder();
  if (!dir) return;
  await api.setRomDir(dir);
  app.busy = true;
  await refresh();
  app.busy = false;
  toast(`Scanned ${dir}`);
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
