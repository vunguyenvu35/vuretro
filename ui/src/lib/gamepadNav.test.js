import test from 'node:test';
import assert from 'node:assert/strict';
import { pickNext, readPad } from './gamepadNav.js';

// a 3x2 grid of 10x10 tiles, 20 px apart:   0 1 2
//                                           3 4 5
const grid = [0, 1, 2, 3, 4, 5].map((i) => ({ x: (i % 3) * 20, y: Math.floor(i / 3) * 20, w: 10, h: 10 }));

test('moves to the neighbour in the pressed direction', () => {
  assert.equal(pickNext(grid, 0, 'right'), 1);
  assert.equal(pickNext(grid, 1, 'right'), 2);
  assert.equal(pickNext(grid, 4, 'left'), 3);
  assert.equal(pickNext(grid, 1, 'down'), 4);
  assert.equal(pickNext(grid, 5, 'up'), 2);
});

test('stays put at an edge and starts at the first item when nothing is focused', () => {
  assert.equal(pickNext(grid, 2, 'right'), -1);
  assert.equal(pickNext(grid, 0, 'up'), -1);
  assert.equal(pickNext(grid, -1, 'down'), 0);
  assert.equal(pickNext([], -1, 'down'), -1);
});

test('keeps to its row or column over a nearer diagonal item', () => {
  const items = [
    { x: 0, y: 0, w: 10, h: 10 }, // current
    { x: 60, y: 0, w: 10, h: 10 }, // same row, far
    { x: 15, y: 40, w: 10, h: 10 }, // below-right, closer in x
  ];
  assert.equal(pickNext(items, 0, 'right'), 1);
  assert.equal(pickNext(items, 0, 'down'), 2);
});

test('reads D-pad, stick and face buttons from a standard pad', () => {
  const pad = (pressed = [], axes = [0, 0]) => ({ buttons: Array.from({ length: 16 }, (_, i) => ({ pressed: pressed.includes(i) })), axes });
  assert.deepEqual(readPad(pad([12])).up, true);
  assert.equal(readPad(pad([], [0.9, 0])).right, true);
  assert.equal(readPad(pad([], [0.3, 0])).right, false, 'inside the dead zone');
  assert.equal(readPad(pad([0])).a, true);
  assert.equal(readPad(pad([1])).b, true);
});
