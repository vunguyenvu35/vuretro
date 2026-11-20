// Rules for the library screen that need no DOM and no Tauri: which games to show, in what
// order, and how results of bulk operations read. Kept pure so `npm test` can cover them.

/**
 * The ways to order the games. `desc` is the way a key goes when first chosen: the newest, the
 * biggest and the most played first, names and systems from A to Z. `emptyLast` keeps games with
 * nothing to compare (never played) at the end, whichever way the order goes.
 */
const SORT_KEYS = {
  recent: { desc: true, emptyLast: true, value: (g) => g.last_played },
  az: { desc: false, value: (g) => g.title },
  system: { desc: false, value: (g, system) => system(g) },
  size: { desc: true, value: (g) => g.size },
  played: { desc: true, emptyLast: true, value: (g) => g.play_secs },
};

/** `asc` or `desc`, for the arrow that shows which way the games are ordered. */
export function sortDirection(key, reverse = false) {
  const k = SORT_KEYS[key] ?? SORT_KEYS.recent;
  return k.desc !== !!reverse ? 'desc' : 'asc';
}

/** How the games are ordered: one or more keys, the first the main one, each going its way or the other. */
export const DEFAULT_SORTS = [{ key: 'recent', reverse: false }];

/**
 * The order after a click on the heading of `key`. A plain click on the main sort turns it round
 * and keeps the others; on anything else it becomes the only sort. With `add` (Shift) the key is
 * added as the next sort; on one that is there it goes round, then, when it is turned round
 * already, it goes (the main sort only turns back, it cannot go).
 */
export function nextSorts(sorts, key, { add = false } = {}) {
  if (!SORT_KEYS[key]) return sorts;
  const now = sorts?.length ? sorts : DEFAULT_SORTS;
  const i = now.findIndex((s) => s.key === key);
  const turned = (j, reverse) => now.map((s, k) => (k === j ? { ...s, reverse } : s));
  if (!add) return i === 0 ? turned(0, !now[0].reverse) : [{ key, reverse: false }];
  if (i < 0) return [...now, { key, reverse: false }];
  if (!now[i].reverse) return turned(i, true);
  return i === 0 ? turned(0, false) : now.filter((_, k) => k !== i);
}

/** The sorts to choose from, in the order they are offered. */
export const SORT_OPTIONS = [
  { value: 'recent', label: 'Recent' },
  { value: 'az', label: 'A–Z' },
  { value: 'system', label: 'System' },
  { value: 'size', label: 'Size' },
  { value: 'played', label: 'Time played' },
];

/** What each way of a sort is called in words: the way it goes when first chosen, then the other. */
const SORT_WAYS = {
  recent: ['Newest first', 'Oldest first'],
  az: ['A to Z', 'Z to A'],
  system: ['A to Z', 'Z to A'],
  size: ['Largest first', 'Smallest first'],
  played: ['Most played first', 'Least played first'],
};
export const sortWay = (key, reverse = false) => (SORT_WAYS[key] ?? SORT_WAYS.recent)[reverse ? 1 : 0];

// The changes the sort menu makes to the list of sorts. Each gives a new list and leaves the old one.
const orDefault = (sorts) => (sorts?.length ? sorts : DEFAULT_SORTS);

/** Sort number `i` is by `key` now (going its first way). If another sort was by `key`, that one goes. */
export function changeSortKey(sorts, i, key) {
  const now = orDefault(sorts);
  if (!SORT_KEYS[key] || i < 0 || i >= now.length) return now;
  return now.map((s, j) => (j === i ? { key, reverse: false } : s)).filter((s, j) => j === i || s.key !== key);
}

/** Sort number `i` goes the other way. */
export function turnSort(sorts, i) {
  const now = orDefault(sorts);
  return now.map((s, j) => (j === i ? { ...s, reverse: !s.reverse } : s));
}

/** Sort number `i` goes. The last one cannot: the games have to be in some order. */
export function removeSort(sorts, i) {
  const now = orDefault(sorts);
  return now.length > 1 ? now.filter((_, j) => j !== i) : now;
}

/** Sort number `i` moves `by` places up (negative) or down; it stops at the ends. */
export function moveSort(sorts, i, by) {
  const now = orDefault(sorts);
  const to = i + by;
  if (i < 0 || i >= now.length || to < 0 || to >= now.length || !by) return now;
  const out = [...now];
  [out[i], out[to]] = [out[to], out[i]];
  return out;
}

/** A sort by the first key not used yet, after the others; the list as it is when all are used. */
export function addSort(sorts) {
  const now = orDefault(sorts);
  const next = SORT_OPTIONS.find((o) => !now.some((s) => s.key === o.value));
  return next ? [...now, { key: next.value, reverse: false }] : now;
}

/** One key's way of telling two games apart: below 0, above 0, or 0 when it cannot. */
function comparator(key, reverse, system) {
  const k = SORT_KEYS[key] ?? SORT_KEYS.recent;
  const sign = (k.desc ? -1 : 1) * (reverse ? -1 : 1);
  return (a, b) => {
    const va = k.value(a, system);
    const vb = k.value(b, system);
    if (k.emptyLast) {
      const [ea, eb] = [!va, !vb];
      if (ea !== eb) return ea ? 1 : -1;
      if (ea) return 0;
    }
    const c = typeof va === 'string' ? va.localeCompare(vb) : va - vb;
    return c * sign;
  };
}

/** What a game can be filtered by besides the sidebar: `value` is kept in the store. */
export const STATUSES = [
  { value: '', label: 'Any status' },
  { value: 'never', label: 'Never played' },
  { value: 'played', label: 'Played' },
  { value: 'ready', label: 'Ready to play' },
  { value: 'blocked', label: 'Cannot play yet' },
  { value: 'duplicates', label: 'Duplicates' },
];

const STATUS_TESTS = {
  never: (g) => !g.last_played && !g.play_secs,
  played: (g) => !!(g.last_played || g.play_secs),
  ready: (g) => !g.blocked,
  blocked: (g) => !!g.blocked,
  duplicates: (g) => !!g.duplicate,
};

/**
 * Games to show for a view. `view` is { system, filter, collection, status, query, sorts } as kept
 * in the store; games of a system the user switched off are hidden. Games the sorts cannot tell
 * apart go by title.
 */
export function visibleGames(
  overview,
  { system = null, filter = 'all', collection = null, status = '', query = '', sorts = DEFAULT_SORTS } = {},
) {
  if (!overview) return [];
  let list = overview.games.filter((g) => overview.systems.find((s) => s.id === g.system)?.enabled !== false);
  if (system) list = list.filter((g) => g.system === system);
  if (collection) list = list.filter((g) => g.tags?.includes(collection));
  if (filter === 'recent') list = list.filter((g) => g.last_played);
  if (filter === 'favorites') list = list.filter((g) => g.favorite);
  if (STATUS_TESTS[status]) list = list.filter(STATUS_TESTS[status]);
  const q = query.trim().toLowerCase();
  if (q) list = list.filter((g) => g.title.toLowerCase().includes(q));
  const names = new Map(overview.systems.map((s) => [s.id, s.name]));
  const system_name = (g) => names.get(g.system) ?? g.system;
  const by = (sorts?.length ? sorts : DEFAULT_SORTS).map((s) => comparator(s.key, s.reverse, system_name));
  return [...list].sort((a, b) => {
    for (const compare of by) {
      const c = compare(a, b);
      if (c) return c;
    }
    return a.title.localeCompare(b.title);
  });
}

/** The game played most recently, if any. */
export function lastPlayed(games) {
  return [...games].filter((g) => g.last_played).sort((a, b) => b.last_played - a.last_played)[0];
}

/**
 * The game "Continue playing" offers: the one played most recently *within the view* (the
 * chosen system, collection or filter), so switching to another system does not keep
 * offering a game of the previous one. The search text and the status filter do not narrow it.
 */
export function heroFor(overview, view) {
  // neither the search nor the status filter narrows it: the card is the game to go on with
  return lastPlayed(visibleGames(overview, { ...view, query: '', status: '' }));
}

/** The games that have no picture (and can run, so one can be made for them). */
export function missingCovers(games, have) {
  return games.filter((g) => !have[g.path] && !g.blocked);
}

export function headingFor(overview, { system, filter, collection }) {
  if (collection) return `Collection: ${collection}`;
  if (system) return overview?.systems.find((s) => s.id === system)?.name ?? system;
  return { all: 'All games', recent: 'Recently played', favorites: 'Favorites' }[filter];
}

/** Paths of the imported games between two paths of `visible` (shift-click range). */
export function pickRange(visible, fromPath, toPath) {
  const a = visible.findIndex((x) => x.path === fromPath);
  const b = visible.findIndex((x) => x.path === toPath);
  if (a < 0 || b < 0) return [];
  return visible
    .slice(Math.min(a, b), Math.max(a, b) + 1)
    .filter((x) => x.imported)
    .map((x) => x.path);
}

export function formatSize(n) {
  return n >= 1e9 ? `${(n / 1e9).toFixed(1)} GB` : `${Math.max(1, Math.round(n / 1e6))} MB`;
}

const fileName = (r) => r.source.split(/[\\/]/).pop();
const plural = (n, word) => `${n} ${word}${n === 1 ? '' : 's'}`;

/** Toast text for the outcomes of a ROM import. */
export function summarizeImport(results) {
  const ok = results.filter((r) => r.ok);
  const bad = results.filter((r) => !r.ok);
  const added = ok.filter((r) => r.message.startsWith('added')).length;
  const bios = ok.filter((r) => r.message.startsWith('BIOS installed'));
  const lines = [];
  if (added) lines.push(`✓ ${plural(added, 'game')} added`);
  lines.push(...bios.map((r) => `✓ ${r.message}`));
  const same = ok.length - added - bios.length;
  if (same) lines.push(`${same} already in the library`);
  lines.push(...bad.slice(0, 4).map((r) => `✕ ${fileName(r)}: ${r.message}`));
  if (bad.length > 4) lines.push(`… and ${bad.length - 4} more`);
  return { text: lines.join('\n') || 'Nothing to import', failed: bad.length };
}

/** Toast text for the outcomes of removing games. */
export function summarizeRemoval(results) {
  const ok = results.filter((r) => r.ok).length;
  const bad = results.filter((r) => !r.ok);
  const lines = [`Removed ${plural(ok, 'game')} from the library`];
  lines.push(...bad.slice(0, 3).map((r) => `✕ ${fileName(r)}: ${r.message}`));
  return { text: lines.join('\n'), failed: bad.length };
}

/** Toast text for the outcomes of a BIOS import. */
export function summarizeBios(results) {
  const bad = results.filter((r) => !r.ok);
  const lines = [...results.filter((r) => r.ok).map((r) => `✓ ${r.message}`), ...bad.map((r) => `✕ ${fileName(r)}: ${r.message}`)];
  return { text: lines.join('\n'), failed: bad.length };
}
