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
