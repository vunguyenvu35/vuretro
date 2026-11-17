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
export const RESERVED = ['Escape', 'Tab', 'F2', 'F5', 'F6', 'F7', 'F8'];

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
