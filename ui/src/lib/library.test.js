import test from 'node:test';
import assert from 'node:assert/strict';
import { visibleGames, sortDirection, nextSorts, SORT_OPTIONS, sortWay, changeSortKey, turnSort, removeSort, moveSort, addSort, STATUSES, heroFor, missingCovers, lastPlayed, headingFor, pickRange, formatSize, summarizeImport, summarizeRemoval, summarizeBios } from './library.js';

const game = (title, o = {}) => ({ system: 'snes', title, path: `/lib/${title}`, size: 1e6, last_played: 0, play_secs: 0, favorite: false, imported: true, ...o });
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
  const one = (key, reverse = false) => ({ sorts: [{ key, reverse }] });
  assert.deepEqual(visibleGames(ov, one('recent')).map((g) => g.title), ['c', 'a', 'b']);
  assert.deepEqual(visibleGames(ov, one('az')).map((g) => g.title), ['a', 'b', 'c']);
  assert.deepEqual(visibleGames(ov, one('size')).map((g) => g.title), ['c', 'b', 'a']);
});

const titles = (list) => list.map((g) => g.title);

test('every sort can go either way, and the arrow says which', () => {
  const ov = overview([
    game('b', { size: 5, last_played: 10, play_secs: 30, system: 'psx' }),
    game('a', { size: 9, last_played: 30, play_secs: 10 }),
    game('c', { size: 1, last_played: 20, play_secs: 20 }),
  ]);
  const order = (key, reverse = false) => titles(visibleGames(ov, { sorts: [{ key, reverse }] }));
  // newest, biggest and most played first; names and systems from A to Z
  assert.deepEqual(order('recent'), ['a', 'c', 'b']);
  assert.deepEqual(order('recent', true), ['b', 'c', 'a']);
  assert.deepEqual(order('az'), ['a', 'b', 'c']);
  assert.deepEqual(order('az', true), ['c', 'b', 'a']);
  assert.deepEqual(order('size'), ['a', 'b', 'c']);
  assert.deepEqual(order('size', true), ['c', 'b', 'a']);
  assert.deepEqual(order('played'), ['b', 'c', 'a']);
  assert.deepEqual(order('played', true), ['a', 'c', 'b']);
  // by system name (PSX before SNES), a tie by title
  assert.deepEqual(order('system'), ['b', 'a', 'c']);
  assert.deepEqual(order('system', true), ['a', 'c', 'b']);

  assert.equal(sortDirection('recent'), 'desc');
  assert.equal(sortDirection('recent', true), 'asc');
  assert.equal(sortDirection('az'), 'asc');
  assert.equal(sortDirection('az', true), 'desc');
  assert.equal(sortDirection('nonsense'), 'desc', 'an unknown key is the default one');
});

test('games with nothing to compare stay at the end, whichever way', () => {
  const ov = overview([game('never'), game('old', { last_played: 5, play_secs: 5 }), game('new', { last_played: 9, play_secs: 9 }), game('also never')]);
  for (const key of ['recent', 'played']) {
    assert.deepEqual(titles(visibleGames(ov, { sorts: [{ key, reverse: false }] })), ['new', 'old', 'also never', 'never'], key);
    assert.deepEqual(titles(visibleGames(ov, { sorts: [{ key, reverse: true }] })), ['old', 'new', 'also never', 'never'], `${key} reversed`);
  }
});

test('filters by status, together with the rest', () => {
  const ov = overview([
    game('fresh'),
    game('played', { last_played: 5, play_secs: 600 }),
    game('cleared', { play_secs: 90 }), // its place in Recent was cleared, but it was played
    game('locked', { blocked: 'BIOS missing' }),
    game('twin1', { duplicate: true }),
    game('twin2', { duplicate: true, system: 'psx' }),
  ]);
  const by = (status, more = {}) => titles(visibleGames(ov, { status, sorts: [{ key: 'az', reverse: false }], ...more }));
  assert.deepEqual(by('never'), ['fresh', 'locked', 'twin1', 'twin2']);
  assert.deepEqual(by('played'), ['cleared', 'played']);
  assert.deepEqual(by('ready'), ['cleared', 'fresh', 'played', 'twin1', 'twin2']);
  assert.deepEqual(by('blocked'), ['locked']);
  assert.deepEqual(by('duplicates'), ['twin1', 'twin2']);
  assert.deepEqual(by('duplicates', { system: 'psx' }), ['twin2']);
  assert.deepEqual(by('never', { query: 'tw' }), ['twin1', 'twin2']);
  assert.deepEqual(by('played', { filter: 'recent' }), ['played'], 'with the sidebar filter');
  assert.deepEqual(by(''), ['cleared', 'fresh', 'locked', 'played', 'twin1', 'twin2'], 'no status: all');
  assert.deepEqual(by('nonsense'), by(''), 'an unknown status filters nothing');
  assert.deepEqual(STATUSES.map((s) => s.value), ['', 'never', 'played', 'ready', 'blocked', 'duplicates']);
});

test('the card to continue with ignores the status filter, as it does the search', () => {
  const ov = overview([game('old', { last_played: 5 }), game('fresh')]);
  assert.equal(heroFor(ov, { status: 'never' })?.title, 'old');
  assert.equal(heroFor(ov, { status: 'never', query: 'fresh' })?.title, 'old');
});

test('several sorts work together: the next one settles what the one before cannot', () => {
  const ov = overview([
    game('zelda', { system: 'psx', size: 5 }),
    game('mario', { system: 'snes', size: 5 }),
    game('crash', { system: 'psx', size: 9 }),
    game('alundra', { system: 'psx', size: 5 }),
    game('metroid', { system: 'snes', size: 1 }),
  ]);
  const order = (...sorts) => titles(visibleGames(ov, { sorts: sorts.map(([key, reverse = false]) => ({ key, reverse })) }));
  // by system, then by name inside each system
  assert.deepEqual(order(['system'], ['az']), ['alundra', 'crash', 'zelda', 'mario', 'metroid']);
  assert.deepEqual(order(['system'], ['az', true]), ['zelda', 'crash', 'alundra', 'metroid', 'mario']);
  // by size, biggest first, the ones of the same size by system, then by name
  assert.deepEqual(order(['size'], ['system'], ['az']), ['crash', 'alundra', 'zelda', 'mario', 'metroid']);
  // the order of the sorts matters
  assert.deepEqual(order(['az'], ['system']), ['alundra', 'crash', 'mario', 'metroid', 'zelda']);
  // never played games are at the end by the first sort, and still ordered by the next
  const never = overview([game('b'), game('a'), game('played', { last_played: 5 })]); // all of the same size
  assert.deepEqual(titles(visibleGames(never, { sorts: [{ key: 'recent', reverse: false }, { key: 'az', reverse: false }] })), ['played', 'a', 'b']);
  // what no sort can tell apart goes by title
  assert.deepEqual(titles(visibleGames(never, { sorts: [{ key: 'size', reverse: false }] })), ['a', 'b', 'played']);
});

test('clicking a heading, and Shift-clicking one, give the sorts', () => {
  const R = (key, reverse = false) => ({ key, reverse });
  const base = [R('recent')];
  // a click turns the main sort round, a click on another makes it the only one
  assert.deepEqual(nextSorts(base, 'recent'), [R('recent', true)]);
  assert.deepEqual(nextSorts([R('recent', true)], 'recent'), [R('recent')]);
  assert.deepEqual(nextSorts(base, 'az'), [R('az')]);
  // Shift adds the next sort; again it turns round; once more it goes
  const two = nextSorts(base, 'az', { add: true });
  assert.deepEqual(two, [R('recent'), R('az')]);
  const turned = nextSorts(two, 'az', { add: true });
  assert.deepEqual(turned, [R('recent'), R('az', true)]);
  assert.deepEqual(nextSorts(turned, 'az', { add: true }), [R('recent')]);
  // the main sort, with Shift, only turns round and back: it cannot go
  assert.deepEqual(nextSorts(base, 'recent', { add: true }), [R('recent', true)]);
  assert.deepEqual(nextSorts([R('recent', true)], 'recent', { add: true }), [R('recent')]);
  // a plain click on the main sort keeps the others; on another one it drops them
  assert.deepEqual(nextSorts(two, 'recent'), [R('recent', true), R('az')]);
  assert.deepEqual(nextSorts(two, 'az'), [R('az')]);
  // three at most make sense, but nothing stops a fourth, and a key is never there twice
  const three = nextSorts(two, 'size', { add: true });
  assert.deepEqual(three.map((s) => s.key), ['recent', 'az', 'size']);
  assert.deepEqual(nextSorts(three, 'size', { add: true }).map((s) => s.key), ['recent', 'az', 'size']);
  // nothing known, nothing changes; an empty list starts from the default
  assert.deepEqual(nextSorts(base, 'nonsense'), base);
  assert.deepEqual(nextSorts([], 'az', { add: true }), [R('recent'), R('az')]);
  assert.deepEqual(nextSorts(undefined, 'recent'), [R('recent', true)]);
});

test('the sort menu changes the sorts one at a time', () => {
  const R = (key, reverse = false) => ({ key, reverse });
  const two = [R('system'), R('az', true)];
  // a different key for a sort starts it going its first way
  assert.deepEqual(changeSortKey(two, 1, 'size'), [R('system'), R('size')]);
  // a key that another sort has: that one goes, so a key is never there twice
  assert.deepEqual(changeSortKey(two, 0, 'az'), [R('az')]);
  assert.deepEqual(changeSortKey(two, 1, 'system'), [R('system')]);
  // turning round, removing, moving
  assert.deepEqual(turnSort(two, 0), [R('system', true), R('az', true)]);
  assert.deepEqual(turnSort(turnSort(two, 1), 1), two, 'twice is back to what it was');
  assert.deepEqual(removeSort(two, 0), [R('az', true)]);
  assert.deepEqual(removeSort([R('az')], 0), [R('az')], 'the last sort stays');
  assert.deepEqual(moveSort(two, 1, -1), [R('az', true), R('system')]);
  assert.deepEqual(moveSort(two, 0, -1), two, 'it stops at the top');
  assert.deepEqual(moveSort(two, 1, 1), two, 'and at the bottom');
  assert.deepEqual(moveSort(two, 5, -1), two, 'a place that is not there changes nothing');
  // adding: the first key not used yet, until all are
  assert.deepEqual(addSort([R('recent')]), [R('recent'), R('az')]);
  assert.deepEqual(addSort([R('az'), R('recent')]), [R('az'), R('recent'), R('system')]);
  const all = SORT_OPTIONS.map((o) => R(o.value));
  assert.deepEqual(addSort(all), all, 'nothing left to add');
  // nothing is changed in place
  const before = JSON.stringify(two);
  [changeSortKey, turnSort, removeSort, moveSort].forEach((f) => f(two, 0, 1));
  addSort(two);
  assert.equal(JSON.stringify(two), before);
  // an empty list is the default one
  assert.deepEqual(addSort([]), [R('recent'), R('az')]);
  assert.deepEqual(turnSort(undefined, 0), [R('recent', true)]);
});

test('every way of sorting has words for both of its ways', () => {
  for (const { value } of SORT_OPTIONS) {
    const [first, other] = [sortWay(value), sortWay(value, true)];
    assert.ok(first && other && first !== other, value);
  }
  assert.equal(sortWay('recent'), 'Newest first');
  assert.equal(sortWay('az', true), 'Z to A');
  assert.equal(sortWay('nonsense'), 'Newest first', 'an unknown key reads as the default');
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
