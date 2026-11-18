import test from 'node:test';
import assert from 'node:assert/strict';
import { visibleGames, lastPlayed, headingFor, pickRange, formatSize, summarizeImport, summarizeRemoval, summarizeBios } from './library.js';

const game = (title, o = {}) => ({ system: 'snes', title, path: `/lib/${title}`, size: 1e6, last_played: 0, favorite: false, imported: true, ...o });
const overview = (games, disabled = []) => ({
  systems: ['snes', 'psx'].map((id) => ({ id, name: id.toUpperCase(), enabled: !disabled.includes(id) })),
  games,
});

test('hides games of a switched-off system', () => {
  const ov = overview([game('a'), game('b', { system: 'psx' })], ['psx']);
  assert.deepEqual(visibleGames(ov).map((g) => g.title), ['a']);
});

test('filters by system, recent, favorites and search text', () => {
  const ov = overview([
    game('Zelda', { last_played: 5, favorite: true }),
    game('Mario'),
    game('Crash', { system: 'psx', last_played: 9 }),
  ]);
  assert.deepEqual(visibleGames(ov, { system: 'psx' }).map((g) => g.title), ['Crash']);
  assert.deepEqual(visibleGames(ov, { filter: 'recent' }).map((g) => g.title), ['Crash', 'Zelda']);
  assert.deepEqual(visibleGames(ov, { filter: 'favorites' }).map((g) => g.title), ['Zelda']);
  assert.deepEqual(visibleGames(ov, { query: '  MAR ' }).map((g) => g.title), ['Mario']);
});

test('sorts by recent (ties by title), az and size', () => {
  const ov = overview([game('b', { size: 5e6 }), game('a', { size: 1e6 }), game('c', { size: 9e6, last_played: 3 })]);
  assert.deepEqual(visibleGames(ov, { sort: 'recent' }).map((g) => g.title), ['c', 'a', 'b']);
  assert.deepEqual(visibleGames(ov, { sort: 'az' }).map((g) => g.title), ['a', 'b', 'c']);
  assert.deepEqual(visibleGames(ov, { sort: 'size' }).map((g) => g.title), ['c', 'b', 'a']);
});

test('no overview means no games', () => {
  assert.deepEqual(visibleGames(null), []);
});

test('lastPlayed ignores never-played games', () => {
  assert.equal(lastPlayed([game('a'), game('b')]), undefined);
  assert.equal(lastPlayed([game('a', { last_played: 1 }), game('b', { last_played: 2 })]).title, 'b');
});

test('heading names the system or the filter', () => {
  const ov = overview([]);
  assert.equal(headingFor(ov, { system: 'psx', filter: 'all' }), 'PSX');
  assert.equal(headingFor(ov, { system: null, filter: 'favorites' }), 'Favorites');
});

test('pickRange selects imported games between two paths, either direction', () => {
  const list = [game('a'), game('b', { imported: false }), game('c'), game('d')];
  assert.deepEqual(pickRange(list, '/lib/a', '/lib/c'), ['/lib/a', '/lib/c']);
  assert.deepEqual(pickRange(list, '/lib/d', '/lib/c'), ['/lib/c', '/lib/d']);
  assert.deepEqual(pickRange(list, '/lib/a', '/nope'), []);
});

test('formatSize', () => {
  assert.equal(formatSize(100), '1 MB');
  assert.equal(formatSize(31_000_000), '31 MB');
  assert.equal(formatSize(1_500_000_000), '1.5 GB');
});

test('import summary counts added, existing and failures', () => {
  const r = summarizeImport([
    { source: '/x/a.sfc', ok: true, message: 'added to snes' },
    { source: '/x/b.sfc', ok: true, message: 'already in the library' },
    { source: 'C:\\x\\c.zip', ok: false, message: 'cannot tell which system' },
  ]);
  assert.equal(r.text, '✓ 1 game added\n1 already in the library\n✕ c.zip: cannot tell which system');
  assert.equal(r.failed, 1);
  assert.equal(summarizeImport([]).text, 'Nothing to import');
});

test('removal and bios summaries', () => {
  assert.equal(summarizeRemoval([{ source: '/a', ok: true, message: 'removed' }]).text, 'Removed 1 game from the library');
  const b = summarizeBios([{ source: '/a/n.zip', ok: true, message: 'Installed n.zip' }, { source: '/a/x.bin', ok: false, message: 'nope' }]);
  assert.equal(b.text, '✓ Installed n.zip\n✕ x.bin: nope');
  assert.equal(b.failed, 1);
});

test('filters by collection and names it in the heading', () => {
  const ov = overview([game('Zelda', { tags: ['RPG', 'Co-op'] }), game('Mario', { tags: ['Platformer'] }), game('Old')]);
  assert.deepEqual(visibleGames(ov, { collection: 'RPG' }).map((g) => g.title), ['Zelda']);
  assert.deepEqual(visibleGames(ov, { collection: 'Nope' }), []);
  assert.equal(headingFor(ov, { system: null, filter: 'all', collection: 'RPG' }), 'Collection: RPG');
  assert.equal(headingFor(ov, { system: null, filter: 'all', collection: null }), 'All games');
});
