import test from 'node:test';
import assert from 'node:assert/strict';
import { gameKey, profilesOf } from './macros.js';

test('a game is found by its file name without extension, lower case, on any OS', () => {
  assert.equal(gameKey('/roms/neogeo/FSwords.zip'), 'fswords');
  assert.equal(gameKey('C:\\Games\\Neo Geo\\Garou.zip'), 'garou');
  assert.equal(gameKey('plain'), 'plain');
  assert.equal(gameKey('/x/.hidden'), '.hidden');
});

test('profiles of a game and the one in use, with defaults for an old config', () => {
  const cfg = { profiles: { fswords: [{ id: 'h', name: 'Haohmaru', macros: [] }] }, active: { fswords: 'h' } };
  assert.deepEqual(profilesOf(cfg, 'fswords'), { profiles: cfg.profiles.fswords, active: 'h' });
  assert.deepEqual(profilesOf(cfg, 'other'), { profiles: [], active: '' });
  assert.deepEqual(profilesOf({ global: [], games: {} }, 'x'), { profiles: [], active: '' });
  assert.deepEqual(profilesOf(null, 'x'), { profiles: [], active: '' });
});
