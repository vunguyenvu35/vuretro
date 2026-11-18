// Names shared with the runner (crates/controls): RetroPad actions, minifb key names,
// gilrs button names. Plus labels and the browser KeyboardEvent.code -> key name map.

export const ACTIONS = [
  { id: 'up', label: 'Up', hint: 'D-pad up' },
  { id: 'down', label: 'Down', hint: 'D-pad down' },
  { id: 'left', label: 'Left', hint: 'D-pad left' },
  { id: 'right', label: 'Right', hint: 'D-pad right' },
  { id: 'b', label: 'B', hint: 'Neo Geo A · SNES B · GBA B' },
  { id: 'a', label: 'A', hint: 'Neo Geo B · SNES A · GBA A' },
  { id: 'y', label: 'Y', hint: 'Neo Geo C · SNES Y' },
  { id: 'x', label: 'X', hint: 'Neo Geo D · SNES X' },
  { id: 'l', label: 'L', hint: 'Shoulder left · PS L1' },
  { id: 'r', label: 'R', hint: 'Shoulder right · PS R1' },
  { id: 'l2', label: 'L2', hint: 'Trigger left' },
  { id: 'r2', label: 'R2', hint: 'Trigger right' },
  { id: 'l3', label: 'L3', hint: 'Stick click left' },
  { id: 'r3', label: 'R3', hint: 'Stick click right' },
  { id: 'select', label: 'Select', hint: 'Neo Geo Coin' },
  { id: 'start', label: 'Start', hint: 'Start' },
];

const CODE_TO_KEY = {
  ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right',
  Enter: 'Enter', Space: 'Space', Backspace: 'Backspace',
  ShiftLeft: 'LeftShift', ShiftRight: 'RightShift', ControlLeft: 'LeftCtrl', ControlRight: 'RightCtrl',
  AltLeft: 'LeftAlt', AltRight: 'RightAlt', Comma: 'Comma', Period: 'Period', Slash: 'Slash',
  Semicolon: 'Semicolon', Quote: 'Apostrophe', BracketLeft: 'LeftBracket', BracketRight: 'RightBracket',
  Minus: 'Minus', Equal: 'Equal', Backslash: 'Backslash', Insert: 'Insert', Delete: 'Delete',
  Home: 'Home', End: 'End', PageUp: 'PageUp', PageDown: 'PageDown',
};
import { modKey } from './systems.js';

/** Keyboard shortcuts of the app window. */
export const APP_SHORTCUTS = [
  { keys: [modKey, 'F'], what: 'Find a game', also: [modKey, 'K'] },
  { keys: [modKey, 'O'], what: 'Import games' },
  { keys: [modKey, '⇧', 'O'], what: 'Import a folder' },
  { keys: [modKey, 'B'], what: 'Import BIOS files' },
  { keys: [modKey, 'R'], what: 'Refresh the library' },
  { keys: [modKey, '1'], what: 'Library' },
  { keys: [modKey, '2'], what: 'Recently played' },
  { keys: [modKey, '3'], what: 'Favorites' },
  { keys: [modKey, '4'], what: 'Cores & BIOS' },
  { keys: [modKey, '5'], what: 'Controls' },
  { keys: [modKey, ','], what: 'Settings' },
  { keys: ['←', '↑', '↓', '→'], what: 'Move between games' },
  { keys: ['Enter'], what: 'Play the selected game' },
  { keys: ['F'], what: 'Add or remove favorite' },
  { keys: ['Esc'], what: 'Clear the search' },
];

/** Keys the game window uses itself (shown on the Controls and Settings screens). */
export const HOTKEYS = [
  { keys: ['F10'], also: ['F9'], what: 'Bigger / smaller window', note: 'Remembered for next time' },
  { keys: ['Alt', 'Enter'], what: 'Full screen on / off' },
  { keys: ['P'], also: ['F3'], what: 'Pause / resume', note: 'Dims the picture and mutes the sound' },
  { keys: ['F8'], what: 'Sound on / off' },
  { keys: ['F5'], what: 'Save state', note: 'Into the current slot' },
  { keys: ['F7'], what: 'Load state' },
  { keys: ['F6'], what: 'Next save slot' },
  { keys: ['F2'], what: 'Reset game' },
  { keys: ['Tab'], what: 'Fast-forward', note: 'Hold. The speed is set in Settings' },
  { keys: ['F1'], what: 'Fast-forward on / off', note: 'Stays on until you press it again' },
  { keys: ['Shift', 'F1'], what: 'Slow motion on / off', note: 'Half speed, without sound' },
  { keys: ['Backspace'], what: 'Rewind', note: 'Hold. Switch it on in Settings' },
  { keys: ['F4'], what: 'CRT scanlines on / off' },
  { keys: ['Shift', 'F4'], what: 'LCD look on / off', note: 'A pixel grid, like a handheld' },
  { keys: ['F11'], what: 'Change disc' },
  { keys: ['F12'], what: 'Screenshot' },
  { keys: ['Esc'], what: 'Quit game' },
];

/** Hold Select on a gamepad and press another button. */
export const PAD_HOTKEYS = [
  { keys: ['Select', 'Start'], what: 'Quit game' },
  { keys: ['Select', 'R1'], what: 'Save state' },
  { keys: ['Select', 'L1'], what: 'Load state' },
  { keys: ['Select', 'D-pad →'], also: ['D-pad ←'], what: 'Next / previous save slot' },
  { keys: ['Select', 'R2'], what: 'Fast-forward', note: 'Hold' },
  { keys: ['Select', 'L2'], what: 'Rewind', note: 'Hold' },
  { keys: ['Select', '○'], what: 'Fast-forward on / off' },
  { keys: ['Select', '△'], what: 'Pause / resume' },
  { keys: ['Select', '□'], what: 'Screenshot' },
  { keys: ['Select', '✕'], what: 'Sound on / off' },
];

export const RESERVED = ['Escape', 'Tab', 'Backspace', 'KeyP', 'F1', 'F2', 'F3', 'F4', 'F5', 'F6', 'F7', 'F8', 'F9', 'F10', 'F11', 'F12'];

/** KeyboardEvent.code -> runner key name, or null if it cannot be bound. */
export function keyName(code) {
  if (CODE_TO_KEY[code]) return CODE_TO_KEY[code];
  let m;
  if ((m = /^Key([A-Z])$/.exec(code))) return m[1];
  if ((m = /^Digit(\d)$/.exec(code))) return 'Key' + m[1];
  if ((m = /^Numpad(\d)$/.exec(code))) return 'NumPad' + m[1];
  return null;
}

const KEY_LABEL = {
  Up: '↑', Down: '↓', Left: '←', Right: '→', Enter: 'Enter', Space: 'Space', Backspace: '⌫',
  LeftShift: 'L‑Shift', RightShift: 'R‑Shift', LeftCtrl: 'L‑Ctrl', RightCtrl: 'R‑Ctrl',
  LeftAlt: 'L‑Alt', RightAlt: 'R‑Alt', Comma: ',', Period: '.', Slash: '/', Semicolon: ';',
  Apostrophe: "'", LeftBracket: '[', RightBracket: ']', Minus: '-', Equal: '=', Backslash: '\\',
};
export function keyLabel(name) {
  if (KEY_LABEL[name]) return KEY_LABEL[name];
  const d = /^Key(\d)$/.exec(name);
  if (d) return d[1];
  const n = /^NumPad(\d)$/.exec(name);
  if (n) return 'Num ' + n[1];
  return name;
}

const PAD_LABEL = {
  South: 'South (A / ✕)', East: 'East (B / ○)', West: 'West (X / □)', North: 'North (Y / △)',
  LeftTrigger: 'L1 / LB', RightTrigger: 'R1 / RB', LeftTrigger2: 'L2 / LT', RightTrigger2: 'R2 / RT',
  Select: 'Select / Back', Start: 'Start', Mode: 'Guide', LeftThumb: 'L3', RightThumb: 'R3',
  DPadUp: 'D‑pad ↑', DPadDown: 'D‑pad ↓', DPadLeft: 'D‑pad ←', DPadRight: 'D‑pad →', C: 'C', Z: 'Z',
};
export const padLabel = (b) => PAD_LABEL[b] ?? b;

export function deviceValue(d) {
  return d.kind === 'gamepad' ? 'gamepad:' + d.id : d.kind;
}
export function parseDevice(v) {
  return v.startsWith('gamepad:') ? { kind: 'gamepad', id: v.slice(8) } : { kind: v };
}
