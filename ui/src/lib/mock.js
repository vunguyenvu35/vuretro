// Mock backend for developing the UI in a plain browser (`npm run dev` without Tauri).
// `api.js` falls back to these handlers, which mirror the Tauri commands one to one.

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
    { id: 'neogeo', coin_op: true, name: 'Neo Geo', core: 'fbneo_libretro', core_choices: [{ name: 'fbneo_libretro', installed: true }], extensions: ['zip', '7z'], enabled: true, core_installed: true, bios_ready: true, game_count: 2, bios: [mockBios('neogeo', 'neogeo.zip', true, 'MVS / AES BIOS set', 'ok')] },
    { id: 'psx', name: 'PlayStation', core: 'pcsx_rearmed_libretro', core_choices: [{ name: 'pcsx_rearmed_libretro', installed: true }], extensions: ['chd', 'cue', 'pbp'], enabled: true, core_installed: true, bios_ready: false, game_count: 1, bios: [mockBios('psx', 'scph5501.bin', true, 'NTSC-U'), mockBios('psx', 'scph5500.bin', false, 'NTSC-J'), mockBios('psx', 'scph5502.bin', false, 'PAL')] },
    { id: 'snes', name: 'Super Nintendo', core: 'snes9x_libretro', core_choices: [{ name: 'snes9x_libretro', installed: true }], extensions: ['sfc', 'smc'], enabled: true, core_installed: true, bios_ready: true, game_count: 1, bios: [] },
    { id: 'gba', name: 'Game Boy Advance', core: 'mgba_libretro', core_choices: [{ name: 'mgba_libretro', installed: true }], extensions: ['gba'], enabled: true, core_installed: true, bios_ready: true, game_count: 1, bios: [mockBios('gba', 'gba_bios.bin', false, 'optional')] },
    { id: 'n64', name: 'Nintendo 64', core: 'mupen64plus_next_libretro', core_choices: [{ name: 'mupen64plus_next_libretro', installed: false }], extensions: ['n64', 'z64'], enabled: false, core_installed: false, bios_ready: true, game_count: 0, bios: [], hardware: true, renderer_ready: false, assessment: { level: 'marginal', reasons: ['4 GB of memory is just enough; 6 GB or more is comfortable', 'the graphics chip cannot be checked: an old or weak one can still make games lag'] } },
  ],
  machine: { cpu_name: 'Mock CPU', cpu_cores: 8, ram_gb: 4, os: 'Mock OS' },
  games: [
    { system: 'neogeo', title: 'Sample Shooter', path: '/roms/neogeo/shooter.zip', size: 31_000_000, last_played: Date.now() / 1000 - 7200, play_secs: 11520, favorite: true, blocked: null, imported: true },
    { system: 'psx', title: 'Sample Racer', path: '/roms/psx/racer.chd', size: 400_000_000, last_played: 0, play_secs: 0, favorite: false, blocked: 'BIOS missing' },
    { system: 'snes', title: 'Sample Adventure', path: '/roms/snes/adventure.sfc', size: 1_048_576, last_played: Date.now() / 1000 - 90000, play_secs: 3600, favorite: false, blocked: null, imported: true },
    { system: 'gba', title: 'Sample Platformer', path: '/roms/gba/platformer.gba', size: 4_194_304, last_played: 0, play_secs: 0, favorite: false, blocked: null },
    { system: 'neogeo', title: 'Sample Fighter', path: '/roms/neogeo/fighter.zip', size: 28_000_000, last_played: 0, play_secs: 0, favorite: false, blocked: null, imported: true },
  ],
});

const mock = {
  get_overview: async () => mockOverview(),
  set_rom_dir: async () => {},
  set_system_enabled: async () => {},
  set_system_core: async () => {},
  install_core: async () => {},
  get_save_slots: async () => [],
  import_dats: async (a) => a.paths.map((p) => ({ source: p, ok: true, message: "3 dumps added" })),
  identify_games: async () => 0,
  set_tags: async (a) => a.tags,
  rename_game: async (a) => (a.name?.trim() ? a.name.trim() : null),
  clear_recent: async () => 0,
  set_game_core: async () => {},
  set_fetch_art: async () => {},
  fetch_art: async () => {},
  set_custom_art: async () => {},
  clear_art: async () => {},
  open_url: async () => {},
  check_update: async () => null,
  install_update: async () => {},
  regenerate_previews: async () => 0,
  set_screenscraper: async () => {},
  get_cheats: async () => "",
  set_cheats: async () => {},
  get_slot_thumbnail: async () => null,
  get_core_options: async () => [
    { key: "region", description: "Region", values: ["auto", "pal", "ntsc"], default: "auto", value: "auto" },
    { key: "overscan", description: "Show overscan", values: ["off", "on"], default: "off", value: "off" },
  ],
  set_core_options: async () => {},
  toggle_favorite: async () => true,
  import_bios: async ({ paths }) => paths.map((p) => ({ source: p, ok: false, message: 'Not available in browser preview' })),
  launch_game: async () => {},
  launch_demo: async () => {},
};

// ---- controls ----
const mockControls = () => ({
  config: {
    players: [
      { device: { kind: 'auto' }, deadzone: 0.35, stick_as_dpad: true,
        keyboard: { up: ['Up'], down: ['Down'], left: ['Left'], right: ['Right'], down_right: ['M'], up_left: ['I'], b: ['Z'], a: ['X'], y: ['A'], x: ['S'], l: ['Q'], r: ['E'], l2: ['W'], r2: ['R'], select: ['RightShift', 'Key5'], start: ['Enter', 'Key1'] },
        gamepad: { up: ['DPadUp'], down: ['DPadDown'], left: ['DPadLeft'], right: ['DPadRight'], b: ['South'], a: ['East'], y: ['West'], x: ['North'], l: ['LeftTrigger'], r: ['RightTrigger'], l2: ['LeftTrigger2'], r2: ['RightTrigger2'], l3: ['LeftThumb'], r3: ['RightThumb'], select: ['Select'], start: ['Start'] } },
      { device: { kind: 'auto' }, deadzone: 0.35, stick_as_dpad: true,
        keyboard: { up: ['I'], down: ['K'], left: ['J'], right: ['L'], b: ['N'], a: ['M'], y: ['H'], x: ['G'], select: ['Key6'], start: ['Key2'] },
        gamepad: { up: ['DPadUp'], down: ['DPadDown'], left: ['DPadLeft'], right: ['DPadRight'], b: ['South'], a: ['East'], y: ['West'], x: ['North'], l: ['LeftTrigger'], r: ['RightTrigger'], l2: ['LeftTrigger2'], r2: ['RightTrigger2'], l3: ['LeftThumb'], r3: ['RightThumb'], select: ['Select'], start: ['Start'] } },
      { device: { kind: 'auto' }, deadzone: 0.35, stick_as_dpad: true,
        keyboard: { up: ['NumPad8'], down: ['NumPad5'], left: ['NumPad4'], right: ['NumPad6'], b: ['NumPad1'], a: ['NumPad2'], y: ['NumPad0'], x: ['NumPad3'], select: ['NumPad7'], start: ['NumPad9'] },
        gamepad: { up: ['DPadUp'], down: ['DPadDown'], left: ['DPadLeft'], right: ['DPadRight'], b: ['South'], a: ['East'], y: ['West'], x: ['North'], l: ['LeftTrigger'], r: ['RightTrigger'], l2: ['LeftTrigger2'], r2: ['RightTrigger2'], l3: ['LeftThumb'], r3: ['RightThumb'], select: ['Select'], start: ['Start'] } },
      { device: { kind: 'auto' }, deadzone: 0.35, stick_as_dpad: true, keyboard: {},
        gamepad: { up: ['DPadUp'], down: ['DPadDown'], left: ['DPadLeft'], right: ['DPadRight'], b: ['South'], a: ['East'], y: ['West'], x: ['North'], l: ['LeftTrigger'], r: ['RightTrigger'], l2: ['LeftTrigger2'], r2: ['RightTrigger2'], l3: ['LeftThumb'], r3: ['RightThumb'], select: ['Select'], start: ['Start'] } },
    ],
  },
  pads: [{ id: 'mock-xbox', name: 'Xbox Wireless Controller' }, { id: 'mock-dual', name: 'DualSense Wireless Controller' }],
});
mock.get_controls = async () => mockControls();
mock.default_controls = async () => mockControls().config;
mock.set_controls = async ({ config }) => config;

// ---- game settings ----
const DEFAULT_GAME = {
  window_scale: 2, sound: true, volume: 100, sync_display: true, pause_on_blur: false, crt: false, rewind: false,
  fast_forward: 4, ff_sound: false, resume: false, lcd: false, scale2x: false, integer_scale: false, fullscreen: false, rumble: true, auto_coin: false,
};
let mockGame = { ...DEFAULT_GAME };
mock.import_roms = async () => [];
mock.apply_patch = async ({ rom }) => ({ source: rom, ok: true, message: 'added to gba as Sample [patch].gba' });
mock.remove_rom = async () => {};
mock.remove_roms = async ({ paths }) => paths.map((p) => ({ source: p, ok: true, message: 'removed' }));
mock.get_game_settings = async () => ({ ...mockGame });
mock.set_game_settings = async ({ settings }) => ((mockGame = { ...settings }), { ...mockGame });

// ---- preview images, macros, reveal ----
mock.get_covers = async () => ({});
mock.generate_covers = async () => {};
let mockMacros = { global: [], systems: {}, games: {}, profiles: {}, active: {} };
mock.get_macros = async () => mockMacros;
mock.set_macro_profile = async ({ game, profile }) => {
  const key = game.split(/[\\/]/).pop().replace(/\.[^.]*$/, '').toLowerCase();
  if (profile) mockMacros.active[key] = profile;
  else delete mockMacros.active[key];
  return mockMacros;
};
mock.compile_move = async ({ notation }) => {
  const text = (notation ?? '').trim();
  if (!text) throw 'type a move first, for example 236P';
  const bad = text.match(/DM|special/i);
  if (bad) throw `"${bad[0]}" is not a move I can play (use directions like 236 and the buttons A B C D)`;
  const steps = [{ actions: ['down'], ms: 50 }, { actions: ['down', 'right'], ms: 50 }, { actions: ['right', 'b'], ms: 50 }];
  return { steps, total_ms: 150 };
};
mock.set_macros = async ({ config }) => ((mockMacros = config), config);
mock.reveal_path = async () => {};

// per-system overrides and key bindings, kept in memory for browser development
const mockOverrides = {};
const mockSystemControls = {};
const scoped = (system) => {
  const ov = mockOverrides[system] ?? {};
  const eff = { ...mockGame };
  for (const k of ['window_scale', 'crt', 'rewind']) if (ov[k] != null) eff[k] = ov[k];
  return { effective: eff, inherited: { ...mockGame }, overridden: { window_scale: null, crt: null, rewind: null, ...ov }, recommended: { integer_scale: true, crt: false, lcd: true } };
};
mock.get_scoped_settings = async ({ system }) => (system ? scoped(system) : { effective: { ...mockGame }, inherited: { ...mockGame }, overridden: { window_scale: null, crt: null, rewind: null } });
mock.set_system_override = async ({ system, overrides }) => {
  const clean = Object.fromEntries(Object.entries(overrides).filter(([, v]) => v != null));
  if (Object.keys(clean).length) mockOverrides[system] = clean;
  else delete mockOverrides[system];
  return scoped(system);
};
const mockGameOv = {};
mock.get_game_override = async ({ path }) => {
  const ov = mockGameOv[path] ?? {};
  const eff = { ...mockGame };
  for (const k of ['window_scale', 'crt', 'rewind']) if (ov[k] != null) eff[k] = ov[k];
  return { effective: eff, inherited: { ...mockGame }, overridden: { window_scale: null, crt: null, rewind: null, ...ov } };
};
mock.set_game_override = async ({ path, overrides }) => {
  const clean = Object.fromEntries(Object.entries(overrides).filter(([, v]) => v != null));
  if (Object.keys(clean).length) mockGameOv[path] = clean;
  else delete mockGameOv[path];
  return mock.get_game_override({ path });
};
const mockGameControls = {};
mock.get_controls_for_game = async ({ path }) => ({ config: mockGameControls[path] ?? (await mock.get_controls()).config, custom: !!mockGameControls[path], pads: [] });
mock.set_controls_for_game = async ({ path, config }) => ((mockGameControls[path] = config), mock.get_controls_for_game({ path }));
mock.reset_controls_for_game = async ({ path }) => (delete mockGameControls[path], mock.get_controls_for_game({ path }));
const controlsView = async (system) => ({ config: mockSystemControls[system] ?? (await mock.get_controls()).config, custom: !!mockSystemControls[system], pads: [] });
mock.get_controls_for = async ({ system }) => (system ? controlsView(system) : { ...(await mock.get_controls()), custom: false });
mock.set_controls_for = async ({ system, config }) => ((mockSystemControls[system] = config), controlsView(system));
mock.reset_controls_for = async ({ system }) => (delete mockSystemControls[system], controlsView(system));

mock.create_backup = async ({ dest }) => ({ path: dest, files: 12, bytes: 480000 });
mock.inspect_backup = async () => ({ format: 1, app_version: '0.1.3', created: Math.floor(Date.now() / 1000) - 86400, includes: ['settings.json', 'library.json', 'config', 'dats', 'saves'] });
mock.restore_backup = async ({ src }) => ({ path: src, files: 12, bytes: 480000 });
mock.export_library = async () => 5;

export { mock, DEFAULT_GAME };
