// Thin wrapper over the Tauri commands. In a plain browser (npm run dev without
// Tauri) it serves mock data so the UI can be developed and screenshotted.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { getCurrentWebview } from '@tauri-apps/api/webview';

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

const mockBios = (system, file, required, note, state = 'missing') => ({ system, file, required, note, state });
const emptyMock = typeof location !== "undefined" && location.hash.includes("empty");
const mockOverview = () => (emptyMock ? { ...fullMock(), rom_dir: null, games: [], systems: fullMock().systems.map((s) => ({ ...s, game_count: 0 })) } : fullMock());
const fullMock = () => ({
  data_dir: '~/Library/Application Support/dev.vuvn.vuretro',
  system_dir: '~/Library/Application Support/dev.vuvn.vuretro/system',
  rom_dir: '~/Games/roms',
  runner_found: true,
  demo_available: true,
  systems: [
    { id: 'neogeo', name: 'Neo Geo', core: 'fbneo_libretro', extensions: ['zip', '7z'], enabled: true, core_installed: true, bios_ready: true, game_count: 2, bios: [mockBios('neogeo', 'neogeo.zip', true, 'MVS / AES BIOS set', 'ok')] },
    { id: 'psx', name: 'PlayStation', core: 'pcsx_rearmed_libretro', extensions: ['chd', 'cue', 'pbp'], enabled: true, core_installed: true, bios_ready: false, game_count: 1, bios: [mockBios('psx', 'scph5501.bin', true, 'NTSC-U'), mockBios('psx', 'scph5500.bin', false, 'NTSC-J'), mockBios('psx', 'scph5502.bin', false, 'PAL')] },
    { id: 'snes', name: 'Super Nintendo', core: 'snes9x_libretro', extensions: ['sfc', 'smc'], enabled: true, core_installed: true, bios_ready: true, game_count: 1, bios: [] },
    { id: 'gba', name: 'Game Boy Advance', core: 'mgba_libretro', extensions: ['gba'], enabled: true, core_installed: true, bios_ready: true, game_count: 1, bios: [mockBios('gba', 'gba_bios.bin', false, 'optional')] },
  ],
  games: [
    { system: 'neogeo', title: 'Sample Shooter', path: '/roms/neogeo/shooter.zip', size: 31_000_000, last_played: Date.now() / 1000 - 7200, play_secs: 11520, favorite: true, blocked: null },
    { system: 'psx', title: 'Sample Racer', path: '/roms/psx/racer.chd', size: 400_000_000, last_played: 0, play_secs: 0, favorite: false, blocked: 'BIOS missing' },
    { system: 'snes', title: 'Sample Adventure', path: '/roms/snes/adventure.sfc', size: 1_048_576, last_played: Date.now() / 1000 - 90000, play_secs: 3600, favorite: false, blocked: null },
    { system: 'gba', title: 'Sample Platformer', path: '/roms/gba/platformer.gba', size: 4_194_304, last_played: 0, play_secs: 0, favorite: false, blocked: null },
    { system: 'neogeo', title: 'Sample Fighter', path: '/roms/neogeo/fighter.zip', size: 28_000_000, last_played: 0, play_secs: 0, favorite: false, blocked: null },
  ],
});

const mock = {
  get_overview: async () => mockOverview(),
  set_rom_dir: async () => {},
  set_system_enabled: async () => {},
  toggle_favorite: async () => true,
  import_bios: async ({ paths }) => paths.map((p) => ({ source: p, ok: false, message: 'Not available in browser preview' })),
  launch_game: async () => {},
  launch_demo: async () => {},
};

const call = (cmd, args = {}) => (inTauri ? invoke(cmd, args) : mock[cmd](args));

export const api = {
  overview: () => call('get_overview'),
  setRomDir: (path) => call('set_rom_dir', { path }),
  setSystemEnabled: (id, enabled) => call('set_system_enabled', { id, enabled }),
  toggleFavorite: (path) => call('toggle_favorite', { path }),
  importBios: (paths) => call('import_bios', { paths }),
  launchGame: (path) => call('launch_game', { path }),
  launchDemo: () => call('launch_demo'),
  async pickFolder() {
    if (!inTauri) return '~/Games/roms';
    return open({ directory: true, multiple: false, title: 'Choose your ROM folder' });
  },
  async pickFiles() {
    if (!inTauri) return [];
    const r = await open({ multiple: true, title: 'Choose BIOS files' });
    return r ? [].concat(r) : [];
  },
  onGameExited: (fn) => (inTauri ? listen('game-exited', fn) : Promise.resolve(() => {})),
  onDragDrop: (fn) => (inTauri ? getCurrentWebview().onDragDropEvent((e) => fn(e.payload)) : Promise.resolve(() => {})),
};

// ---- controls ----
const mockControls = () => ({
  config: {
    players: [
      { device: { kind: 'auto' }, deadzone: 0.35, stick_as_dpad: true,
        keyboard: { up: ['Up'], down: ['Down'], left: ['Left'], right: ['Right'], b: ['Z'], a: ['X'], y: ['A'], x: ['S'], l: ['Q'], r: ['E'], l2: ['W'], r2: ['R'], select: ['RightShift', 'Key5'], start: ['Enter', 'Key1'] },
        gamepad: { up: ['DPadUp'], down: ['DPadDown'], left: ['DPadLeft'], right: ['DPadRight'], b: ['South'], a: ['East'], y: ['West'], x: ['North'], l: ['LeftTrigger'], r: ['RightTrigger'], l2: ['LeftTrigger2'], r2: ['RightTrigger2'], l3: ['LeftThumb'], r3: ['RightThumb'], select: ['Select'], start: ['Start'] } },
      { device: { kind: 'auto' }, deadzone: 0.35, stick_as_dpad: true,
        keyboard: { up: ['I'], down: ['K'], left: ['J'], right: ['L'], b: ['N'], a: ['M'], y: ['H'], x: ['G'], select: ['Key6'], start: ['Key2'] },
        gamepad: { up: ['DPadUp'], down: ['DPadDown'], left: ['DPadLeft'], right: ['DPadRight'], b: ['South'], a: ['East'], y: ['West'], x: ['North'], l: ['LeftTrigger'], r: ['RightTrigger'], l2: ['LeftTrigger2'], r2: ['RightTrigger2'], l3: ['LeftThumb'], r3: ['RightThumb'], select: ['Select'], start: ['Start'] } },
    ],
  },
  pads: [{ id: 'mock-xbox', name: 'Xbox Wireless Controller' }, { id: 'mock-dual', name: 'DualSense Wireless Controller' }],
});
mock.get_controls = async () => mockControls();
mock.default_controls = async () => mockControls().config;
mock.set_controls = async ({ config }) => config;

api.controls = () => call('get_controls');
api.defaultControls = () => call('default_controls');
api.setControls = (config) => call('set_controls', { config });
api.onPadsChanged = (fn) => (inTauri ? listen('pads-changed', (e) => fn(e.payload)) : Promise.resolve(() => {}));
api.onPadButton = (fn) => (inTauri ? listen('pad-button', (e) => fn(e.payload)) : Promise.resolve(() => {}));
