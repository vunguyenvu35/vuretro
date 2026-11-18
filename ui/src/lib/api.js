// The only door from the UI to the backend: one function per Tauri command. In a plain
// browser (no Tauri) the calls are answered by `mock.js` so the UI can be developed.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
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
  installCore: (core) => call('install_core', { core }),
  toggleFavorite: (path) => call('toggle_favorite', { path }),
  importBios: (paths) => call('import_bios', { paths }),
  importRoms: (paths, system = null) => call('import_roms', { paths, system }),
  removeRom: (path) => call('remove_rom', { path }),
  removeRoms: (paths) => call('remove_roms', { paths }),
  reveal: (path) => call('reveal_path', { path }),

  // running games
  launchGame: (path) => call('launch_game', { path }),
  launchDemo: () => call('launch_demo'),

  // controls and macros
  controls: () => call('get_controls'),
  defaultControls: () => call('default_controls'),
  setControls: (config) => call('set_controls', { config }),
  macros: () => call('get_macros'),
  setMacros: (config) => call('set_macros', { config }),

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
  onPadsChanged: (fn) => on('pads-changed', fn),
  onCoreProgress: (fn) => on('core-progress', fn),
  onPadButton: (fn) => on('pad-button', fn),
  onCoversUpdated: (fn) => on('covers-updated', fn),
  onCoversDone: (fn) => on('covers-done', fn),
  onMenu: (fn) => on('menu', fn),
  onDragDrop: (fn) => (inTauri ? getCurrentWebview().onDragDropEvent((e) => fn(e.payload)) : Promise.resolve(() => {})),
};
