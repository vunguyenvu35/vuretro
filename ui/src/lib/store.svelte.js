import { api } from './api.js';
import { missingCovers, nextSorts, summarizeBios, summarizeImport, summarizeRemoval } from './library.js';
import { toast, confirmDialog } from './ui.svelte.js';
import { clampSidebar, parseSidebar } from './layout.js';

function readLayout() {
  try {
    return localStorage.getItem('vuretro.layout') === 'list' ? 'list' : 'grid';
  } catch {
    return 'grid';
  }
}

function readSidebar() {
  try {
    return parseSidebar(localStorage.getItem('vuretro.sidebar'));
  } catch {
    return parseSidebar(null);
  }
}

function readFlag(key) {
  try {
    return localStorage.getItem(key) === '1';
  } catch {
    return false;
  }
}

function readSections() {
  try {
    const v = JSON.parse(localStorage.getItem('vuretro.sections') ?? '{}');
    return v && typeof v === 'object' ? v : {};
  } catch {
    return {};
  }
}

export const app = $state({
  sidebarW: readSidebar(), // px, dragged by the user
  view: 'library', // library | native | cores | controls | macros | settings
  overview: null,
  loaded: false,
  filter: 'all', // all | recent | favorites
  system: null, // system id or null
  collection: null, // collection name or null
  query: '',
  sorts: [{ key: 'recent', reverse: false }], // how the games are ordered: keys recent | az | system | size | played, the first the main one
  status: '', // '' | never | played | ready | blocked | duplicates: a filter next to the sidebar's
  layout: readLayout(), // grid | list
  selected: null, // game path
  renaming: null, // path of the game whose name is being edited in the detail panel
  busy: false,
  selecting: false, // choosing several games to remove at once
  picked: [], // paths of the chosen games
  capturing: false, // a key or button is being recorded on the Controls screen
  dialog: null, // 'about' | 'support': a small dialog over the page
  scope: '', // system id the Settings and Controls screens edit; empty = all games
  downloading: null, // { core, percent } while Play fetches the core a game needs
  nativeFound: [], // installed native games not yet in the library: { id, name, exe, args }
  nativeCovers: {}, // pictures of the native games by id (data: URLs)
  nativeTools: [], // the tools Road Rash's build needs and this computer lacks
  preparing: null, // what a Windows game is doing before it starts the first time
  playing: null,
  playingNote: '', // what runs it, when it is not a core of the app ("wine") // the title of the game that is open, so it can be stopped from the app
  nativeInstall: { running: false, step: '', log: [] }, // setting Road Rash up from the disc
  native: [], // games that are programs of their own: { id, name, exe, args }
  sidebarFolded: readFlag('vuretro.sidebarFolded'), // the sidebar is a rail of icons
  foldMoving: false,
  folded: readSections(), // sidebar groups the user closed: { systems: true }
  installing: {}, // core name -> percent while it downloads (here or from the Cores screen)
  coreUpdates: [], // installed cores that have a newer build on the server
  checkingUpdates: false,
});

const hash = typeof location !== "undefined" ? location.hash.slice(1).split(",") : [];
if (["cores", "controls", "macros", "settings", "native"].includes(hash[0])) app.view = hash[0];
export const startSelect = hash.includes("select");
if (["about", "support"].includes(hash[1])) app.dialog = hash[1]; // for development: #settings,support

/** A click on the heading of a sort: see `nextSorts`. With `add` (Shift) it joins the sorts already there. */
export function setSort(key, { add = false } = {}) {
  app.sorts = nextSorts(app.sorts, key, { add });
}

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

/*
 * Previews are read and made only for the games on screen (plus the one in the hero and the
 * details panel), so a library of hundreds does not start hundreds of background runs. A game
 * that already has a picture (downloaded, chosen, or made before) just shows it; the others
 * are made one at a time, and scrolling elsewhere makes the backend drop what was waiting.
 */
const onScreen = new Set(); // paths of the cards that are in view or about to be
let pinned = []; // the hero and the details panel
let coverTimer;

/** A card scrolled into view (or out of it). */
export function setOnScreen(path, yes) {
  if (!path) return;
  if (yes) onScreen.add(path);
  else onScreen.delete(path);
  scheduleCovers();
}

/** Games that must have a picture whether or not their card is in view. */
export function pinCovers(paths) {
  pinned = paths.filter(Boolean);
  scheduleCovers();
}

/** Look at what is on screen soon, not on every scroll event. */
export function scheduleCovers() {
  clearTimeout(coverTimer);
  coverTimer = setTimeout(requestCovers, 200);
}

async function requestCovers() {
  const byPath = new Map((app.overview?.games ?? []).map((g) => [g.path, g]));
  const games = [...new Set([...pinned, ...onScreen])].map((p) => byPath.get(p)).filter(Boolean);
  await loadCovers(games.filter((g) => !(g.path in covers)).map((g) => g.path));
  // an empty list is a request too: it drops what was still waiting
  api.generateCovers(missingCovers(games, covers).map((g) => ({ path: g.path, system: g.system }))).catch(() => {});
}

const coreName = (core) => core.replace(/_libretro$/, '');

/** Download or update one core, showing its progress in `app.installing`. `true` when it was installed. */
export async function installCore(core, { quiet = false } = {}) {
  if (core in app.installing) return false;
  app.installing[core] = 0;
  try {
    await api.installCore(core);
    app.coreUpdates = app.coreUpdates.filter((c) => c !== core);
    if (!quiet) toast(`Installed ${coreName(core)}`, 'info');
    return true;
  } catch (e) {
    // a download started elsewhere (Play, an earlier visit) is still running: not an error
    if (String(e).includes('already being')) return false;
    toast(`${coreName(core)}: ${e}`, 'error');
    return false;
  } finally {
    delete app.installing[core];
    refresh();
  }
}

/** Download the cores of the systems that are on and missing, one after the other. */
export async function installMissingCores({ withRuntime = true, includeOff = true } = {}) {
  let list = [];
  try {
    list = await api.missingCores(includeOff);
  } catch {
    return;
  }
  if (!withRuntime) list = list.filter((c) => c !== 'wine'); // Wine is a big download of its own
  let done = 0;
  for (const core of list) if (await installCore(core, { quiet: true })) done++;
  if (done) toast(`Installed ${done} core${done === 1 ? '' : 's'}`, 'info');
}

/** Ask the server which installed cores have a newer build. `quiet` keeps errors and "all up to date" silent. */
export async function checkCoreUpdates({ quiet = false } = {}) {
  if (app.checkingUpdates) return;
  app.checkingUpdates = true;
  try {
    app.coreUpdates = await api.coreUpdates();
    if (!quiet) toast(app.coreUpdates.length ? `${app.coreUpdates.length} core update${app.coreUpdates.length === 1 ? '' : 's'} available` : 'All cores are up to date', 'info');
  } catch (e) {
    if (!quiet) toast(String(e), 'error');
  } finally {
    app.checkingUpdates = false;
  }
}

/** Update the cores that have a newer build, one after the other. */
export async function updateCores() {
  for (const core of [...app.coreUpdates]) await installCore(core, { quiet: true });
  toast(app.coreUpdates.length ? 'Some cores could not be updated' : 'Cores updated', app.coreUpdates.length ? 'error' : 'info');
}

let startedChores = false;
/** Once, after the library first shows: fetch missing cores if the user asked, and look for updates. */
function startCoreChores() {
  if (startedChores || !app.overview) return;
  startedChores = true;
  setTimeout(async () => {
    if (app.overview?.auto_install_cores) await installMissingCores({ withRuntime: false, includeOff: false });
    checkCoreUpdates({ quiet: true });
  }, 3000);
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
  startCoreChores();
  loadNative();
  scheduleCovers();
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

/** Make a patched copy of a game from an IPS, UPS or BPS patch the user picks. */
export async function applyPatch(game) {
  const files = await api.pickFiles('Choose a patch (.ips, .ups or .bps)');
  const patch = files?.[0];
  if (!patch) return;
  app.busy = true;
  try {
    report([await api.applyPatch(game.path, patch)]);
  } catch (e) {
    toast(String(e), 'error');
  }
  await refresh();
  app.busy = false;
}

export async function toggleFavorite(game) {
  await api.toggleFavorite(game.path);
  refresh();
}

/**
 * Give a game a name of its own in the library; an empty name or `null` gives its original
 * one back. Throws when the name is refused (too long), so the dialog can say so.
 */
export async function renameGame(game, name) {
  await api.renameGame(game.path, name);
  await refresh();
}

/** Empty the Recent list. The games, their play time and saves stay. */
export async function clearRecent() {
  try {
    await api.clearRecent();
    toast('Recent list cleared');
  } catch (e) {
    toast(String(e), 'error');
  }
  await refresh();
}

export async function play(game, slot = null) {
  if (game.blocked) return toast(game.blocked, 'error');
  try {
    await api.launchGame(game.path, slot);
    app.playing = game.title;
    // a Windows game runs in Wine, which the Dock shows as "wine"
    app.playingNote = game.system === 'windows' ? 'wine' : '';
    toast(`Started ${game.title}`);
  } catch (e) {
    toast(String(e), 'error');
  } finally {
    app.downloading = null;
  }
  refresh();
}

/** Run the installer of the disc of a Windows game; Play then starts what it installed. */
export async function runInstaller(game) {
  try {
    toast('The installer opens in its own window: follow it, then close it');
    const program = await api.runPcInstaller(game.path);
    toast(`Installed ${program}: press Play to start it`);
  } catch (e) {
    toast(String(e), 'error');
  }
  refresh();
}

export async function loadNative() {
  try {
    app.native = await api.nativeGames();
    app.nativeFound = await api.foundNativeGames();
    app.nativeCovers = await api.nativeCovers();
    app.nativeTools = await api.nativeToolsMissing().catch(() => []);
  } catch (e) {
    toast(String(e), 'error');
  }
}

/** Add a program to the library as a game of its own. Resolves to true when it was added. */
export async function addNative(name, exe, args, cover = null) {
  try {
    const game = await api.addNativeGame(name, exe, args, cover);
    toast(`Added ${game.name}`);
    await loadNative();
    return true;
  } catch (e) {
    toast(String(e), 'error');
    return false;
  }
}

/** Take in a line of what the installer prints. */
export function installLine(line) {
  const log = app.nativeInstall.log;
  log.push(String(line));
  if (log.length > 200) log.splice(0, log.length - 200);
  // the steps are announced as "1/4 ..."
  if (/^\d\/\d /.test(line)) app.nativeInstall.step = String(line);
}

async function runInstall(job, done) {
  if (app.nativeInstall.running) return;
  app.nativeInstall = { running: true, step: '', log: [] };
  try {
    await job();
    toast(done);
  } catch (e) {
    toast(String(e), 'error');
    installLine(`Failed: ${e}`);
  } finally {
    app.nativeInstall.running = false;
  }
  await loadNative();
}

/** Install the tools the build needs (Homebrew on a Mac). */
export const installTools = () => runInstall(() => api.installNativeTools(), 'The tools are installed');

/** Build Road Rash from its source and read the game's data from the user's disc. */
export const installRoadRash = (disc) => runInstall(() => api.installRoadRash(disc), 'Road Rash is ready: press Play');

export async function removeNative(game) {
  const yes = await confirmDialog({
    title: `Remove ${game.name}?`,
    message: 'It leaves the library. The program and its files stay where they are.',
    confirmLabel: 'Remove',
    danger: true,
  });
  if (!yes) return;
  await api.removeNativeGame(game.id);
  await loadNative();
}

export async function playNative(game) {
  try {
    await api.launchNative(game.id);
    toast(`Started ${game.name}`);
  } catch (e) {
    toast(String(e), 'error');
  }
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
    case 'help-about': return (app.dialog = 'about');
    case 'help-support': return (app.dialog = 'support');
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
  scheduleCovers();
}

/** Set the sidebar width (px); `save` also remembers it for the next start. */
export function setSidebarWidth(width, save = false) {
  app.sidebarW = clampSidebar(width, typeof window === 'undefined' ? Infinity : window.innerWidth);
  if (!save) return;
  try {
    localStorage.setItem('vuretro.sidebar', String(app.sidebarW));
  } catch {
    /* a width that is not remembered is fine */
  }
}

/** Throw away every preview picture and make them again (chosen or downloaded art stays). */
export async function regenerateCovers() {
  const ok = await confirmDialog({
    title: 'Make all previews again?',
    message: 'Every preview picture is deleted and made again in the background. Cover art you chose or downloaded is kept.',
    confirmLabel: 'Make again',
  });
  if (!ok) return;
  const n = await api.regeneratePreviews();
  for (const k of Object.keys(covers)) delete covers[k];
  scheduleCovers();
  toast(`${n} previews are being made again`);
}

/** Fold the sidebar to a rail of icons, or open it again; remembered for the next start. */
export function toggleSidebar() {
  app.sidebarFolded = !app.sidebarFolded;
  app.foldMoving = true; // the width eases while it folds, and only then (dragging must not lag)
  setTimeout(() => (app.foldMoving = false), 260);
  try {
    localStorage.setItem('vuretro.sidebarFolded', app.sidebarFolded ? '1' : '0');
  } catch {
    /* not remembered: fine */
  }
}

/** Close or open one group of the sidebar (Library, Systems...); remembered. */
export function toggleSection(name) {
  app.folded[name] = !app.folded[name];
  try {
    localStorage.setItem('vuretro.sections', JSON.stringify(app.folded));
  } catch {
    /* not remembered: fine */
  }
}
