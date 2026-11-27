// The only door from the UI to the backend: one function per Tauri command. In a plain
// browser (no Tauri) the calls are answered by `mock.js` so the UI can be developed.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open, save } from '@tauri-apps/plugin-dialog';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { mock, DEFAULT_GAME } from './mock.js';

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

const call = (cmd, args = {}) => (inTauri ? invoke(cmd, args) : mock[cmd](args));
/** Subscribe to a backend event; outside Tauri there are none. */
const on = (event, fn) => (inTauri ? listen(event, (e) => fn(e.payload)) : Promise.resolve(() => {}));

export const api = {
  // library
  overview: () => call('get_overview'),
  setRomDir: (path) => call('set_rom_dir', { path }),
  setSystemEnabled: (id, enabled) => call('set_system_enabled', { id, enabled }),
  setSystemCore: (id, core) => call('set_system_core', { id, core }),
  importDats: (paths) => call('import_dats', { paths }),
  identifyGames: () => call('identify_games'),
  setTags: (path, tags) => call('set_tags', { path, tags }),
  renameGame: (path, name) => call('rename_game', { path, name }),
  clearRecent: () => call('clear_recent'),
  setGameCore: (path, core) => call('set_game_core', { path, core }),
  resetMameInputs: (path) => call('reset_mame_inputs', { path }),
  setFetchArt: (enabled) => call('set_fetch_art', { enabled }),
  cheats: (path) => call('get_cheats', { path }),
  setCheats: (path, text) => call('set_cheats', { path, text }),
  fetchArt: () => call('fetch_art'),
  setCustomArt: (path, image) => call('set_custom_art', { path, image }),
  setScreenscraper: (user, password) => call('set_screenscraper', { user, password }),
  clearArt: (path) => call('clear_art', { path }),
  regeneratePreviews: () => call('regenerate_previews'),
  openUrl: (url) => call('open_url', { url }),
  /** the newer version ({version, current, notes}), or null when this is the latest */
  checkUpdate: () => call('check_update'),
  installUpdate: () => call('install_update'),
  async appVersion() {
    if (!inTauri) return '0.0.0-dev';
    const { getVersion } = await import('@tauri-apps/api/app');
    return getVersion();
  },
  async pickImage() {
    if (!inTauri) return null;
    const r = await open({ multiple: false, title: 'Choose a cover image', filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg'] }] });
    return r || null;
  },
  // backup and export
  createBackup: (dest, saves, covers) => call('create_backup', { dest, saves, covers }),
  inspectBackup: (src) => call('inspect_backup', { src }),
  restoreBackup: (src) => call('restore_backup', { src }),
  exportLibrary: (dest) => call('export_library', { dest }),
  /** Ask where to save a file; `null` when the user cancels (or outside the app). */
  async pickSavePath(title, defaultPath, extension, label) {
    if (!inTauri) return null;
    return (await save({ title, defaultPath, filters: [{ name: label, extensions: [extension] }] })) || null;
  },
  async pickBackupFile() {
    if (!inTauri) return null;
    return (await open({ multiple: false, title: 'Choose a Vuretro backup', filters: [{ name: 'Vuretro backup', extensions: ['zip'] }] })) || null;
  },
  installCore: (core) => call('install_core', { core }),
  coreOptions: (core) => call('get_core_options', { core }),
  setCoreOptions: (core, overrides) => call('set_core_options', { core, overrides }),
  toggleFavorite: (path) => call('toggle_favorite', { path }),
  importBios: (paths) => call('import_bios', { paths }),
  importRoms: (paths, system = null) => call('import_roms', { paths, system }),
  applyPatch: (rom, patch) => call('apply_patch', { rom, patch }),
  removeRom: (path) => call('remove_rom', { path }),
  removeRoms: (paths) => call('remove_roms', { paths }),
  reveal: (path) => call('reveal_path', { path }),

  // running games
  launchGame: (path, slot = null) => call('launch_game', { path, slot }),
  stopGame: () => call('stop_game'),
  runPcInstaller: (path) => call('run_pc_installer', { path }),
  saveSlots: (path) => call('get_save_slots', { path }),
  slotThumbnail: (path, slot) => call('get_slot_thumbnail', { path, slot }),
  launchDemo: () => call('launch_demo'),
  // native games: programs of their own, started as they are (no core, no ROM)
  nativeGames: () => call('list_native_games'),
  foundNativeGames: () => call('found_native_games'),
  programsInFolder: (dir) => call('programs_in_folder', { dir }),
  addNativeGame: (name, exe, args = [], cover = null) => call('add_native_game', { name, exe, args, cover }),
  // setting up Road Rash from the user's own disc: the tools it needs, and the build
  nativeToolsMissing: () => call('native_tools_missing'),
  installNativeTools: () => call('install_native_tools'),
  installRoadRash: (disc) => call('install_road_rash', { disc }),
  nativeCovers: () => call('native_covers'),
  onNativeInstallLog: (fn) => on('native-install-log', fn),
  async pickDisc() {
    if (!inTauri) return null;
    return (await open({ multiple: false, title: 'Choose your Road Rash PC disc image', filters: [{ name: 'Disc image', extensions: ['iso', 'img', 'bin'] }] })) || null;
  },
  removeNativeGame: (id) => call('remove_native_game', { id }),
  launchNative: (id) => call('launch_native', { id }),
  async pickProgram(title = 'Choose the game program') {
    if (!inTauri) return null;
    return (await open({ multiple: false, title })) || null;
  },

  // controls and macros
  controls: () => call('get_controls'),
  /** Give a pad a name of the user's own (empty = the system's name); returns the pads as shown. */
  renamePad: (id, name) => call('rename_pad', { id, name }),
  defaultControls: () => call('default_controls'),
  setControls: (config) => call('set_controls', { config }),
  controlsFor: (system) => call('get_controls_for', { system: system || null }),
  setControlsFor: (system, config) => call('set_controls_for', { system, config }),
  resetControlsFor: (system) => call('reset_controls_for', { system }),
  scopedSettings: (system) => call('get_scoped_settings', { system: system || null }),
  setSystemOverride: (system, overrides) => call('set_system_override', { system, overrides }),
  gameOverride: (path) => call('get_game_override', { path }),
  controlsForGame: (path) => call('get_controls_for_game', { path }),
  setControlsForGame: (path, config) => call('set_controls_for_game', { path, config }),
  resetControlsForGame: (path) => call('reset_controls_for_game', { path }),
  setGameOverride: (path, overrides) => call('set_game_override', { path, overrides }),
  macros: () => call('get_macros'),
  setMacros: (config) => call('set_macros', { config }),
  setMacroProfile: (game, profile) => call('set_macro_profile', { game, profile }),
  compileMove: (notation, mirror = false) => call('compile_move', { notation, mirror }),

  // game settings
  gameSettings: () => call('get_game_settings'),
  setGameSettings: (settings) => call('set_game_settings', { settings }),
  defaultGameSettings: async () => ({ ...DEFAULT_GAME }),

  // preview images
  covers: (paths) => call('get_covers', { paths }),
  generateCovers: (games) => call('generate_covers', { games }),

  // file dialogs
  async pickFolder(title = 'Choose a folder') {
    if (!inTauri) return '~/Games/roms';
    return open({ directory: true, multiple: false, title });
  },
  async pickFiles(title = 'Choose BIOS files') {
    if (!inTauri) return [];
    const r = await open({ multiple: true, title });
    return r ? [].concat(r) : [];
  },

  // events (each returns a promise of an unsubscribe function)
  onGameExited: (fn) => on('game-exited', fn),
  onPcPreparing: (fn) => on('pc-preparing', fn),
  onPcPrepared: (fn) => on('pc-prepared', fn),
  // what the running game reports: {event: 'state', action: 'saved'|'loaded', slot},
  // {event: 'screenshot', path}, {event: 'progress', played_secs}, ...
  onGameEvent: (fn) => on('game-event', fn),
  onUpdateProgress: (fn) => on('update-progress', fn),
  onPadsChanged: (fn) => on('pads-changed', fn),
  /** a 2.4G receiver is plugged in but its pad is asleep (true) or awake / gone (false) */
  onDongleWaiting: (fn) => on('dongle-waiting', fn),
  onCoreProgress: (fn) => on('core-progress', fn),
  onCoreDownloading: (fn) => on('core-downloading', fn),
  onPadButton: (fn) => on('pad-button', fn),
  onCoversUpdated: (fn) => on('covers-updated', fn),
  onCoversDone: (fn) => on('covers-done', fn),
  onMenu: (fn) => on('menu', fn),
  onDragDrop: (fn) => (inTauri ? getCurrentWebview().onDragDropEvent((e) => fn(e.payload)) : Promise.resolve(() => {})),
};
