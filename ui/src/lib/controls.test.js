import test from 'node:test';
import assert from 'node:assert/strict';
import { ACTIONS, NEOGEO_NAMES, actionLabel } from './controls.js';

const by = (id) => ACTIONS.find((a) => a.id === id);

test('on a Neo Geo the face buttons are called A B C D, in the order of the default keys Z X A S', () => {
  assert.deepEqual(['b', 'a', 'y', 'x'].map((id) => actionLabel(by(id), true)), ['A', 'B', 'C', 'D']);
});

test('the standard names stay when Neo Geo naming is off, and other buttons keep theirs', () => {
  assert.deepEqual(['b', 'a', 'y', 'x'].map((id) => actionLabel(by(id), false)), ['B', 'A', 'Y', 'X']);
  assert.equal(actionLabel(by('up'), true), 'Up');
  assert.equal(actionLabel(by('start'), true), 'Start');
  assert.equal(actionLabel(by('select'), true), 'Coin');
});

test('every name in the Neo Geo table is a real button', () => {
  for (const id of Object.keys(NEOGEO_NAMES)) assert.ok(by(id), id);
});

test('the hints on the Controls screen agree with the Neo Geo names', () => {
  for (const id of ['b', 'a', 'y', 'x']) assert.match(by(id).hint, new RegExp(`Neo Geo ${NEOGEO_NAMES[id]}\\b`));
});

test('a diagonal key presses its two directions and other actions press themselves', async () => {
  const { DIAGONALS, pressedBy } = await import('./controls.js');
  assert.equal(DIAGONALS.length, 4);
  assert.deepEqual(pressedBy('down_right'), ['down', 'right']);
  assert.deepEqual(pressedBy('up_left'), ['up', 'left']);
  assert.deepEqual(pressedBy('b'), ['b']);
  assert.deepEqual(pressedBy('nonsense'), []);
});

test('on a PlayStation the face buttons are called ✕ ○ □ △ and the shoulders L1 R1 L2 R2', async () => {
  const { PLAYSTATION_NAMES, nameOf } = await import('./controls.js');
  assert.deepEqual(['b', 'a', 'y', 'x'].map((id) => nameOf(by(id), PLAYSTATION_NAMES)), ['✕', '○', '□', '△']);
  assert.deepEqual(['l', 'r', 'l2', 'r2'].map((id) => nameOf(by(id), PLAYSTATION_NAMES)), ['L1', 'R1', 'L2', 'R2']);
  // buttons with no special name, and a system with no table, keep the standard names
  assert.equal(nameOf(by('start'), PLAYSTATION_NAMES), 'Start');
  assert.equal(nameOf(by('b'), null), 'B');
});

test('swapping ✕ and ○ swaps the keys and the pad buttons of that player, and back again', async () => {
  const { swapButtons } = await import('./controls.js');
  const player = { keyboard: { b: ['Z'], a: ['X'], y: ['A'] }, gamepad: { b: ['South'], a: [] } };
  swapButtons(player, 'b', 'a');
  assert.deepEqual(player.keyboard, { b: ['X'], a: ['Z'], y: ['A'] });
  assert.deepEqual(player.gamepad, { b: [], a: ['South'] });
  swapButtons(player, 'b', 'a');
  assert.deepEqual(player.keyboard.b, ['Z']);
});

test('arcade keys give a PlayStation player Z = □ (attack), X = ✕ (jump), A = ○, S = △', async () => {
  const { useArcadeKeys } = await import('./controls.js');
  // the default keys, plus a key that has to be taken away from the button that had it
  const player = { keyboard: { b: ['Z'], a: ['X'], y: ['A'], x: ['S'], l: ['Q', 'A'], start: ['Enter'] } };
  useArcadeKeys(player);
  assert.deepEqual([player.keyboard.y, player.keyboard.b, player.keyboard.a, player.keyboard.x], [['Z'], ['X'], ['A'], ['S']]);
  assert.deepEqual(player.keyboard.l, ['Q'], 'A is no longer also L1');
  assert.deepEqual(player.keyboard.start, ['Enter']);
});
