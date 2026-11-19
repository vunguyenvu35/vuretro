import test from 'node:test';
import assert from 'node:assert/strict';
import { visibleGames, heroFor, missingCovers, lastPlayed, headingFor, pickRange, formatSize, summarizeImport, summarizeRemoval, summarizeBios } from './library.js';

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

import { clampSidebar, parseSidebar, SIDEBAR_DEFAULT, SIDEBAR_MIN, SIDEBAR_MAX } from './layout.js';

test('the sidebar width stays within its limits and half of a small window', () => {
  assert.equal(clampSidebar(300), 300);
  assert.equal(clampSidebar(10), SIDEBAR_MIN);
  assert.equal(clampSidebar(9999), SIDEBAR_MAX);
  assert.equal(clampSidebar(400, 600), 300, 'half of a 600 px window');
  assert.equal(clampSidebar(400, 200), SIDEBAR_MIN, 'the minimum wins in a tiny window');
  assert.equal(clampSidebar(NaN), SIDEBAR_DEFAULT);
});

test('a saved sidebar width is read safely', () => {
  assert.equal(parseSidebar('310'), 310);
  assert.equal(parseSidebar('5'), SIDEBAR_MIN);
  for (const bad of [null, undefined, '', 'wide', 'NaN']) assert.equal(parseSidebar(bad), SIDEBAR_DEFAULT, String(bad));
});

import { parseScope, gameScope } from './scope.js';

test('a scope string tells all games, a system and a game apart', () => {
  assert.deepEqual(parseScope(''), { kind: 'all', id: '' });
  assert.deepEqual(parseScope(undefined), { kind: 'all', id: '' });
  assert.deepEqual(parseScope('snes'), { kind: 'system', id: 'snes' });
  assert.deepEqual(parseScope(gameScope('/roms/snes/Zelda: Link.sfc')), { kind: 'game', id: '/roms/snes/Zelda: Link.sfc' });
});

test('an import reports installed BIOS files by name and does not count them as duplicates', () => {
  const { text, failed } = summarizeImport([
    { source: '/x/a.zip', ok: true, message: 'added to neogeo' },
    { source: '/x/neogeo.zip', ok: true, message: 'BIOS installed: neogeo.zip' },
    { source: '/x/b.zip', ok: true, message: 'already in the library' },
  ]);
  assert.equal(failed, 0);
  assert.equal(text, '✓ 1 game added\n✓ BIOS installed: neogeo.zip\n1 already in the library');
});

import { backupFileName, formatBytes } from './layout.js';

test('backup file names carry the date, and sizes read naturally', () => {
  assert.equal(backupFileName(new Date(2026, 10, 8)), 'vuretro-backup-2026-11-08.zip');
  assert.equal(formatBytes(340_000), '340 KB');
  assert.equal(formatBytes(1_234_000), '1.2 MB');
  assert.equal(formatBytes(2_500_000_000), '2.5 GB');
  assert.equal(formatBytes(10), '1 KB');
});

test('the hero is the latest game of the chosen system, not of the whole library', () => {
  const ov = overview([
    game('Zelda', { last_played: 9 }),
    game('Mario', { last_played: 3 }),
    game('Crash', { system: 'psx', last_played: 7 }),
    game('Spyro', { system: 'psx' }),
  ]);
  assert.equal(heroFor(ov, { filter: 'all' }).title, 'Zelda');
  assert.equal(heroFor(ov, { system: 'psx', filter: 'all' }).title, 'Crash');
  assert.equal(heroFor(ov, { system: 'snes', filter: 'all' }).title, 'Zelda');
  // a system where nothing was played offers nothing
  const none = overview([game('Zelda', { last_played: 9 }), game('Spyro', { system: 'psx' })]);
  assert.equal(heroFor(none, { system: 'psx', filter: 'all' }), undefined);
  // favorites view: only favorites count; the search box does not narrow it
  const fav = overview([game('A', { last_played: 5 }), game('B', { last_played: 2, favorite: true })]);
  assert.equal(heroFor(fav, { filter: 'favorites', query: 'zzz' }).title, 'B');
});

test('only games without a picture that can run need one made', () => {
  const games = [
    { path: '/a', blocked: null },
    { path: '/b', blocked: null },
    { path: '/c', blocked: 'BIOS missing' },
    { path: '/d', blocked: null },
  ];
  const have = { '/b': 'data:image/png;base64,xx' };
  assert.deepEqual(missingCovers(games, have).map((g) => g.path), ['/a', '/d']);
  assert.deepEqual(missingCovers(games, {}).map((g) => g.path), ['/a', '/b', '/d']);
  assert.deepEqual(missingCovers([], have), []);
});
