import { test } from 'node:test';
import assert from 'node:assert/strict';
import { filterSystems } from './layout.js';

const systems = [
  { id: 'neogeo', name: 'Neo Geo', game_count: 267 },
  { id: 'psx', name: 'PlayStation', game_count: 1 },
  { id: 'snes', name: 'Super Nintendo', game_count: 0 },
  { id: 'c64', name: 'Commodore 64', game_count: 0 },
];
const ids = (l) => l.map((s) => s.id);

test('no query and no switch lists every system', () => {
  assert.deepEqual(ids(filterSystems(systems)), ['neogeo', 'psx', 'snes', 'c64']);
});

test('the query matches names and ids, ignoring case and spaces around it', () => {
  assert.deepEqual(ids(filterSystems(systems, '  play ')), ['psx']);
  assert.deepEqual(ids(filterSystems(systems, 'C64')), ['c64']);
  assert.deepEqual(ids(filterSystems(systems, 'nintendo')), ['snes']);
  assert.deepEqual(ids(filterSystems(systems, 'zzz')), []);
});

test('only with games hides the empty systems', () => {
  assert.deepEqual(ids(filterSystems(systems, '', true)), ['neogeo', 'psx']);
});

test('the open system stays whatever was typed', () => {
  assert.deepEqual(ids(filterSystems(systems, 'play', true, 'snes')), ['psx', 'snes']);
});
