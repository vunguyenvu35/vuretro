// Per-system accent colors (docs/UI.md) and display helpers.
export const SYSTEM_COLORS = {
  neogeo: '#ffb347',
  psx: '#5ad1ff',
  snes: '#b79cff',
  gba: '#9be36b',
  nes: '#ff6b6b',
  gb: '#a3c46a',
  gbc: '#e08be0',
  genesis: '#4fb0ff',
  sms: '#ff8a3d',
  gg: '#6fd0c0',
  pce: '#ffd24d',
  atari2600: '#d98c4a',
  atari7800: '#c9a15c',
  lynx: '#e0b04a',
  ngp: '#7fb3ff',
  wonderswan: '#8fd0ff',
  virtualboy: '#ff5f5f',
  sega32x: '#5f8dff',
  nds: '#c2c8d6',
  arcade: '#ff9f43',
};
export const colorOf = (id) => SYSTEM_COLORS[id] ?? '#8b93a7';

export function formatPlaytime(secs) {
  if (!secs) return '—';
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (h) return `${h}h ${m}m`;
  return m ? `${m}m` : '<1m';
}

export function formatAgo(ts) {
  if (!ts) return 'never';
  const s = Math.max(0, Date.now() / 1000 - ts);
  if (s < 90) return 'just now';
  if (s < 3600) return `${Math.floor(s / 60)} min ago`;
  if (s < 86400) return `${Math.floor(s / 3600)} h ago`;
  return `${Math.floor(s / 86400)} d ago`;
}

export const isMac = typeof navigator !== 'undefined' && /Mac/i.test(navigator.platform);
export const modKey = isMac ? '⌘' : 'Ctrl';
