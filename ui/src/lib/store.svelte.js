import { api } from './api.js';
import { summarizeBios, summarizeImport, summarizeRemoval } from './library.js';
import { toast } from './ui.svelte.js';

function readLayout() {
  try {
    return localStorage.getItem('vuretro.layout') === 'list' ? 'list' : 'grid';
  } catch {
    return 'grid';
  }
}

export const app = $state({
  view: 'library', // library | cores | controls | macros | settings
  overview: null,
  loaded: false,
  filter: 'all', // all | recent | favorites
  system: null, // system id or null
  collection: null, // collection name or null
  query: '',
  sort: 'recent', // recent | az | size
  layout: readLayout(), // grid | list
  selected: null, // game path
  busy: false,
  selecting: false, // choosing several games to remove at once
  picked: [], // paths of the chosen games
  capturing: false, // a key or button is being recorded on the Controls screen
});

const hash = typeof location !== "undefined" ? location.hash.slice(1).split(",") : [];
if (["cores", "controls", "macros", "settings"].includes(hash[0])) app.view = hash[0];
export const startSelect = hash.includes("select");

export function setLayout(layout) {
  app.layout = layout;
  try {
    localStorage.setItem('vuretro.layout', layout);
  } catch {
    /* remembering the choice is a convenience only */
  }
}

/** Preview images (data URLs) by game path. */
export const covers = $state({});

export async function loadCovers(paths) {
  if (!paths.length) return;
  try {
    Object.assign(covers, await api.covers(paths));
  } catch {
    /* previews are optional */
  }
}

let identifying = false;
/** Look games up in the DAT files in the background; names show up on the next refresh. */
async function identifyLater() {
  const ov = app.overview;
  if (identifying || !ov?.dat_entries || ov.games.every((g) => g.identified)) return;
  identifying = true;
  try {
    if ((await api.identifyGames()) > 0) {
      app.overview = await api.overview();
    }
    if (app.overview?.fetch_art) api.fetchArt().catch(() => {});
  } catch {
    /* names are optional */
  }
  identifying = false;
}

/** Add DAT files (No-Intro / Redump) so games get official names. */
export async function importDats(paths) {
  if (!paths?.length) return;
  const results = await api.importDats(paths);
  const bad = results.filter((r) => !r.ok);
  toast(
    [...results.filter((r) => r.ok).map((r) => `✓ ${r.message}`), ...bad.map((r) => `✕ ${r.source.split(/[\\/]/).pop()}: ${r.message}`)].join("\n"),
    bad.length ? 'error' : 'info',
  );
  await refresh();
}

/** Show the previews that exist and make the ones that are missing, in the background. */
export async function syncCovers() {
  const games = app.overview?.games ?? [];
  await loadCovers(games.filter((g) => !(g.path in covers)).map((g) => g.path));
  const todo = games.filter((g) => !covers[g.path] && !g.blocked);
  if (todo.length) api.generateCovers(todo.map((g) => ({ path: g.path, system: g.system }))).catch(() => {});
}

/** A game just closed: its last frame replaced the preview. */
export async function reloadLastPlayed() {
  const last = [...(app.overview?.games ?? [])].sort((a, b) => b.last_played - a.last_played)[0];
  if (last?.last_played) await loadCovers([last.path]);
}

export async function refresh() {
  try {
    app.overview = await api.overview();
    identifyLater();
    if (startSelect && !app.selected) app.selected = app.overview.games.find((g) => g.blocked)?.path ?? app.overview.games[0]?.path ?? null;
  } catch (e) {
    toast(String(e), 'error');
  }
  app.loaded = true;
  syncCovers();
}

function report(results) {
  const { text, failed } = summarizeImport(results);
  toast(text, failed ? 'error' : 'info');
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

/** Remove several imported games at once. Linked-folder games are never deleted. */
export async function removeMany(games) {
  if (!games.length) return;
  try {
    const results = await api.removeRoms(games.map((g) => g.path));
    const { text, failed } = summarizeRemoval(results);
    toast(text, failed ? 'error' : 'info');
  } catch (e) {
    toast(String(e), 'error');
  }
  app.picked = [];
  app.selecting = false;
  await refresh();
}

export async function toggleFavorite(game) {
  await api.toggleFavorite(game.path);
  refresh();
}

export async function play(game, slot = null) {
  if (game.blocked) return toast(game.blocked, 'error');
  try {
    await api.launchGame(game.path, slot);
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

/** Copy BIOS files into the app's system folder. */
export async function importBios(paths) {
  if (!paths?.length) return;
  const { text, failed } = summarizeBios(await api.importBios(paths));
  toast(text, failed ? 'error' : 'info');
  refresh();
}

/** Actions from the native menu bar (see build_menu in src-tauri). */
export async function handleMenu(id) {
  const library = (filter) => {
    app.view = 'library';
    app.filter = filter;
    app.system = null;
  };
  switch (id) {
    case 'import-roms': return pickRomFiles();
    case 'import-folder': return pickRomFolder();
    case 'import-bios': return importBios(await api.pickFiles('Choose BIOS files'));
    case 'link-folder': return linkFolder();
    case 'unlink-folder': return unlinkFolder();
    case 'run-demo': return runDemo();
    case 'view-library': return library('all');
    case 'view-recent': return library('recent');
    case 'view-favorites': return library('favorites');
    case 'view-cores': return (app.view = 'cores');
    case 'view-controls': return (app.view = 'controls');
    case 'view-settings': return (app.view = 'settings');
    case 'refresh': return refresh();
    case 'find':
      app.view = 'library';
      await new Promise((r) => setTimeout(r, 50));
      document.querySelector('input[aria-label="Search games"]')?.focus();
  }
}

/** Let the user pick a picture for a game's cover. */
export async function chooseCover(game) {
  const image = await api.pickImage();
  if (!image) return;
  try {
    await api.setCustomArt(game.path, image);
    await loadCovers([game.path]);
  } catch (e) {
    toast(String(e), 'error');
  }
}

/** Remove downloaded or chosen art; the screenshot preview (if any) comes back. */
export async function resetCover(game) {
  await api.clearArt(game.path);
  delete covers[game.path];
  await loadCovers([game.path]);
  syncCovers();
}
