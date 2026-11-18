// Drive the whole UI with a gamepad: D-pad or left stick moves the focus to the nearest
// control in that direction, A presses it, B goes back. The geometry is a pure function
// (`pickNext`) so it is covered by `npm test`; the rest is thin glue around the browser's
// Gamepad API.

/**
 * Index of the item to focus when moving `dir` ('up' | 'down' | 'left' | 'right') from
 * `current`, or -1 when nothing lies that way. `items` are rectangles { x, y, w, h }.
 * Candidates must lie in the half-plane of the direction; among them the nearest wins,
 * with sideways drift counted double so rows and columns stay in line.
 */
export function pickNext(items, current, dir) {
  const c = items[current];
  if (!c) return items.length ? 0 : -1;
  const cx = c.x + c.w / 2;
  const cy = c.y + c.h / 2;
  let best = -1;
  let bestScore = Infinity;
  items.forEach((it, i) => {
    if (i === current) return;
    const dx = it.x + it.w / 2 - cx;
    const dy = it.y + it.h / 2 - cy;
    const [along, across] = dir === 'up' ? [-dy, dx] : dir === 'down' ? [dy, dx] : dir === 'left' ? [-dx, dy] : [dx, dy];
    if (along <= 1) return; // not in that direction
    const score = along + Math.abs(across) * 2;
    if (score < bestScore) {
      bestScore = score;
      best = i;
    }
  });
  return best;
}

/** Standard-mapping buttons and the stick dead zone. */
const BTN = { a: 0, b: 1, up: 12, down: 13, left: 14, right: 15 };
const STICK = 0.55;
const REPEAT_FIRST = 380; // ms before a held direction repeats
const REPEAT_EVERY = 110;

/** Which commands a pad state asks for right now. */
export function readPad(pad) {
  const b = (i) => !!pad.buttons[i]?.pressed;
  const [ax = 0, ay = 0] = pad.axes;
  return {
    up: b(BTN.up) || ay < -STICK,
    down: b(BTN.down) || ay > STICK,
    left: b(BTN.left) || ax < -STICK,
    right: b(BTN.right) || ax > STICK,
    a: b(BTN.a),
    b: b(BTN.b),
  };
}

const FOCUSABLE = 'button:not([disabled]), [role="button"]:not([aria-disabled="true"]), a[href], input:not([disabled]), [role="switch"]';

function focusables() {
  return [...document.querySelectorAll(FOCUSABLE)].filter((el) => {
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0 && getComputedStyle(el).visibility !== 'hidden';
  });
}

/** Start listening. Returns a function that stops it. */
export function startGamepadNav(paused = () => false) {
  if (typeof window === 'undefined' || !navigator.getGamepads) return () => {};
  let raf = 0;
  const held = {}; // command -> time it was first seen / last repeated
  const was = {};

  const move = (dir) => {
    document.documentElement.dataset.input = 'pad';
    const els = focusables();
    const items = els.map((el) => {
      const r = el.getBoundingClientRect();
      return { x: r.x, y: r.y, w: r.width, h: r.height };
    });
    const cur = els.indexOf(document.activeElement);
    const next = pickNext(items, cur, dir);
    if (next >= 0) {
      els[next].focus({ preventScroll: false });
      els[next].scrollIntoView?.({ block: 'nearest', inline: 'nearest' });
    }
  };

  const press = (cmd, now) => {
    if (cmd === 'a') document.activeElement?.click?.();
    else if (cmd === 'b') document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    else move(cmd);
    held[cmd] = now;
  };

  const mouse = () => delete document.documentElement.dataset.input;
  window.addEventListener('pointerdown', mouse);
  const tick = (now) => {
    const pad = [...navigator.getGamepads()].find(Boolean);
    if (pad && !paused()) {
      const cmds = readPad(pad);
      for (const cmd of Object.keys(cmds)) {
        if (!cmds[cmd]) {
          was[cmd] = false;
        } else if (!was[cmd]) {
          was[cmd] = true;
          press(cmd, now + REPEAT_FIRST - REPEAT_EVERY);
        } else if (cmd !== 'a' && cmd !== 'b' && now - held[cmd] >= REPEAT_EVERY) {
          press(cmd, now);
        }
      }
    }
    raf = requestAnimationFrame(tick);
  };
  raf = requestAnimationFrame(tick);
  return () => {
    cancelAnimationFrame(raf);
    window.removeEventListener('pointerdown', mouse);
  };
}
